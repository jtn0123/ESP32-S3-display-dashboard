// Voltage monitoring for power supply diagnostics
// Especially useful during high-power operations like WiFi initialization

use anyhow::Result;
use std::sync::atomic::{AtomicU16, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

// Global voltage tracking
static MIN_VOLTAGE: AtomicU16 = AtomicU16::new(u16::MAX);
static MAX_VOLTAGE: AtomicU16 = AtomicU16::new(0);
static LAST_VOLTAGE: AtomicU16 = AtomicU16::new(0);
static VOLTAGE_DROP_COUNT: AtomicU16 = AtomicU16::new(0);
static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(false);

// Voltage thresholds
const VOLTAGE_DROP_THRESHOLD: u16 = 200; // 200mV drop is concerning
const CRITICAL_VOLTAGE: u16 = 3300; // 3.3V is minimum for stable operation
const USB_VOLTAGE_MIN: u16 = 4500; // Minimum expected USB voltage

pub struct VoltageMonitor {
    start_time: Instant,
    samples: Vec<(u16, Instant)>, // (voltage_mv, timestamp)
    initial_voltage: u16,
}

impl VoltageMonitor {
    pub fn start_monitoring() -> Self {
        // Read initial voltage
        let initial = Self::read_current_voltage();
        
        MONITORING_ACTIVE.store(true, Ordering::Relaxed);
        MIN_VOLTAGE.store(initial, Ordering::Relaxed);
        MAX_VOLTAGE.store(initial, Ordering::Relaxed);
        LAST_VOLTAGE.store(initial, Ordering::Relaxed);
        VOLTAGE_DROP_COUNT.store(0, Ordering::Relaxed);
        
        log::info!("Voltage monitoring started. Initial voltage: {}mV", initial);
        
        Self {
            start_time: Instant::now(),
            samples: vec![(initial, Instant::now())],
            initial_voltage: initial,
        }
    }
    
    pub fn sample(&mut self) {
        let voltage = Self::read_current_voltage();
        let now = Instant::now();
        
        self.samples.push((voltage, now));
        
        // Update global tracking
        let last = LAST_VOLTAGE.load(Ordering::Relaxed);
        LAST_VOLTAGE.store(voltage, Ordering::Relaxed);
        
        // Update min/max
        let mut min = MIN_VOLTAGE.load(Ordering::Relaxed);
        while voltage < min {
            match MIN_VOLTAGE.compare_exchange(min, voltage, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => min = x,
            }
        }
        
        let mut max = MAX_VOLTAGE.load(Ordering::Relaxed);
        while voltage > max {
            match MAX_VOLTAGE.compare_exchange(max, voltage, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => max = x,
            }
        }
        
        // Check for voltage drop
        if last > 0 && last.saturating_sub(voltage) > VOLTAGE_DROP_THRESHOLD {
            VOLTAGE_DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            log::warn!("Voltage drop detected: {}mV -> {}mV (drop: {}mV)", 
                      last, voltage, last - voltage);
        }
        
        // Check for critical voltage
        if voltage < CRITICAL_VOLTAGE {
            log::error!("CRITICAL: Voltage below safe threshold: {}mV", voltage);
        }
    }
    
    pub fn stop_monitoring(self) -> VoltageReport {
        MONITORING_ACTIVE.store(false, Ordering::Relaxed);
        
        let duration = self.start_time.elapsed();
        let min_voltage = MIN_VOLTAGE.load(Ordering::Relaxed);
        let max_voltage = MAX_VOLTAGE.load(Ordering::Relaxed);
        let drop_count = VOLTAGE_DROP_COUNT.load(Ordering::Relaxed);
        let final_voltage = LAST_VOLTAGE.load(Ordering::Relaxed);
        
        // Calculate max voltage drop
        let max_drop = self.initial_voltage.saturating_sub(min_voltage);
        
        // Determine if on USB power
        let is_usb = self.initial_voltage > USB_VOLTAGE_MIN;
        
        // Log summary
        log::info!("=== Voltage Monitoring Summary ===");
        log::info!("Duration: {:?}", duration);
        log::info!("Initial voltage: {}mV", self.initial_voltage);
        log::info!("Final voltage: {}mV", final_voltage);
        log::info!("Min voltage: {}mV", min_voltage);
        log::info!("Max voltage: {}mV", max_voltage);
        log::info!("Max drop: {}mV", max_drop);
        log::info!("Drop events: {}", drop_count);
        log::info!("Power source: {}", if is_usb { "USB" } else { "Battery" });
        
        // Warnings
        if max_drop > 500 {
            log::warn!("WARNING: Large voltage drop detected ({}mV). Power supply may be inadequate.", max_drop);
        }
        if min_voltage < CRITICAL_VOLTAGE {
            log::error!("ERROR: Voltage dropped below critical threshold!");
        }
        
        VoltageReport {
            _duration: duration,
            initial_voltage: self.initial_voltage,
            _final_voltage: final_voltage,
            min_voltage,
            _max_voltage: max_voltage,
            max_drop,
            drop_count,
            _is_usb: is_usb,
            _samples: self.samples,
        }
    }
    
    fn read_current_voltage() -> u16 {
        // Read from ADC using direct register access
        unsafe {
            use esp_idf_sys::*;
            
            // GPIO4 = ADC1 channel 3
            let raw_value = adc1_get_raw(adc1_channel_t_ADC1_CHANNEL_3);
            if raw_value < 0 {
                return 0;
            }
            
            // Convert to millivolts (with voltage divider compensation)
            let measured_mv = ((raw_value as u32 * 3100) / 4095) as u16;
            let battery_mv = measured_mv * 2; // 1:1 voltage divider
            
            battery_mv
        }
    }
}

pub struct VoltageReport {
    pub _duration: Duration,
    pub initial_voltage: u16,
    pub _final_voltage: u16,
    pub min_voltage: u16,
    pub _max_voltage: u16,
    pub max_drop: u16,
    pub drop_count: u16,
    pub _is_usb: bool,
    pub _samples: Vec<(u16, Instant)>,
}

impl VoltageReport {
    pub fn had_critical_drops(&self) -> bool {
        self.min_voltage < CRITICAL_VOLTAGE || self.max_drop > 500
    }
    
    pub fn get_diagnosis(&self) -> &'static str {
        if self.min_voltage < CRITICAL_VOLTAGE {
            "Critical voltage drop - power supply insufficient"
        } else if self.max_drop > 500 {
            "Significant voltage drop - power supply may be marginal"
        } else if self.drop_count > 5 {
            "Multiple voltage drops - power supply unstable"
        } else if self.max_drop > 200 {
            "Minor voltage drops - normal for WiFi operation"
        } else {
            "Voltage stable - power supply adequate"
        }
    }
}

