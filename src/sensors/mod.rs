// Sensor abstraction layer for ESP32-S3 dashboard

use anyhow;

// Individual sensor modules removed - not implemented yet

// Sensor data struct for UI consumption
#[derive(Debug, Clone)]
pub struct SensorData {
    pub temperature: f32,
    pub battery_percentage: u8,
    pub light_level: u16,
}

impl Default for SensorData {
    fn default() -> Self {
        Self {
            temperature: 25.0,
            battery_percentage: 100,
            light_level: 0,
        }
    }
}

// SensorError removed - not used

// Sensor traits and history tracking removed - not implemented yet

// Mock sensor removed - Sensor trait was removed

// Sensor manager for coordinating multiple sensors
pub struct SensorManager {
}

impl SensorManager {
    pub fn new(_battery_pin: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static) -> Result<Self, anyhow::Error> {
        Ok(Self {})
    }
    
    pub fn sample(&mut self) -> Result<SensorData, anyhow::Error> {
        // Return mock sensor data for now
        Ok(SensorData {
            temperature: 25.0,
            battery_percentage: 85,
            light_level: 500,
        })
    }
}

// Tests removed - test types no longer exist