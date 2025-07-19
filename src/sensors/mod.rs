// Sensor abstraction layer for ESP32-S3 dashboard

use anyhow;
// use crate::hardware::battery::BatteryMonitor;  // Temporarily disabled
use esp_idf_hal::gpio::Gpio4;
use esp_idf_hal::adc::ADC1;

// Sensor data struct for UI consumption
#[derive(Debug, Clone)]
pub struct SensorData {
    pub _temperature: f32,
    pub _battery_percentage: u8,
    pub _battery_voltage: u16,  // mV
    pub _is_charging: bool,
    pub _is_on_usb: bool,
    pub _light_level: u16,
}

impl Default for SensorData {
    fn default() -> Self {
        Self {
            _temperature: 25.0,
            _battery_percentage: 100,
            _battery_voltage: 4200,
            _is_charging: false,
            _is_on_usb: false,
            _light_level: 0,
        }
    }
}

// SensorError removed - not used

// Sensor traits and history tracking removed - not implemented yet

// Mock sensor removed - Sensor trait was removed

// Sensor manager for coordinating multiple sensors
pub struct SensorManager {
    // battery_monitor: Option<BatteryMonitor>,
}

impl SensorManager {
    pub fn new(_adc1: ADC1, _battery_pin: Gpio4) -> Result<Self, anyhow::Error> {
        // Battery monitoring temporarily disabled until ADC API is fixed
        log::warn!("Battery monitoring temporarily disabled - ADC API needs update");
        
        Ok(Self {
            // battery_monitor: None,
        })
    }
    
    pub fn sample(&mut self) -> Result<SensorData, anyhow::Error> {
        // Read internal temperature sensor
        let temperature = self.read_internal_temperature();
        
        // Read battery data - temporarily return simulated values
        // TODO: Fix ADC API and re-enable real battery monitoring
        let (battery_percentage, battery_voltage, is_charging, is_on_usb) = (75, 3800, false, false);
        
        Ok(SensorData {
            _temperature: temperature,
            _battery_percentage: battery_percentage,
            _battery_voltage: battery_voltage,
            _is_charging: is_charging,
            _is_on_usb: is_on_usb,
            _light_level: 0, // No light sensor on T-Display
        })
    }
    
    fn read_internal_temperature(&self) -> f32 {
        unsafe {
            use esp_idf_sys::*;
            
            // Initialize temperature sensor
            let tsens_config = temperature_sensor_config_t {
                range_min: -10,
                range_max: 80,
                clk_src: soc_periph_temperature_sensor_clk_src_t_TEMPERATURE_SENSOR_CLK_SRC_DEFAULT,
            };
            
            let mut handle: temperature_sensor_handle_t = std::ptr::null_mut();
            temperature_sensor_install(&tsens_config, &mut handle);
            temperature_sensor_enable(handle);
            
            // Read temperature
            let mut temp_celsius = 0.0f32;
            temperature_sensor_get_celsius(handle, &mut temp_celsius);
            
            // Clean up
            temperature_sensor_disable(handle);
            temperature_sensor_uninstall(handle);
            
            temp_celsius
        }
    }
    
    // Battery reading moved to battery_monitor
}

// Tests removed - test types no longer exist