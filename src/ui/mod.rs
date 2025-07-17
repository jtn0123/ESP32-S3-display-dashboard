use anyhow::Result;
use crate::display::{DisplayManager, colors::*};
use crate::sensors::SensorData;
use crate::system::{ButtonEvent, SystemInfo};
use crate::ota::OtaStatus;
use std::time::Instant;

pub struct UiManager {
    current_screen: usize,
    sensor_data: SensorData,
    last_update: Instant,
    animation_progress: f32,
    last_rendered_screen: Option<usize>,
    system_info: SystemInfo,
    network_connected: bool,
    network_ip: Option<String>,
    network_ssid: String,
    ota_status: OtaStatus,
}

impl UiManager {
    #[allow(dead_code)]
    fn draw_header(&self, display: &mut DisplayManager, title: &str, bg_color: u16) -> Result<()> {
        // Header background
        display.fill_rect(0, 0, 320, 30, bg_color)?;
        
        // Title
        display.draw_text_centered(8, title, WHITE, None, 2)?;
        
        // Battery indicator
        display.fill_rect(5, 5, 50, 20, bg_color)?;
        let battery_color = if self.sensor_data._battery_percentage > 50 { WHITE } 
                           else if self.sensor_data._battery_percentage > 20 { YELLOW } 
                           else { PRIMARY_RED };
        let battery_str = format!("{}%", self.sensor_data._battery_percentage);
        display.draw_text(10, 8, &battery_str, battery_color, None, 1)?;
        
        // Time
        display.fill_rect(240, 5, 75, 20, bg_color)?;
        let time_str = self.system_info.format_uptime();
        display.draw_text(245, 8, &time_str, WHITE, None, 1)?;
        
        Ok(())
    }
    pub fn new(_display: &mut DisplayManager) -> Result<Self> {
        Ok(Self {
            current_screen: 0,
            sensor_data: SensorData::default(),
            last_update: Instant::now(),
            animation_progress: 0.0,
            last_rendered_screen: None,
            system_info: SystemInfo::new(),
            network_connected: false,
            network_ip: None,
            network_ssid: String::from("Not connected"),
            ota_status: OtaStatus::Idle,
        })
    }

