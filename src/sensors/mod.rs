// Sensor abstraction layer for ESP32-S3 dashboard

pub mod history;

use anyhow::Result;
use esp_idf_hal::gpio::Gpio4;
use esp_idf_hal::adc::ADC1;

// Battery monitoring helper functions
fn voltage_to_percentage(voltage: u16) -> u8 {
    // Convert millivolts to volts for calculation
    let voltage_v = voltage as f32 / 1000.0;
    // Simple linear approximation: 3.0V = 0%, 4.2V = 100%
    let percentage = ((voltage_v - 3.0) / 1.2 * 100.0).clamp(0.0, 100.0);
    percentage as u8
}

fn is_battery_connected(_adc_raw: u16, voltage: u16) -> bool {
    voltage > 2500 // Battery connected if voltage > 2.5V (in millivolts)
}

fn is_on_usb_power(voltage: u16, battery_connected: bool) -> bool {
    voltage > 4500 || !battery_connected // > 4.5V indicates USB power
}

fn is_charging(voltage: u16, battery_connected: bool) -> bool {
    battery_connected && voltage > 4000 // > 4.0V and battery connected = charging
}

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
pub struct SensorManager {
    temp_sensor_handle: Option<esp_idf_sys::temperature_sensor_handle_t>,
    last_battery_voltage: u16,
    last_adc_raw: u16,
    // ADC channel pin number for battery monitoring (GPIO4)
    battery_pin: u8,
    // Stability: avoid global mutable counters
    sample_count: u32,
}

