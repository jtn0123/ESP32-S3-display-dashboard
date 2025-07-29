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
        // Core 1 will return N/A values to avoid confusion with fake data
        log::info!("Core 1 SensorTask: Hardware sensors on Core 0, returning N/A values");
        
        Self {
            tx,
            temp_sensor_handle: None,  // Don't initialize here to avoid conflicts
            temp_history: vec![0.0; 5],  // 0.0 indicates N/A
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
        
        // Format values - show "N/A" if 0 (not available)
        let temp_str = if temperature == 0.0 { "N/A".to_string() } else { format!("{:.1}°C", temperature) };
        let battery_str = if battery_percentage == 0 { "N/A".to_string() } else { format!("{}%", battery_percentage) };
        let voltage_str = if battery_voltage == 0 { "N/A".to_string() } else { format!("{:.2}V", battery_voltage as f32 / 1000.0) };
        let cpu0_str = if cpu0 == 0 { "N/A".to_string() } else { format!("{}%", cpu0) };
        let cpu1_str = if cpu1 == 0 { "N/A".to_string() } else { format!("{}%", cpu1) };
        
        log::info!("Core 1 Sensor: Temp={}, Battery={} ({}), CPU0={}, CPU1={}", 
            temp_str, battery_str, voltage_str, cpu0_str, cpu1_str);
        
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
        // Core 1 cannot access the temperature sensor (initialized on Core 0)
        // Return 0.0 to indicate "not available" - Core 0 has the real sensor
        // This prevents confusion from fake/simulated values
        log::debug!("Core 1: Temperature reading not available (sensor on Core 0)");
        0.0
    }
    
    fn read_battery(&self) -> (u8, u16, bool, bool) {
        // Core 1 cannot access ADC hardware (initialized on Core 0)
        // Return zeros to indicate "not available" - Core 0 has the real battery monitor
        // This prevents confusion from fake/simulated values
        log::debug!("Core 1: Battery reading not available (ADC on Core 0)");
        (0, 0, false, false)
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