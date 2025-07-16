// Sensor abstraction layer for ESP32-S3 dashboard

use embassy_time::{Duration, Timer};
use heapless::Vec;
use anyhow;

pub mod temperature;
pub mod battery;
pub mod ambient_light;

pub use temperature::TemperatureSensor;
pub use battery::BatteryVoltageSensor;
pub use ambient_light::AmbientLightSensor;

// Sensor data struct for UI consumption
#[derive(Debug, Clone)]
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub battery_voltage: f32,
    pub battery_percentage: u8,
    pub light_level: u16,
}

impl Default for SensorData {
    fn default() -> Self {
        Self {
            temperature: 25.0,
            humidity: 50.0,
            pressure: 1013.25,
            battery_voltage: 3.7,
            battery_percentage: 100,
            light_level: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SensorError {
    InitializationFailed,
    ReadTimeout,
    InvalidReading,
    CalibrationFailed,
    NotAvailable,
}

// Core sensor trait that all sensors must implement
pub trait Sensor {
    type Reading;
    
    /// Initialize the sensor hardware
    fn init(&mut self) -> Result<(), SensorError>;
    
    /// Read current sensor value
    fn read(&mut self) -> Result<Self::Reading, SensorError>;
    
    /// Check if sensor is ready for reading
    fn is_ready(&self) -> bool;
    
    /// Get minimum time between readings
    fn min_read_interval(&self) -> Duration;
    
    /// Get sensor name for debugging
    fn name(&self) -> &'static str;
}

// Extended trait for sensors that support calibration
pub trait Calibratable: Sensor {
    fn calibrate(&mut self) -> Result<(), SensorError>;
    fn is_calibrated(&self) -> bool;
    fn reset_calibration(&mut self);
}

// Extended trait for sensors with configurable ranges
pub trait RangeConfigurable: Sensor {
    fn set_range(&mut self, min: f32, max: f32) -> Result<(), SensorError>;
    fn get_range(&self) -> (f32, f32);
}

// Sensor reading with metadata
#[derive(Debug, Clone)]
pub struct SensorReading<T> {
    pub value: T,
    pub timestamp: u32,  // Milliseconds since boot
    pub quality: ReadingQuality,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadingQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Invalid,
}

// Sensor fusion for combining multiple sensor readings
pub struct SensorFusion<T, const N: usize> {
    sensors: Vec<Box<dyn Sensor<Reading = T>>, N>,
    weights: [f32; N],
    last_readings: Vec<Option<T>, N>,
}

impl<T: Clone + Default, const N: usize> SensorFusion<T, N> {
    pub fn new() -> Self {
        Self {
            sensors: Vec::new(),
            weights: [1.0 / N as f32; N],
            last_readings: Vec::new(),
        }
    }
    
    pub fn add_sensor(&mut self, sensor: Box<dyn Sensor<Reading = T>>) -> Result<(), ()> {
        let _ = self.sensors.push(sensor);
        let _ = self.last_readings.push(None);
        Ok(())
    }
    
    pub fn set_weights(&mut self, weights: [f32; N]) {
        // Normalize weights
        let sum: f32 = weights.iter().sum();
        for (i, w) in weights.iter().enumerate() {
            self.weights[i] = w / sum;
        }
    }
    
    pub async fn read_all(&mut self) -> Vec<Result<T, SensorError>, N> {
        let mut results = Vec::new();
        
        for (i, sensor) in self.sensors.iter_mut().enumerate() {
            let result = sensor.read();
            if let Ok(ref value) = result {
                self.last_readings[i] = Some(value.clone());
            }
            results.push(result).ok();
        }
        
        results
    }
}

// Sensor history for tracking trends
pub struct SensorHistory<T, const N: usize> {
    readings: Vec<SensorReading<T>, N>,
    max_age: Duration,
}

impl<T: Clone, const N: usize> SensorHistory<T, N> {
    pub fn new(max_age: Duration) -> Self {
        Self {
            readings: Vec::new(),
            max_age,
        }
    }
    
    pub fn add(&mut self, reading: SensorReading<T>) -> Result<(), SensorReading<T>> {
        // Remove old readings
        let current_time = reading.timestamp;
        self.readings.retain(|r| {
            (current_time - r.timestamp) < self.max_age.as_millis() as u32
        });
        
        self.readings.push(reading)
    }
    
    pub fn get_latest(&self) -> Option<&SensorReading<T>> {
        self.readings.last()
    }
    
    pub fn get_average(&self) -> Option<T>
    where
        T: core::ops::Add<Output = T> + core::ops::Div<f32, Output = T> + Default + Copy,
    {
        if self.readings.is_empty() {
            return None;
        }
        
        let sum = self.readings.iter()
            .map(|r| r.value)
            .fold(T::default(), |acc, val| acc + val);
        
        Some(sum / self.readings.len() as f32)
    }
    
    pub fn get_trend(&self) -> Trend {
        if self.readings.len() < 2 {
            return Trend::Stable;
        }
        
        // Simple trend detection - can be made more sophisticated
        // This is a placeholder that would need T to implement comparison traits
        Trend::Stable
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Trend {
    Rising,
    Falling,
    Stable,
}

// Mock sensor for testing
#[cfg(test)]
pub struct MockSensor<T> {
    value: T,
    name: &'static str,
    ready: bool,
}

#[cfg(test)]
impl<T: Clone> MockSensor<T> {
    pub fn new(name: &'static str, value: T) -> Self {
        Self {
            value,
            name,
            ready: true,
        }
    }
    
    pub fn set_value(&mut self, value: T) {
        self.value = value;
    }
    
    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }
}

#[cfg(test)]
impl<T: Clone> Sensor for MockSensor<T> {
    type Reading = T;
    
    fn init(&mut self) -> Result<(), SensorError> {
        Ok(())
    }
    
    fn read(&mut self) -> Result<Self::Reading, SensorError> {
        if self.ready {
            Ok(self.value.clone())
        } else {
            Err(SensorError::NotAvailable)
        }
    }
    
    fn is_ready(&self) -> bool {
        self.ready
    }
    
    fn min_read_interval(&self) -> Duration {
        Duration::from_millis(100)
    }
    
    fn name(&self) -> &'static str {
        self.name
    }
}

// Sensor manager for coordinating multiple sensors
pub struct SensorManager {
    initialized: bool,
    battery_pin: Option<esp_idf_hal::gpio::AnyIOPin>,
}

impl SensorManager {
    pub fn new(battery_pin: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static) -> Result<Self, anyhow::Error> {
        Ok(Self {
            initialized: false,
            battery_pin: Some(battery_pin.into()),
        })
    }
    
    pub async fn init_all(&mut self) -> Result<(), SensorError> {
        // Initialize all sensors
        // This would be expanded with actual sensor initialization
        self.initialized = true;
        Ok(())
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn sample(&mut self) -> Result<SensorData, anyhow::Error> {
        // Return mock sensor data for now
        Ok(SensorData {
            temperature: 25.0,
            humidity: 60.0,
            pressure: 1013.25,
            battery_voltage: 3.7,
            battery_percentage: 85,
            light_level: 500,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sensor_reading_creation() {
        let reading = SensorReading {
            value: 25.5,
            timestamp: 1000,
            quality: ReadingQuality::Good,
        };
        
        assert_eq!(reading.value, 25.5);
        assert_eq!(reading.timestamp, 1000);
        assert!(matches!(reading.quality, ReadingQuality::Good));
    }
    
    #[test]
    fn test_mock_sensor() {
        let mut sensor = MockSensor::new("test", 42);
        
        assert_eq!(sensor.name(), "test");
        assert!(sensor.is_ready());
        assert_eq!(sensor.read().unwrap(), 42);
        
        sensor.set_ready(false);
        assert!(sensor.read().is_err());
    }
    
    #[test]
    fn test_sensor_history() {
        let mut history: SensorHistory<f32, 10> = SensorHistory::new(Duration::from_secs(60));
        
        let reading1 = SensorReading {
            value: 20.0,
            timestamp: 1000,
            quality: ReadingQuality::Good,
        };
        
        let reading2 = SensorReading {
            value: 22.0,
            timestamp: 2000,
            quality: ReadingQuality::Good,
        };
        
        history.add(reading1).unwrap();
        history.add(reading2).unwrap();
        
        assert_eq!(history.get_latest().unwrap().value, 22.0);
    }
}