// Helper function for WiFi initialization monitoring
pub fn monitor_wifi_init<F, R>(operation_name: &str, operation: F) -> Result<R>
where
    F: FnOnce() -> Result<R>,
{
    log::info!("Starting voltage monitoring for: {}", operation_name);
    
    let monitor = VoltageMonitor::start_monitoring();
    
    // Sample voltage every 100ms in background
    let monitor_handle = Arc::new(Mutex::new(monitor));
    
    // Start background sampling thread
    let _sampling_active = Arc::new(AtomicBool::new(true));
    
    // Since we're on ESP32, we can't use std::thread::spawn
    // Instead, we'll take a few manual samples before and after
    
    // Sample before operation
    if let Ok(mut m) = monitor_handle.lock() {
        m.sample();
    }
    
    // Run the operation
    let result = operation();
    
    // Sample after operation
    if let Ok(mut m) = monitor_handle.lock() {
        m.sample();
        m.sample(); // Extra sample
    }
    
    // Get report
    let report = match monitor_handle.lock() {
        Ok(mut monitor_guard) => {
            let final_monitor = std::mem::replace(&mut *monitor_guard, VoltageMonitor::start_monitoring());
            final_monitor.stop_monitoring()
        }
        Err(e) => {
            log::error!("Voltage monitor lock failed: {}", e);
            // Construct a conservative fallback report from global trackers
            VoltageReport {
                _duration: Duration::from_secs(0),
                initial_voltage: LAST_VOLTAGE.load(Ordering::Relaxed),
                _final_voltage: LAST_VOLTAGE.load(Ordering::Relaxed),
                min_voltage: MIN_VOLTAGE.load(Ordering::Relaxed),
                _max_voltage: MAX_VOLTAGE.load(Ordering::Relaxed),
                max_drop: 0,
                drop_count: VOLTAGE_DROP_COUNT.load(Ordering::Relaxed),
                _is_usb: false,
                _samples: Vec::new(),
            }
        }
    };
    
    // Log diagnosis
    log::info!("{}: {}", operation_name, report.get_diagnosis());
    
    // Add to diagnostics if we have critical issues
    if report.had_critical_drops() {
        crate::diagnostics::log_power_issue(operation_name, &report);
    }
    
    result
}

// Get current voltage reading
pub fn get_current_voltage() -> u16 {
    VoltageMonitor::read_current_voltage()
}

// Get monitoring statistics
pub fn get_voltage_stats() -> (u16, u16, u16, u16) {
    (
        MIN_VOLTAGE.load(Ordering::Relaxed),
        MAX_VOLTAGE.load(Ordering::Relaxed),
        LAST_VOLTAGE.load(Ordering::Relaxed),
        VOLTAGE_DROP_COUNT.load(Ordering::Relaxed),
    )
}