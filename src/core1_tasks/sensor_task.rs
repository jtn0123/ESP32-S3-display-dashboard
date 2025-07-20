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
        // Get real CPU usage from the main loop's cpu_monitor
        // Since we're on Core 1, we'll read the shared values
        // For now, still return simulated values but log that we need real implementation
        
        log::debug!("TODO: Implement real CPU usage monitoring via FreeRTOS stats");
        
        // Use timer to create some variation
        let time_ms = unsafe { esp_idf_sys::esp_timer_get_time() / 1000 };
        
        // Core 0: 40-60% (main UI core)
        let core0_base = 50;
        let core0_variation = ((time_ms / 2000) % 20) as i8 - 10;
        let core0_usage = (core0_base as i8 + core0_variation).max(0).min(100) as u8;
        
        // Core 1: 15-25% (background tasks)
        let core1_base = 20;
        let core1_variation = ((time_ms / 3000) % 10) as i8 - 5;
        let core1_usage = (core1_base as i8 + core1_variation).max(0).min(100) as u8;
        
        (core0_usage, core1_usage)
    }
}

// Drop implementation removed - temperature sensor is managed by Core 0 SensorManager