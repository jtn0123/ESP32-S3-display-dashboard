// Temperature sensor implementation

use super::{Sensor, SensorError, Calibratable};
use embassy_time::Duration;

pub struct TemperatureSensor {
    // TODO: Add ADC pin when ADC API is available
    // pin: Option<AdcPin<'static>>,
    calibration_offset: f32,
    last_reading: Option<f32>,
    initialized: bool,
}

impl TemperatureSensor {
    pub fn new() -> Self {
        Self {
            // pin: None,
            calibration_offset: 0.0,
            last_reading: None,
            initialized: false,
        }
    }
    
    // TODO: Restore when ADC API is available
    // pub fn with_pin(mut self, pin: AdcPin<'static>) -> Self {
    //     self.pin = Some(pin);
    //     self
    // }
    
    fn adc_to_temperature(&self, adc_value: u16) -> f32 {
        // Convert ADC reading to temperature
        // This is a simplified linear conversion - real sensors would use
        // their specific conversion formula (e.g., thermistor equation)
        
        // Assuming 0-3.3V input, 12-bit ADC, and linear sensor
        // that outputs 10mV/°C with 500mV offset at 0°C
        let voltage = (adc_value as f32 / 4095.0) * 3.3;
        let temp_celsius = ((voltage - 0.5) * 100.0) + self.calibration_offset;
        
        temp_celsius
    }
    
    pub fn to_fahrenheit(celsius: f32) -> f32 {
        celsius * 9.0 / 5.0 + 32.0
    }
}

impl Sensor for TemperatureSensor {
    type Reading = f32;
    
    fn init(&mut self) -> Result<(), SensorError> {
        // TODO: Check ADC pin when available
        // if self.pin.is_some() {
        self.initialized = true;
        Ok(())
        // } else {
        //     Err(SensorError::InitializationFailed)
        // }
    }
    
    fn read(&mut self) -> Result<Self::Reading, SensorError> {
        if !self.initialized {
            return Err(SensorError::NotAvailable);
        }
        
        // In real implementation, this would read from ADC
        // For now, return a simulated value
        let simulated_adc = 2048; // Middle of 12-bit range
        let temperature = self.adc_to_temperature(simulated_adc);
        
        self.last_reading = Some(temperature);
        Ok(temperature)
    }
    
    fn is_ready(&self) -> bool {
        self.initialized
    }
    
    fn min_read_interval(&self) -> Duration {
        Duration::from_millis(100) // Temperature doesn't change rapidly
    }
    
    fn name(&self) -> &'static str {
        "Temperature"
    }
}

impl Calibratable for TemperatureSensor {
    fn calibrate(&mut self) -> Result<(), SensorError> {
        // Simple calibration: read current value and adjust offset
        // to make it match a known reference temperature
        if let Ok(current) = self.read() {
            // Assume room temperature is 22°C for calibration
            self.calibration_offset = 22.0 - current;
            Ok(())
        } else {
            Err(SensorError::CalibrationFailed)
        }
    }
    
    fn is_calibrated(&self) -> bool {
        self.calibration_offset != 0.0
    }
    
    fn reset_calibration(&mut self) {
        self.calibration_offset = 0.0;
    }
}

// Internal temperature sensor (built into ESP32)
pub struct InternalTemperatureSensor {
    initialized: bool,
}

impl InternalTemperatureSensor {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
}

impl Sensor for InternalTemperatureSensor {
    type Reading = f32;
    
    fn init(&mut self) -> Result<(), SensorError> {
        // Initialize internal temperature sensor
        // This would use esp_hal's temperature sensor API
        self.initialized = true;
        Ok(())
    }
    
    fn read(&mut self) -> Result<Self::Reading, SensorError> {
        if !self.initialized {
            return Err(SensorError::NotAvailable);
        }
        
        // Read from internal sensor
        // Simulated value for now
        Ok(45.0) // Typical ESP32 internal temperature
    }
    
    fn is_ready(&self) -> bool {
        self.initialized
    }
    
    fn min_read_interval(&self) -> Duration {
        Duration::from_secs(1) // Internal temp changes slowly
    }
    
    fn name(&self) -> &'static str {
        "Internal Temperature"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_temperature_conversion() {
        let sensor = TemperatureSensor::new();
        
        // Test ADC to temperature conversion
        let temp = sensor.adc_to_temperature(2048); // Middle of range
        assert!(temp > 15.0 && temp < 35.0); // Reasonable room temperature
        
        // Test Celsius to Fahrenheit
        assert_eq!(TemperatureSensor::to_fahrenheit(0.0), 32.0);
        assert_eq!(TemperatureSensor::to_fahrenheit(100.0), 212.0);
        assert_eq!(TemperatureSensor::to_fahrenheit(20.0), 68.0);
    }
    
    #[test]
    fn test_calibration() {
        let mut sensor = TemperatureSensor::new();
        sensor.initialized = true;
        
        // Initial offset should be zero
        assert_eq!(sensor.calibration_offset, 0.0);
        
        // After calibration, offset should be non-zero
        sensor.calibrate().ok();
        assert!(sensor.is_calibrated());
        
        // Reset should clear calibration
        sensor.reset_calibration();
        assert!(!sensor.is_calibrated());
    }
}