impl SensorManager {
    pub fn new(_adc1: ADC1, _battery_pin: Gpio4) -> Result<Self> {
        // For now, we'll use direct ADC register access for battery monitoring
        // The esp-idf-hal ADC API seems to have changed
        log::info!("Battery monitoring initialized (using direct register access)");
        
        log::info!("Initializing temperature sensor...");
        
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
                log::error!("Failed to install temperature sensor: {} (ESP_ERR code)", ret);
                None
            } else {
                let enable_ret = temperature_sensor_enable(handle);
                if enable_ret != 0 {
                    log::error!("Failed to enable temperature sensor: {} (ESP_ERR code)", enable_ret);
                    temperature_sensor_uninstall(handle);
                    None
                } else {
                    log::info!("Temperature sensor initialized successfully, handle: {:?}", handle);
                    // Try to read immediately to verify it's working
                    let mut test_temp = 0.0f32;
                    let read_ret = temperature_sensor_get_celsius(handle, &mut test_temp);
                    if read_ret == 0 {
                        log::info!("Temperature sensor test read: {:.1}°C", test_temp);
                    } else {
                        log::warn!("Temperature sensor test read failed: {}", read_ret);
                    }
                    Some(handle)
                }
            }
        };
        
        // Initialize ADC for battery monitoring using direct register access
        Self::init_adc_direct();
        
        // Read initial battery voltage
        let initial_raw = Self::read_adc_direct(4); // GPIO4 is ADC1 channel 3
        let initial_voltage = Self::adc_to_millivolts(initial_raw);
        log::info!("Initial battery reading: {} raw, {} mV", initial_raw, initial_voltage);
        
        Ok(Self {
            temp_sensor_handle: temp_handle,
            last_battery_voltage: initial_voltage,
            last_adc_raw: initial_raw,
            battery_pin: 4, // GPIO4
            sample_count: 0,
        })
    }
    
    // Initialize ADC using direct register access
    fn init_adc_direct() {
        unsafe {
            use esp_idf_sys::*;
            
            // ADC power is managed automatically in newer ESP-IDF
            // No need to call adc_power_acquire()
            
            // Configure ADC1 for 12-bit resolution
            adc1_config_width(adc_bits_width_t_ADC_WIDTH_BIT_12);
            
            // Configure channel 3 (GPIO4) with 11dB attenuation
            adc1_config_channel_atten(adc1_channel_t_ADC1_CHANNEL_3, adc_atten_t_ADC_ATTEN_DB_11);
            
            log::info!("ADC1 initialized for battery monitoring on GPIO4 (channel 3)");
        }
    }
    
    // Read ADC value using direct register access
    fn read_adc_direct(gpio_num: u8) -> u16 {
        unsafe {
            use esp_idf_sys::*;
            
            // GPIO4 = ADC1 channel 3
            let channel = if gpio_num == 4 {
                adc1_channel_t_ADC1_CHANNEL_3
            } else {
                log::warn!("Unsupported GPIO {} for ADC", gpio_num);
                return 0;
            };
            
            // Read ADC value
            let raw_value = adc1_get_raw(channel);
            if raw_value < 0 {
                log::warn!("ADC read failed: {}", raw_value);
                0
            } else {
                raw_value as u16
            }
        }
    }
    
    // Convert ADC reading to millivolts
    // ESP32-S3 ADC with 11dB attenuation: 0-3100mV range
    fn adc_to_millivolts(adc_value: u16) -> u16 {
        // T-Display-S3 has a voltage divider on the battery pin (GPIO4)
        // The divider is 100k + 100k, so we measure half the battery voltage
        // With 11dB attenuation, the ADC range is approximately 0-3100mV
        // ADC resolution is 12-bit (0-4095)
        
        // First convert ADC to measured voltage
        let measured_mv = ((adc_value as u32 * 3100) / 4095) as u16;
        
        // Then double it to get actual battery voltage (due to 1:1 voltage divider)
        let battery_mv = measured_mv * 2;
        battery_mv
    }
    
    // Update battery voltage from external ADC reading
    pub fn sample(&mut self) -> Result<SensorData> {
        // Read internal temperature sensor
        let temperature = self.read_internal_temperature();
        
        // Read battery voltage from ADC using direct register access
        let adc_raw = Self::read_adc_direct(self.battery_pin);
        let battery_voltage = Self::adc_to_millivolts(adc_raw);
        
        // Update stored values
        self.last_adc_raw = adc_raw;
        self.last_battery_voltage = battery_voltage;
        
        // Calculate battery metrics
        let battery_percentage = voltage_to_percentage(battery_voltage);
        let battery_connected = is_battery_connected(adc_raw, battery_voltage);
        let is_on_usb = is_on_usb_power(battery_voltage, battery_connected);
        let is_charging = is_charging(battery_voltage, battery_connected);
        
        // Log battery readings periodically (every 10th sample to reduce spam)
        self.sample_count = self.sample_count.wrapping_add(1);
        if self.sample_count % 10 == 0 {
            log::warn!("[BATTERY_SAMPLE] Voltage: {}mV ({:.3}V), Percentage: {}%, ADC raw: {}, USB: {}, Charging: {}", 
                battery_voltage, battery_voltage as f32 / 1000.0, battery_percentage, adc_raw, is_on_usb, is_charging);
            
            // Extra debug for voltage divider calculation
            let measured_at_pin = ((adc_raw as u32 * 3100) / 4095) as u16;
            log::warn!("[BATTERY_SAMPLE] ADC pin voltage: {}mV (before 2x multiplier)", measured_at_pin);
        }
        
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
                    25.0 // Default to reasonable ambient temp
                } else {
                    // Note: ESP32-S3 internal temperature sensor reads the die temperature,
                    // which is typically 20-40°C above ambient temperature.
                    // For user display, we estimate ambient by subtracting an offset.
                    const DIE_TO_AMBIENT_OFFSET: f32 = 35.0; // Typical offset at normal operation
                    let ambient_estimate = temp_celsius - DIE_TO_AMBIENT_OFFSET;
                    
                    log::info!("Temperature sensor: Die={:.1}°C, Ambient≈{:.1}°C", 
                              temp_celsius, ambient_estimate);
                    
                    // Return estimated ambient temperature for display
                    ambient_estimate
                }
            }
        } else {
            log::warn!("Temperature sensor not initialized, using default");
            25.0 // Default ambient temperature
        }
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
        
        // ADC power is managed automatically in newer ESP-IDF
        // No need to release ADC power manually
    }
}