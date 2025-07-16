use anyhow::Result;
use crate::display::{DisplayManager, colors::*};
use crate::sensors::SensorData;
use crate::system::ButtonEvent;
use std::time::Instant;

pub struct UiManager {
    current_screen: usize,
    sensor_data: SensorData,
    last_update: Instant,
    animation_progress: f32,
}

impl UiManager {
    pub fn new(_display: &mut DisplayManager) -> Result<Self> {
        Ok(Self {
            current_screen: 0,
            sensor_data: SensorData::default(),
            last_update: Instant::now(),
            animation_progress: 0.0,
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
        display.draw_text_centered(160, "v1.0.0", TEXT_SECONDARY, None, 1)?;
        
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
            _ => {}
        }
        Ok(())
    }

    pub fn update_sensor_data(&mut self, data: SensorData) {
        self.sensor_data = data;
    }

    pub fn update(&mut self) -> Result<()> {
        // Update animation progress
        let elapsed = self.last_update.elapsed().as_secs_f32();
        self.animation_progress = (self.animation_progress + elapsed * 2.0).min(1.0);
        self.last_update = Instant::now();
        Ok(())
    }

    pub fn render(&mut self, display: &mut DisplayManager) -> Result<()> {
        match self.current_screen {
            0 => self.render_system_screen(display),
            1 => self.render_network_screen(display),
            2 => self.render_sensor_screen(display),
            3 => self.render_settings_screen(display),
            _ => Ok(()),
        }
    }

    fn render_system_screen(&mut self, display: &mut DisplayManager) -> Result<()> {
        // Clear screen
        display.clear(BLACK)?;
        
        // Header (using actual display width)
        display.fill_rect(0, 0, 300, 30, PRIMARY_BLUE)?;
        display.draw_text_centered(8, "System Status", WHITE, None, 2)?;
        
        // System info
        let y_start = 50;
        let line_height = 25;
        
        // Uptime
        display.draw_text(10, y_start, "Uptime:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start, "00:05:32", PRIMARY_GREEN, None, 1)?;
        
        // Memory
        let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
        let heap_str = format!("{} KB", free_heap / 1024);
        display.draw_text(10, y_start + line_height, "Free Heap:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height, &heap_str, PRIMARY_GREEN, None, 1)?;
        
        // CPU
        display.draw_text(10, y_start + line_height * 2, "CPU Freq:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height * 2, "240 MHz", PRIMARY_GREEN, None, 1)?;
        
        // Progress indicator
        let progress = (self.animation_progress * 100.0) as u8;
        display.draw_progress_bar(10, 140, 300, 15, progress, PRIMARY_GREEN, SURFACE_LIGHT, BORDER_COLOR)?;
        
        // Button hints
        display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
        display.draw_text(230, 160, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }

    fn render_network_screen(&mut self, display: &mut DisplayManager) -> Result<()> {
        // Clear screen
        display.clear(BLACK)?;
        
        // Header
        display.fill_rect(0, 0, 320, 30, PRIMARY_PURPLE)?;
        display.draw_text_centered(8, "Network Status", WHITE, None, 2)?;
        
        // Network info
        let y_start = 50;
        let line_height = 25;
        
        // WiFi Status
        display.draw_text(10, y_start, "WiFi:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start, "Connected", PRIMARY_GREEN, None, 1)?;
        
        // SSID
        display.draw_text(10, y_start + line_height, "SSID:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height, "ESP-Network", TEXT_PRIMARY, None, 1)?;
        
        // IP Address
        display.draw_text(10, y_start + line_height * 2, "IP:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(120, y_start + line_height * 2, "192.168.1.100", TEXT_PRIMARY, None, 1)?;
        
        // Signal strength indicator
        let signal_strength = 75;
        display.draw_text(10, 130, "Signal:", TEXT_PRIMARY, None, 1)?;
        display.draw_progress_bar(80, 130, 100, 10, signal_strength, PRIMARY_GREEN, SURFACE_LIGHT, BORDER_COLOR)?;
        display.draw_text(190, 130, &format!("{}%", signal_strength), TEXT_PRIMARY, None, 1)?;
        
        // Button hints
        display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
        display.draw_text(230, 160, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }

    fn render_sensor_screen(&mut self, display: &mut DisplayManager) -> Result<()> {
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
        let battery_percent = self.sensor_data.battery_percentage;
        let battery_color = if battery_percent > 50 { PRIMARY_GREEN } else if battery_percent > 20 { YELLOW } else { PRIMARY_RED };
        display.draw_progress_bar(100, y_start, 150, 15, battery_percent, battery_color, SURFACE_LIGHT, BORDER_COLOR)?;
        display.draw_text(260, y_start, &format!("{}%", battery_percent), battery_color, None, 1)?;
        
        // Temperature
        display.draw_text(10, y_start + line_height, "Temp:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(100, y_start + line_height, &format!("{}°C", self.sensor_data.temperature), TEXT_PRIMARY, None, 1)?;
        
        // Light level
        display.draw_text(10, y_start + line_height * 2, "Light:", TEXT_PRIMARY, None, 1)?;
        display.draw_text(100, y_start + line_height * 2, &format!("{} lux", self.sensor_data.light_level), TEXT_PRIMARY, None, 1)?;
        
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

    fn render_settings_screen(&mut self, display: &mut DisplayManager) -> Result<()> {
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
        display.draw_text(120, y_start + line_height * 3, "1.0.0-rust", TEXT_SECONDARY, None, 1)?;
        
        // Button hints
        display.draw_text(10, 160, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
        display.draw_text(230, 160, "[USER] Select", TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }
}