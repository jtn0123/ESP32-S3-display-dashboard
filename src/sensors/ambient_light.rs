// Ambient light sensor implementation

use super::{Sensor, SensorError, RangeConfigurable};
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum LightCondition {
    Dark,           // < 10 lux
    Dim,            // 10-100 lux
    Indoor,         // 100-1000 lux
    Bright,         // 1000-10000 lux
    Daylight,       // > 10000 lux
}

pub struct AmbientLightSensor {
    gain: SensorGain,
    integration_time: IntegrationTime,
    last_reading: Option<u16>,
    initialized: bool,
    min_lux: f32,
    max_lux: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum SensorGain {
    Low,     // 1x
    Medium,  // 16x
    High,    // 64x
    Max,     // 128x
}

#[derive(Debug, Clone, Copy)]
pub enum IntegrationTime {
    Fast,    // 100ms
    Medium,  // 200ms
    Slow,    // 400ms
}

impl AmbientLightSensor {
    pub fn new() -> Self {
        Self {
            gain: SensorGain::Medium,
            integration_time: IntegrationTime::Medium,
            last_reading: None,
            initialized: false,
            min_lux: 0.0,
            max_lux: 120000.0, // Bright sunlight
        }
    }
    
    pub fn set_gain(&mut self, gain: SensorGain) {
        self.gain = gain;
    }
    
    pub fn set_integration_time(&mut self, time: IntegrationTime) {
        self.integration_time = time;
    }
    
    fn raw_to_lux(&self, raw: u16) -> f32 {
        // Convert raw sensor reading to lux
        // This is sensor-specific and simplified here
        let gain_factor = match self.gain {
            SensorGain::Low => 1.0,
            SensorGain::Medium => 16.0,
            SensorGain::High => 64.0,
            SensorGain::Max => 128.0,
        };
        
        let time_factor = match self.integration_time {
            IntegrationTime::Fast => 1.0,
            IntegrationTime::Medium => 2.0,
            IntegrationTime::Slow => 4.0,
        };
        
        (raw as f32) / (gain_factor * time_factor)
    }
    
    pub fn get_light_condition(lux: f32) -> LightCondition {
        match lux as u32 {
            0..=10 => LightCondition::Dark,
            11..=100 => LightCondition::Dim,
            101..=1000 => LightCondition::Indoor,
            1001..=10000 => LightCondition::Bright,
            _ => LightCondition::Daylight,
        }
    }
    
    pub fn auto_adjust_gain(&mut self, current_raw: u16) {
        // Auto-adjust gain based on current reading
        if current_raw < 100 && !matches!(self.gain, SensorGain::Max) {
            // Too dark, increase gain
            self.gain = match self.gain {
                SensorGain::Low => SensorGain::Medium,
                SensorGain::Medium => SensorGain::High,
                SensorGain::High => SensorGain::Max,
                SensorGain::Max => SensorGain::Max,
            };
        } else if current_raw > 60000 && !matches!(self.gain, SensorGain::Low) {
            // Too bright, decrease gain
            self.gain = match self.gain {
                SensorGain::Low => SensorGain::Low,
                SensorGain::Medium => SensorGain::Low,
                SensorGain::High => SensorGain::Medium,
                SensorGain::Max => SensorGain::High,
            };
        }
    }
}

impl Sensor for AmbientLightSensor {
    type Reading = LightReading;
    
    fn init(&mut self) -> Result<(), SensorError> {
        // Initialize I2C and configure sensor
        self.initialized = true;
        Ok(())
    }
    
    fn read(&mut self) -> Result<Self::Reading, SensorError> {
        if !self.initialized {
            return Err(SensorError::NotAvailable);
        }
        
        // Simulate sensor reading
        let raw_value = match self.get_simulated_time_of_day() {
            0..=6 => 50,      // Night
            7..=8 => 500,     // Morning
            9..=17 => 5000,   // Day
            18..=20 => 1000,  // Evening
            _ => 100,         // Night
        };
        
        // Auto-adjust gain if needed
        self.auto_adjust_gain(raw_value);
        
        let lux = self.raw_to_lux(raw_value);
        let condition = Self::get_light_condition(lux);
        
        self.last_reading = Some(raw_value);
        
        Ok(LightReading {
            lux,
            raw: raw_value,
            condition,
            gain: self.gain,
        })
    }
    
    fn is_ready(&self) -> bool {
        self.initialized
    }
    
    fn min_read_interval(&self) -> Duration {
        match self.integration_time {
            IntegrationTime::Fast => Duration::from_millis(100),
            IntegrationTime::Medium => Duration::from_millis(200),
            IntegrationTime::Slow => Duration::from_millis(400),
        }
    }
    
    fn name(&self) -> &'static str {
        "Ambient Light"
    }
}

impl RangeConfigurable for AmbientLightSensor {
    fn set_range(&mut self, min: f32, max: f32) -> Result<(), SensorError> {
        if min >= 0.0 && max > min {
            self.min_lux = min;
            self.max_lux = max;
            Ok(())
        } else {
            Err(SensorError::InvalidReading)
        }
    }
    
    fn get_range(&self) -> (f32, f32) {
        (self.min_lux, self.max_lux)
    }
}

impl AmbientLightSensor {
    // Helper for simulation
    fn get_simulated_time_of_day(&self) -> u8 {
        // In real implementation, this would use RTC
        12 // Noon
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LightReading {
    pub lux: f32,
    pub raw: u16,
    pub condition: LightCondition,
    pub gain: SensorGain,
}

impl LightReading {
    pub fn get_display_brightness_suggestion(&self) -> u8 {
        // Suggest display brightness based on ambient light
        match self.condition {
            LightCondition::Dark => 20,
            LightCondition::Dim => 40,
            LightCondition::Indoor => 60,
            LightCondition::Bright => 80,
            LightCondition::Daylight => 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_light_conditions() {
        assert!(matches!(AmbientLightSensor::get_light_condition(5.0), LightCondition::Dark));
        assert!(matches!(AmbientLightSensor::get_light_condition(50.0), LightCondition::Dim));
        assert!(matches!(AmbientLightSensor::get_light_condition(500.0), LightCondition::Indoor));
        assert!(matches!(AmbientLightSensor::get_light_condition(5000.0), LightCondition::Bright));
        assert!(matches!(AmbientLightSensor::get_light_condition(50000.0), LightCondition::Daylight));
    }
    
    #[test]
    fn test_auto_gain_adjustment() {
        let mut sensor = AmbientLightSensor::new();
        
        // Test increase gain on dark reading
        sensor.gain = SensorGain::Low;
        sensor.auto_adjust_gain(50);
        assert!(matches!(sensor.gain, SensorGain::Medium));
        
        // Test decrease gain on bright reading
        sensor.gain = SensorGain::High;
        sensor.auto_adjust_gain(65000);
        assert!(matches!(sensor.gain, SensorGain::Medium));
    }
    
    #[test]
    fn test_brightness_suggestions() {
        let reading = LightReading {
            lux: 5.0,
            raw: 100,
            condition: LightCondition::Dark,
            gain: SensorGain::High,
        };
        
        assert_eq!(reading.get_display_brightness_suggestion(), 20);
    }
}