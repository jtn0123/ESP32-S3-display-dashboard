// Sensor monitoring task for Core 1
// Handles temperature, battery, and other sensor readings at regular intervals

use std::sync::mpsc::Sender;
use anyhow::Result;
use esp_idf_sys::*;

#[derive(Debug, Clone)]
pub struct SensorUpdate {
    pub temperature: f32,
    pub battery_percentage: u8,
    pub battery_voltage: u16,  // mV
    pub is_charging: bool,
    pub is_on_usb: bool,
    pub cpu_usage_core0: u8,
    pub cpu_usage_core1: u8,
}

pub struct SensorTask {
    tx: Sender<SensorUpdate>,
    temp_sensor_handle: Option<temperature_sensor_handle_t>,
    // Circular buffers for filtering
    temp_history: Vec<f32>,
    temp_index: usize,
}

impl SensorTask {
    pub fn new() -> Result<Self> {
        // We'll create the channel externally and pass the sender
        Ok(Self::new_with_channel(std::sync::mpsc::channel().0))
    }
    
    pub fn new_with_channel(tx: Sender<SensorUpdate>) -> Self {
        // NOTE: Temperature sensor is initialized by main SensorManager on Core 0
        // Core 1 will read simulated values for now to avoid conflicts
        log::info!("Core 1 SensorTask: Using simulated temperature (main sensor on Core 0)");
        
        Self {
            tx,
            temp_sensor_handle: None,  // Don't initialize here to avoid conflicts
            temp_history: vec![45.0; 5],  // 5-sample moving average, typical ESP32 temp
            temp_index: 0,
        }
    }
    
    pub fn update(&mut self) -> Result<()> {
        // Read temperature
        let temperature = self.read_temperature();
        
        // Apply moving average filter
        self.temp_history[self.temp_index] = temperature;
        self.temp_index = (self.temp_index + 1) % self.temp_history.len();
        let filtered_temp = self.temp_history.iter().sum::<f32>() / self.temp_history.len() as f32;
        
        // Read battery (TODO: implement real battery reading when ADC API is fixed)
        let (battery_percentage, battery_voltage, is_charging, is_on_usb) = self.read_battery();
        
        // Read CPU usage
        let (cpu0, cpu1) = self.read_cpu_usage();
        
        // Format CPU usage - show "N/A" if 0 (not available)
        let cpu0_str = if cpu0 == 0 { "N/A".to_string() } else { format!("{}%", cpu0) };
        let cpu1_str = if cpu1 == 0 { "N/A".to_string() } else { format!("{}%", cpu1) };
        
        log::info!("Core 1 Sensor: Temp={:.1}°C (filtered={:.1}°C), Battery={}% ({:.2}V), CPU0={}, CPU1={}", 
            temperature, filtered_temp, battery_percentage, battery_voltage as f32 / 1000.0, cpu0_str, cpu1_str);
        
        // Send update to Core 0
        let update = SensorUpdate {
            temperature: filtered_temp,
            battery_percentage,
            battery_voltage,
            is_charging,
            is_on_usb,
            cpu_usage_core0: cpu0,
            cpu_usage_core1: cpu1,
        };
        
        // Send update (will block if channel is full)
        if let Err(e) = self.tx.send(update) {
            log::error!("Failed to send sensor update: {}", e);
        } else {
            log::debug!("Core 1: Sensor update sent to Core 0");
        }
        
        Ok(())
    }
    
    fn read_temperature(&self) -> f32 {
        // Use simulated temperature with realistic variation
        // ESP32-S3 typically runs between 40-55°C under normal load
        let time_ms = unsafe { esp_idf_sys::esp_timer_get_time() / 1000 };
        
        // Base temperature with slow variation
        let base_temp = 45.0;
        let slow_variation = ((time_ms as f64 / 30000.0).sin() * 3.0) as f32;
        let fast_variation = ((time_ms as f64 / 5000.0).sin() * 1.0) as f32;
        
        base_temp + slow_variation + fast_variation
    }
    
    fn read_battery(&self) -> (u8, u16, bool, bool) {
        // TODO: Implement real battery reading via ADC when available
        // For T-Display-S3:
        // - Battery voltage on GPIO4 (through voltage divider)
        // - Charging status would be on GPIO6 (if connected)
        
        // For now, detect if we're on USB power
        // When on USB, voltage is typically stable at ~5V
        // Return realistic values for USB power
        
        let is_on_usb = true; // Always true when powered via USB
        let is_charging = false; // No battery connected in most dev setups
        
        if is_on_usb {
            // USB powered - full "battery"
            (100, 4200, is_charging, is_on_usb)
        } else {
            // Battery powered (not implemented yet)
            let voltage = 3700; // Nominal 3.7V LiPo
            let percentage = 50; // Assume half charge
            (percentage, voltage, is_charging, is_on_usb)
        }
    }
    
    fn read_cpu_usage(&self) -> (u8, u8) {
        // Use a local CpuMonitor instance to get real CPU usage
        // Note: This creates a new instance each time, but that's OK since
        // we're reading system-wide idle task stats
        let mut cpu_monitor = crate::dual_core::CpuMonitor::new();
        
        // Get real CPU usage from FreeRTOS idle task statistics
        let (core0_usage, core1_usage) = cpu_monitor.get_cpu_usage();
        
        // Log the real values for debugging
        log::debug!("Real CPU usage: Core0={}%, Core1={}%", core0_usage, core1_usage);
        
        (core0_usage, core1_usage)
    }
}

// Drop implementation removed - temperature sensor is managed by Core 0 SensorManager