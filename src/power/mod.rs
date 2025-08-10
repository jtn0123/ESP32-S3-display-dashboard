// Power management system for ESP32-S3 dashboard

// pub mod voltage_monitor; // removed (unused)

use std::time::{Duration, Instant};
use esp_idf_hal::gpio::{AnyIOPin, Output, PinDriver};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerMode {
    Active,      // Full brightness, all features enabled
    PowerSave,   // Minimal brightness, reduced update rate
    Sleep,       // Display off, wake on button press
}

#[derive(Debug, Clone, Copy)]
pub struct PowerConfig {
    pub active_brightness: u8,         // 0-100
    pub power_save_brightness: u8,     // 0-100
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            active_brightness: 100,
            power_save_brightness: 20,
        }
    }
}

pub struct PowerManager {
    current_mode: PowerMode,
    last_activity: Instant,
    config: PowerConfig,
    brightness_level: u8,
    backlight_pin: Option<PinDriver<'static, AnyIOPin, Output>>,
    force_power_save: bool,
}

impl PowerManager {
    pub fn new(config: PowerConfig) -> Self {
        Self {
            current_mode: PowerMode::Active,
            last_activity: Instant::now(),
            config,
            brightness_level: config.active_brightness,
            backlight_pin: None,
            force_power_save: false,
        }
    }
    
    pub fn with_backlight(mut self, backlight_pin: PinDriver<'static, AnyIOPin, Output>) -> Self {
        self.backlight_pin = Some(backlight_pin);
        self
    }
    
    pub fn activity_detected(&mut self) {
        log::info!("PowerManager: activity_detected called, current_mode = {:?}", self.current_mode);
        self.last_activity = Instant::now();
        
        // Wake from sleep or power save
        if self.current_mode == PowerMode::Sleep || self.current_mode == PowerMode::PowerSave {
            log::info!("PowerManager: waking from sleep/power save to Active mode");
            self.set_mode(PowerMode::Active);
        }
    }
    
    
    fn set_mode(&mut self, mode: PowerMode) {
        self.current_mode = mode;
        
        // Update brightness based on mode
        self.brightness_level = match mode {
            PowerMode::Active => self.config.active_brightness,
            PowerMode::PowerSave => self.config.power_save_brightness,
            PowerMode::Sleep => 0,
        };
        
        self.update_backlight();
    }
    
    fn update_backlight(&mut self) {
        if let Some(ref mut pin) = self.backlight_pin {
            if self.brightness_level == 0 {
                // Turn off backlight
                pin.set_low().ok();
            } else {
                // PWM implementation would require LEDC peripheral
                // Currently using simple on/off control
                pin.set_high().ok();
            }
        }
    }
    
    pub fn get_mode(&self) -> PowerMode {
        self.current_mode
    }
    
    pub fn get_brightness(&self) -> u8 {
        self.brightness_level
    }
    
    pub fn get_update_rate(&self) -> Duration {
        match self.current_mode {
            PowerMode::Active => Duration::from_millis(33),      // 30 FPS
            PowerMode::PowerSave => Duration::from_millis(100),  // 10 FPS
            PowerMode::Sleep => Duration::from_secs(1),          // 1 FPS (minimal)
        }
    }
    
    
    pub fn set_brightness(&mut self, brightness: u8) {
        // Manual brightness adjustment
        self.brightness_level = brightness.min(100);
        self.update_backlight();
        
        // If manually adjusting brightness, ensure we're not in sleep
        if self.current_mode == PowerMode::Sleep && brightness > 0 {
            self.set_mode(PowerMode::Active);
        }
    }
    
    pub fn get_power_stats(&self) -> PowerStats {
        PowerStats {
            mode: self.current_mode,
            brightness: self.brightness_level,
            idle_time: self.last_activity.elapsed(),
            force_power_save: self.force_power_save,
        }
    }
}

#[derive(Debug)]
pub struct PowerStats {
    pub mode: PowerMode,
    pub brightness: u8,
    pub idle_time: Duration,
    pub force_power_save: bool,
}

// Task-specific power management
pub struct TaskPowerManager {
    wifi_enabled: bool,
    sensor_polling_rate: Duration,
    display_refresh_rate: Duration,
}

impl TaskPowerManager {
    pub fn new() -> Self {
        Self {
            wifi_enabled: true,
            sensor_polling_rate: Duration::from_secs(5),
            display_refresh_rate: Duration::from_millis(33),
        }
    }
    
    pub fn apply_power_mode(&mut self, mode: PowerMode) {
        match mode {
            PowerMode::Active => {
                self.wifi_enabled = true;
                self.sensor_polling_rate = Duration::from_secs(5);
                self.display_refresh_rate = Duration::from_millis(33);
            }
            PowerMode::PowerSave => {
                self.wifi_enabled = false;
                self.sensor_polling_rate = Duration::from_secs(30);
                self.display_refresh_rate = Duration::from_millis(100);
            }
            PowerMode::Sleep => {
                self.wifi_enabled = false;
                self.sensor_polling_rate = Duration::from_secs(60);
                self.display_refresh_rate = Duration::from_secs(1);
            }
        }
    }
    
    pub fn should_poll_sensors(&self, last_poll: Instant) -> bool {
        last_poll.elapsed() >= self.sensor_polling_rate
    }
    
    pub fn should_refresh_display(&self, last_refresh: Instant) -> bool {
        last_refresh.elapsed() >= self.display_refresh_rate
    }
    
    pub fn is_wifi_enabled(&self) -> bool {
        self.wifi_enabled
    }
}

// Battery-aware power optimization
pub fn calculate_optimal_brightness(battery_percentage: u8, ambient_light: u16) -> u8 {
    // Base brightness on ambient light
    let base_brightness = if ambient_light < 100 {
        30  // Dark environment
    } else if ambient_light < 1000 {
        60  // Indoor lighting
    } else {
        100 // Bright/outdoor
    };
    
    // Adjust based on battery level
    let battery_factor = if battery_percentage < 20 {
        0.5  // Half brightness when battery is low
    } else if battery_percentage < 50 {
        0.75 // 75% brightness when battery is medium
    } else {
        1.0  // Full brightness when battery is good
    };
    
    (base_brightness as f32 * battery_factor) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_power_mode_transitions() {
        let config = PowerConfig::default();
        let mut pm = PowerManager::new(config);
        
        assert_eq!(pm.get_mode(), PowerMode::Active);
        
        // Simulate low battery
        let mut sensor_data = SensorData::default();
        sensor_data._battery_percentage = 15;
        pm.update(&sensor_data);
        
        // Should be in power save due to low battery
        assert_eq!(pm.get_mode(), PowerMode::PowerSave);
    }
    
    #[test]
    fn test_brightness_calculation() {
        // Low battery, dark environment
        assert_eq!(calculate_optimal_brightness(15, 50), 15);
        
        // Good battery, bright environment
        assert_eq!(calculate_optimal_brightness(80, 2000), 100);
        
        // Medium battery, indoor lighting
        assert_eq!(calculate_optimal_brightness(40, 500), 45);
    }
    
    #[test]
    fn test_update_rates() {
        let pm = PowerManager::new(PowerConfig::default());
        
        // Different modes should have different update rates
        assert!(pm.get_update_rate().as_millis() < 50); // Active mode
    }
}