    pub fn show_boot_screen(&mut self, display: &mut DisplayManager) -> Result<()> {
        log::info!("Showing boot screen");
        
        // Clear screen
        display.clear(BLACK)?;
        
        // Draw title
        display.draw_text_centered(50, "ESP32-S3 Dashboard", WHITE, None, 2)?;
        display.draw_text_centered(80, "Rust Edition", PRIMARY_BLUE, None, 1)?;
        
        // Draw progress bar
        display.draw_progress_bar(60, 120, 200, 20, 50, PRIMARY_BLUE, SURFACE_LIGHT, BORDER_COLOR)?;
        
        // Version info
        display.draw_text_centered(160, "v4.12", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }

    pub fn handle_button_event(&mut self, event: ButtonEvent) -> Result<()> {
        match event {
            ButtonEvent::Button1Click => {
                log::info!("Previous screen");
                self.current_screen = self.current_screen.saturating_sub(1);
                self.animation_progress = 0.0;
            }
            ButtonEvent::Button2Click => {
                log::info!("Next screen");
                self.current_screen = (self.current_screen + 1) % 4;
                self.animation_progress = 0.0;
            }
            ButtonEvent::Button1LongPress | ButtonEvent::Button2LongPress => {
                log::info!("Long press detected");
            }
            ButtonEvent::Button1Press | ButtonEvent::Button1Release | 
            ButtonEvent::Button2Press | ButtonEvent::Button2Release => {
                // Ignore press/release events, only handle clicks
            }
        }
        Ok(())
    }

    pub fn update_sensor_data(&mut self, data: SensorData) {
        self.sensor_data = data;
    }
    
    pub fn update_network_status(&mut self, connected: bool, ip: Option<String>, ssid: String) {
        self.network_connected = connected;
        self.network_ip = ip;
        self.network_ssid = ssid;
    }
    
    pub fn update_ota_status(&mut self, status: OtaStatus) {
        self.ota_status = status;
    }

    pub fn update(&mut self) -> Result<()> {
        // Update animation progress
        let elapsed = self.last_update.elapsed().as_secs_f32();
        self.animation_progress = (self.animation_progress + elapsed * 2.0).min(1.0);
        self.last_update = Instant::now();
        Ok(())
    }

    pub fn render(&mut self, display: &mut DisplayManager) -> Result<()> {
        // Only log screen changes, not every frame
        let screen_changed = self.last_rendered_screen != Some(self.current_screen);
        if screen_changed {
            log::info!("Switching to screen {}", self.current_screen);
            self.last_rendered_screen = Some(self.current_screen);
        }
        
        match self.current_screen {
            0 => self.render_system_screen(display, screen_changed),
            1 => self.render_network_screen(display, screen_changed),
            2 => self.render_sensor_screen(display, screen_changed),
            3 => self.render_settings_screen(display, screen_changed),
            _ => Ok(()),
        }
    }

    fn render_system_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        // Only clear screen when switching to this screen
        if screen_changed {
            log::info!("render_system_screen: Clearing screen for new screen");
            display.clear(BLACK)?;
            
            // Draw static elements that don't change
            // Header (using actual display width)
            display.fill_rect(0, 0, 320, 30, PRIMARY_BLUE)?;
            display.draw_text_centered(8, "System Status", WHITE, None, 2)?;
            
            // Static labels
            let y_start = 50;
            let line_height = 25;
            display.draw_text(10, y_start, "Uptime:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height, "Free Heap:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 2, "CPU Freq:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 3, "Flash:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 4, "Temp:", TEXT_PRIMARY, None, 1)?;
            
            // Button hints
            display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(230, 160, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        }
        
        // Update time in header (always update, even if screen didn't change)
        // Clear time area and show uptime as a clock
        display.fill_rect(240, 5, 75, 20, PRIMARY_BLUE)?;
        let time_str = self.system_info.format_uptime();
        display.draw_text(245, 8, &time_str, WHITE, None, 1)?;
        
        // Battery indicator in header
        display.fill_rect(5, 5, 50, 20, PRIMARY_BLUE)?;
        let battery_color = if self.sensor_data._battery_percentage > 50 { PRIMARY_GREEN } 
                           else if self.sensor_data._battery_percentage > 20 { YELLOW } 
                           else { PRIMARY_RED };
        let battery_str = format!("{}%", self.sensor_data._battery_percentage);
        display.draw_text(10, 8, &battery_str, battery_color, None, 1)?;
        
        // Dynamic content - update values by clearing their areas first
        let y_start = 50;
        let line_height = 25;
        
        // Uptime value (clear old value area)
        display.fill_rect(120, y_start, 100, 20, BLACK)?;
        let uptime = self.system_info.format_uptime();
        display.draw_text(120, y_start, &uptime, PRIMARY_GREEN, None, 1)?;
        
        // Memory value
        let heap_kb = self.system_info.get_free_heap_kb();
        let heap_str = format!("{} KB", heap_kb);
        display.fill_rect(120, y_start + line_height, 100, 20, BLACK)?;
        display.draw_text(120, y_start + line_height, &heap_str, PRIMARY_GREEN, None, 1)?;
        
        // CPU value
        display.fill_rect(120, y_start + line_height * 2, 100, 20, BLACK)?;
        let cpu_freq = self.system_info.get_cpu_freq_mhz();
        let cpu_str = format!("{} MHz", cpu_freq);
        display.draw_text(120, y_start + line_height * 2, &cpu_str, PRIMARY_GREEN, None, 1)?;
        
        // Flash storage value
        display.fill_rect(120, y_start + line_height * 3, 100, 20, BLACK)?;
        let (flash_total, app_size) = self.system_info.get_flash_info();
        let flash_str = format!("{}/{}MB", app_size, flash_total);
        display.draw_text(120, y_start + line_height * 3, &flash_str, PRIMARY_GREEN, None, 1)?;
        
        // Temperature value
        display.fill_rect(120, y_start + line_height * 4, 100, 20, BLACK)?;
        let temp_str = format!("{:.1}°C", self.sensor_data._temperature);
        let temp_color = if self.sensor_data._temperature > 50.0 { PRIMARY_RED } 
                        else if self.sensor_data._temperature > 40.0 { YELLOW } 
                        else { PRIMARY_GREEN };
        display.draw_text(120, y_start + line_height * 4, &temp_str, temp_color, None, 1)?;
        
        // Progress indicator (move down to make room)
        let progress = (self.animation_progress * 100.0) as u8;
        display.draw_progress_bar(10, 140, 280, 15, progress, PRIMARY_GREEN, SURFACE_LIGHT, BORDER_COLOR)?;
        
        Ok(())
    }

    fn render_network_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        if screen_changed {
            // Clear screen
            display.clear(BLACK)?;
            
            // Header
            display.fill_rect(0, 0, 320, 30, PRIMARY_PURPLE)?;
            display.draw_text_centered(8, "Network Status", WHITE, None, 2)?;
            
            // Static labels
            let y_start = 50;
            let line_height = 25;
            display.draw_text(10, y_start, "WiFi:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height, "SSID:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 2, "IP:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, 130, "Signal:", TEXT_PRIMARY, None, 1)?;
            
            // Button hints
            display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(230, 160, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        }
        
        // Update time in header
        display.fill_rect(240, 5, 75, 20, PRIMARY_PURPLE)?;
        let time_str = self.system_info.format_uptime();
        display.draw_text(245, 8, &time_str, WHITE, None, 1)?;
        
        // Dynamic content
        let y_start = 50;
        let line_height = 25;
        
        // WiFi Status
        display.fill_rect(120, y_start, 180, 20, BLACK)?;
        let status_text = if self.network_connected { "Connected" } else { "Disconnected" };
        let status_color = if self.network_connected { PRIMARY_GREEN } else { PRIMARY_RED };
        display.draw_text(120, y_start, status_text, status_color, None, 1)?;
        
        // SSID
        display.fill_rect(120, y_start + line_height, 180, 20, BLACK)?;
        display.draw_text(120, y_start + line_height, &self.network_ssid, TEXT_PRIMARY, None, 1)?;
        
        // IP Address
        display.fill_rect(120, y_start + line_height * 2, 180, 20, BLACK)?;
        let ip_text = self.network_ip.as_deref().unwrap_or("No IP");
        display.draw_text(120, y_start + line_height * 2, ip_text, TEXT_PRIMARY, None, 1)?;
        
        // Signal strength indicator
        let signal_strength = 75;
        display.draw_text(10, 110, "Signal:", TEXT_PRIMARY, None, 1)?;
        display.draw_progress_bar(80, 110, 100, 10, signal_strength, PRIMARY_GREEN, SURFACE_LIGHT, BORDER_COLOR)?;
        display.draw_text(190, 110, &format!("{}%", signal_strength), TEXT_PRIMARY, None, 1)?;
        
        // OTA Status
        display.draw_text(10, 130, "OTA Status:", TEXT_PRIMARY, None, 1)?;
        let (ota_text, ota_color) = match self.ota_status {
            OtaStatus::Idle => ("Ready", TEXT_SECONDARY),
            OtaStatus::Downloading { progress } => {
                display.draw_progress_bar(120, 145, 180, 10, progress, PRIMARY_BLUE, SURFACE_LIGHT, BORDER_COLOR)?;
                ("Downloading", PRIMARY_BLUE)
            },
            OtaStatus::Verifying => ("Verifying", YELLOW),
            OtaStatus::Ready => ("Update Ready", PRIMARY_GREEN),
            OtaStatus::Failed => ("Failed", PRIMARY_RED),
        };
        display.fill_rect(120, 130, 180, 20, BLACK)?;
        display.draw_text(120, 130, ota_text, ota_color, None, 1)?;
        
        // Web URLs
        if self.network_connected {
            display.draw_text_centered(95, &format!("Config: http://{}", self.network_ip.as_deref().unwrap_or("?.?.?.?")), TEXT_SECONDARY, None, 1)?;
            display.draw_text_centered(105, &format!("OTA: http://{}:8080/ota", self.network_ip.as_deref().unwrap_or("?.?.?.?")), TEXT_SECONDARY, None, 1)?;
        }
        
        // Button hints
        display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
        display.draw_text(230, 160, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }

    fn render_sensor_screen(&mut self, display: &mut DisplayManager, _screen_changed: bool) -> Result<()> {
        // Clear screen
        display.clear(BLACK)?;
        
        // Header
        display.fill_rect(0, 0, 320, 30, PRIMARY_GREEN)?;
        display.draw_text_centered(8, "Sensor Data", WHITE, None, 2)?;
        
        // Sensor values
        let y_start = 50;
        let line_height = 30;
        
        // Battery
        display.draw_text(10, y_start, "Battery:", TEXT_PRIMARY, None, 1)?;
        let battery_percent = self.sensor_data._battery_percentage;
        let battery_color = if battery_percent > 50 { PRIMARY_GREEN } else if battery_percent > 20 { YELLOW } else { PRIMARY_RED };
        display.draw_progress_bar(100, y_start, 150, 15, battery_percent, battery_color, SURFACE_LIGHT, BORDER_COLOR)?;
        display.draw_text(260, y_start, &format!("{}%", battery_percent), battery_color, None, 1)?;
        
        // Temperature
        display.draw_text(10, y_start + line_height, "Temp:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(100, y_start + line_height, &format!("{}°C", self.sensor_data._temperature), TEXT_PRIMARY, None, 1)?;
        
        // Light level
        display.draw_text(10, y_start + line_height * 2, "Light:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(100, y_start + line_height * 2, &format!("{} lux", self.sensor_data._light_level), TEXT_PRIMARY, None, 1)?;
        
        // Visual indicator
        let radius = 20;
        let cx = 160;
        let cy = 130;
        display.draw_circle(cx, cy, radius, BORDER_COLOR)?;
        let fill_radius = (radius as f32 * self.animation_progress) as u16;
        if fill_radius > 0 {
            display.fill_circle(cx, cy, fill_radius, PRIMARY_GREEN)?;
        }
        
        // Button hints
        display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
        display.draw_text(230, 160, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }

    fn render_settings_screen(&mut self, display: &mut DisplayManager, _screen_changed: bool) -> Result<()> {
        // Clear screen
        display.clear(BLACK)?;
        
        // Header
        display.fill_rect(0, 0, 320, 30, ACCENT_ORANGE)?;
        display.draw_text_centered(8, "Settings", WHITE, None, 2)?;
        
        // Settings options
        let y_start = 50;
        let line_height = 30;
        
        // Brightness
        display.draw_text(10, y_start, "Brightness:", TEXT_PRIMARY, None, 1)?;
        display.draw_progress_bar(120, y_start, 100, 15, 80, PRIMARY_BLUE, SURFACE_LIGHT, BORDER_COLOR)?;
        display.draw_text(230, y_start, "80%", TEXT_PRIMARY, None, 1)?;
        
        // Auto-dim
        display.draw_text(10, y_start + line_height, "Auto-dim:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height, "ON", PRIMARY_GREEN, None, 1)?;
        
        // Update speed
        display.draw_text(10, y_start + line_height * 2, "Update:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height * 2, "Normal", TEXT_PRIMARY, None, 1)?;
        
        // Version
        display.draw_text(10, y_start + line_height * 3, "Version:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height * 3, "v4.13-rust", TEXT_SECONDARY, None, 1)?;
        
        // Button hints
        display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
        display.draw_text(230, 160, "[USER] Select", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }
}