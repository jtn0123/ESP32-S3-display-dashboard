// Sensor abstraction layer for ESP32-S3 dashboard

use anyhow::Result;
use crate::hardware::battery::BatteryMonitor;
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

// Sensor manager for coordinating multiple sensors
// NOTE: Due to lifetime constraints with ADC drivers, we don't store them here
// Instead, ADC reading is handled externally and passed in
pub struct SensorManager {
    battery_monitor: BatteryMonitor,
    temp_sensor_handle: Option<esp_idf_sys::temperature_sensor_handle_t>,
    last_battery_voltage: u16,
    last_adc_raw: u16,
}

impl SensorManager {
    pub fn new(_adc1: ADC1, _battery_pin: Gpio4) -> Result<Self> {
        // Create battery monitor
        let battery_monitor = BatteryMonitor::new();
        
        // Initialize temperature sensor
        let temp_handle = unsafe {
            use esp_idf_sys::*;
            
            let tsens_config = temperature_sensor_config_t {
                range_min: -10,
                range_max: 80,
                clk_src: soc_periph_temperature_sensor_clk_src_t_TEMPERATURE_SENSOR_CLK_SRC_DEFAULT,
            };
            
            let mut handle: temperature_sensor_handle_t = std::ptr::null_mut();
            let ret = temperature_sensor_install(&tsens_config, &mut handle);
            if ret != 0 {
                log::warn!("Failed to install temperature sensor: {}", ret);
                None
            } else {
                temperature_sensor_enable(handle);
                log::info!("Temperature sensor initialized successfully");
                Some(handle)
            }
        };
        
        Ok(Self {
            battery_monitor,
            temp_sensor_handle: temp_handle,
            last_battery_voltage: 4200, // Default to full battery
            last_adc_raw: 4095,
        })
    }
    
    // Update battery voltage from external ADC reading
    pub fn update_battery_voltage(&mut self, voltage: u16, adc_raw: u16) {
        self.last_battery_voltage = voltage;
        self.last_adc_raw = adc_raw;
    }
    
    pub fn sample(&mut self) -> Result<SensorData> {
        // Read internal temperature sensor
        let temperature = self.read_internal_temperature();
        
        // Use last known battery values
        let battery_voltage = self.last_battery_voltage;
        let battery_percentage = BatteryMonitor::voltage_to_percentage(battery_voltage);
        let battery_connected = BatteryMonitor::is_battery_connected(self.last_adc_raw, battery_voltage);
        let is_on_usb = BatteryMonitor::is_on_usb_power(battery_voltage, battery_connected);
        let is_charging = BatteryMonitor::is_charging(battery_voltage, battery_connected);
        
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
        if let Some(handle) = self.temp_sensor_handle {
            unsafe {
                use esp_idf_sys::*;
                
                let mut temp_celsius = 0.0f32;
                let ret = temperature_sensor_get_celsius(handle, &mut temp_celsius);
                if ret != 0 {
                    log::warn!("Failed to read temperature: {}", ret);
                    25.0 // Default fallback
                } else {
                    temp_celsius
                }
            }
        } else {
            log::debug!("Temperature sensor not initialized, using default");
            25.0 // Default fallback
        }
    }
    
    pub fn get_battery_monitor_mut(&mut self) -> &mut BatteryMonitor {
        &mut self.battery_monitor
    }
}

impl Drop for SensorManager {
    fn drop(&mut self) {
        if let Some(handle) = self.temp_sensor_handle {
            unsafe {
                use esp_idf_sys::*;
                temperature_sensor_disable(handle);
                temperature_sensor_uninstall(handle);
            }
        }
    }
}