use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::hal::cpu::Core;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerMode {
    Normal,      // Full performance
    PowerSave,   // Reduced CPU freq, dimmed display
    Sleep,       // Display off, WiFi sleep
    DeepSleep,   // Deep sleep mode
}

pub struct PowerManager {
    current_mode: PowerMode,
    last_activity: Instant,
    dim_timeout: Duration,
    sleep_timeout: Duration,
    auto_dim_enabled: bool,
    current_brightness: u8,
}

impl PowerManager {
    pub fn new(dim_timeout_secs: u32, sleep_timeout_secs: u32) -> Self {
        Self {
            current_mode: PowerMode::Normal,
            last_activity: Instant::now(),
            dim_timeout: Duration::from_secs(dim_timeout_secs as u64),
            sleep_timeout: Duration::from_secs(sleep_timeout_secs as u64),
            auto_dim_enabled: true,
            current_brightness: 100,
        }
    }

    pub fn activity_detected(&mut self) {
        self.last_activity = Instant::now();
        
        // Wake up from power save modes
        if self.current_mode != PowerMode::Normal {
            self.set_power_mode(PowerMode::Normal).ok();
        }
    }

    pub fn update(&mut self) -> Result<PowerMode> {
        if !self.auto_dim_enabled {
            return Ok(self.current_mode);
        }

        let idle_time = self.last_activity.elapsed();

        let new_mode = if idle_time >= self.sleep_timeout {
            PowerMode::Sleep
        } else if idle_time >= self.dim_timeout {
            PowerMode::PowerSave
        } else {
            PowerMode::Normal
        };

        if new_mode != self.current_mode {
            self.set_power_mode(new_mode)?;
        }

        Ok(self.current_mode)
    }

    pub fn set_power_mode(&mut self, mode: PowerMode) -> Result<()> {
        log::info!("Setting power mode to {:?}", mode);

        match mode {
            PowerMode::Normal => {
                // Full performance
                self.set_cpu_frequency(240)?;
                self.current_brightness = 100;
            }
            PowerMode::PowerSave => {
                // Reduced performance
                self.set_cpu_frequency(80)?;
                self.current_brightness = 30;
            }
            PowerMode::Sleep => {
                // Minimal power
                self.set_cpu_frequency(40)?;
                self.current_brightness = 0;
                // TODO: Put WiFi to sleep
            }
            PowerMode::DeepSleep => {
                // Enter deep sleep
                self.enter_deep_sleep();
            }
        }

        self.current_mode = mode;
        Ok(())
    }

    fn set_cpu_frequency(&self, mhz: u32) -> Result<()> {
        // TODO: Implement CPU frequency scaling
        // This would require esp_pm_configure with the appropriate config
        log::info!("CPU frequency set to {} MHz", mhz);
        Ok(())
    }

    fn enter_deep_sleep(&self) -> ! {
        log::info!("Entering deep sleep mode");
        
        // Configure wake up sources
        unsafe {
            // Wake up on button press (GPIO0 - BOOT button)
            esp_idf_sys::esp_sleep_enable_ext0_wakeup(0, 0); // GPIO0, low level
            
            // Or wake up after timeout (e.g., 1 hour)
            esp_idf_sys::esp_sleep_enable_timer_wakeup(3600 * 1000000); // microseconds
            
            // Enter deep sleep (never returns)
            esp_idf_sys::esp_deep_sleep_start();
        }
    }

    pub fn get_current_mode(&self) -> PowerMode {
        self.current_mode
    }

    pub fn get_brightness(&self) -> u8 {
        self.current_brightness
    }

    pub fn set_auto_dim(&mut self, enabled: bool) {
        self.auto_dim_enabled = enabled;
        if !enabled && self.current_mode == PowerMode::PowerSave {
            self.set_power_mode(PowerMode::Normal).ok();
        }
    }

    pub fn get_power_stats(&self) -> PowerStats {
        PowerStats {
            current_mode: self.current_mode,
            idle_time: self.last_activity.elapsed(),
            auto_dim_enabled: self.auto_dim_enabled,
            brightness: self.current_brightness,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PowerStats {
    pub current_mode: PowerMode,
    pub idle_time: Duration,
    pub auto_dim_enabled: bool,
    pub brightness: u8,
}

// Power consumption estimates (mA)
impl PowerMode {
    pub fn estimated_current_ma(&self) -> u32 {
        match self {
            PowerMode::Normal => 120,      // WiFi active, display on, 240MHz
            PowerMode::PowerSave => 60,    // WiFi active, display dimmed, 80MHz
            PowerMode::Sleep => 20,        // WiFi sleep, display off
            PowerMode::DeepSleep => 1,     // Deep sleep mode
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PowerMode::Normal => "Normal",
            PowerMode::PowerSave => "Power Save",
            PowerMode::Sleep => "Sleep",
            PowerMode::DeepSleep => "Deep Sleep",
        }
    }
}