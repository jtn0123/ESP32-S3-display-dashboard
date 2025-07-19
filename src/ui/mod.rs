use anyhow::Result;
use crate::display::{DisplayManager, colors::*};
use crate::sensors::SensorData;
use crate::system::{ButtonEvent, SystemInfo};
use crate::ota::OtaStatus;
use std::time::Instant;

// Text cache entry
#[derive(Clone)]
struct TextCache {
    text: String,
    x: u16,
    y: u16,
    color: u16,
    rendered: bool,
}

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
    network_signal: i8,
    network_gateway: Option<String>,
    network_mac: String,
    ota_status: OtaStatus,
    // FPS tracking
    fps: f32,
    frame_count: u32,
    last_fps_update: Instant,
    // Cached values to avoid redundant updates
    cached_uptime: String,
    cached_heap: String,
    cached_cpu: String,
    cached_flash: String,
    cached_temp: String,
    cached_battery: u8,
    // Pre-allocated string buffer for formatting
    string_buffer: String,
    // Skip render counter
    skip_renders: u32,
    total_renders: u32,
    // Text cache for static labels
    text_cache: Vec<TextCache>,
    // OTA screen caching
    cached_ota_time: u64,
    cached_ota_status: String,
    cached_ota_endpoints: (String, String),
    cached_network_ip: Option<String>,
    cached_ota_status_enum: Option<OtaStatus>,
    ota_screen_initialized: bool,
    // Screen initialization flags for optimized rendering
    system_screen_initialized: bool,
    network_screen_initialized: bool,
    sensor_screen_initialized: bool,
    settings_screen_initialized: bool,
    // Global time caching for all screens
    global_cached_time: u64,
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
            network_signal: -100,
            network_gateway: None,
            network_mac: String::from("Unknown"),
            ota_status: OtaStatus::Idle,
            fps: 0.0,
            frame_count: 0,
            last_fps_update: Instant::now(),
            cached_uptime: String::new(),
            cached_heap: String::new(),
            cached_cpu: String::new(),
            cached_flash: String::new(),
            cached_temp: String::new(),
            cached_battery: 0,
            string_buffer: String::with_capacity(32),
            skip_renders: 0,
            total_renders: 0,
            text_cache: Vec::with_capacity(20),
            cached_ota_time: 0,
            cached_ota_status: String::new(),
            cached_ota_endpoints: (String::new(), String::new()),
            cached_network_ip: None,
            cached_ota_status_enum: None,
            ota_screen_initialized: false,
            system_screen_initialized: false,
            network_screen_initialized: false,
            sensor_screen_initialized: false,
            settings_screen_initialized: false,
            global_cached_time: 0,
        })
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
                self.current_screen = (self.current_screen + 1) % 5;
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
        // Force render when sensor data updates
        self.force_next_render();
    }
    
    pub fn update_network_status(&mut self, connected: bool, ip: Option<String>, ssid: String, signal: i8, gateway: Option<String>, mac: String) {
        self.network_connected = connected;
        self.network_ip = ip;
        self.network_ssid = ssid;
        self.network_signal = signal;
        self.network_gateway = gateway;
        self.network_mac = mac;
    }
    
    pub fn update_ota_status(&mut self, status: OtaStatus) {
        self.ota_status = status;
    }
    
    fn force_next_render(&mut self) {
        // Force render by clearing cached values
        self.cached_uptime.clear();
        self.cached_heap.clear();
        self.cached_cpu.clear();
        self.cached_flash.clear();
        self.cached_temp.clear();
        self.cached_battery = 255; // Invalid value to force update
    }

    pub fn update(&mut self) -> Result<()> {
        // Update animation progress with frame skipping
        let elapsed = self.last_update.elapsed().as_secs_f32();
        
        // Only update animation every 100ms to reduce overhead
        if elapsed > 0.1 {
            self.animation_progress = (self.animation_progress + elapsed * 2.0).min(1.0);
            self.last_update = Instant::now();
        }
        
        Ok(())
    }

    pub fn render(&mut self, display: &mut DisplayManager) -> Result<()> {
        // Track if anything needs updating
        static mut RENDER_NEEDED: bool = true;
        
        self.total_renders += 1;
        
        // Always count frames for FPS
        self.frame_count += 1;
        let elapsed = self.last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 1.0 {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_fps_update = Instant::now();
            unsafe { RENDER_NEEDED = true; }
        }
        
        // Check if screen changed
        let screen_changed = self.last_rendered_screen != Some(self.current_screen);
        if screen_changed {
            log::info!("Switching to screen {}", self.current_screen);
            self.last_rendered_screen = Some(self.current_screen);
            unsafe { RENDER_NEEDED = true; }
            
            // Reset all screen initialization flags when switching
            self.system_screen_initialized = false;
            self.network_screen_initialized = false;
            self.sensor_screen_initialized = false;
            self.settings_screen_initialized = false;
            self.ota_screen_initialized = false;
        }
        
        // Skip render if nothing changed (except on screen change)
        unsafe {
            if !RENDER_NEEDED && !screen_changed {
                self.skip_renders += 1;
                // Still need to update and render FPS counter
                self.render_fps_counter(display)?;
                return Ok(());
            }
            RENDER_NEEDED = false; // Reset flag
        }
        
        // Log render efficiency every 100 renders
        if self.total_renders % 100 == 0 {
            let skip_rate = (self.skip_renders as f32 / self.total_renders as f32) * 100.0;
            log::info!("Render efficiency: {:.1}% skipped ({}/{})", 
                      skip_rate, self.skip_renders, self.total_renders);
        }
        
        // Render the current screen
        match self.current_screen {
            0 => self.render_system_screen(display, screen_changed)?,
            1 => self.render_network_screen(display, screen_changed)?,
            2 => self.render_sensor_screen(display, screen_changed)?,
            3 => self.render_settings_screen(display, screen_changed)?,
            4 => self.render_ota_screen(display, screen_changed)?,
            _ => {}
        }
        
        // Render FPS counter (always visible in corner)
        self.render_fps_counter(display)?;
        
        // Render OTA overlay if OTA is in progress
        if let OtaStatus::Downloading { progress } = self.ota_status {
            self.render_ota_overlay(display, progress)?;
        }
        
        Ok(())
    }

    fn render_system_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        // Early exit if nothing needs updating
        if !screen_changed && self.system_screen_initialized {
            // Check if any values actually changed
            let heap_kb = self.system_info.get_free_heap_kb();
            let heap_str = format!("{} KB", heap_kb);
            let cpu_freq = self.system_info.get_cpu_freq_mhz();
            let cpu_str = format!("{} MHz", cpu_freq);
            let temp_str = format!("{:.1}°C", self.sensor_data._temperature);
            
            if heap_str == self.cached_heap && 
               cpu_str == self.cached_cpu && 
               temp_str == self.cached_temp &&
               self.sensor_data._battery_percentage == self.cached_battery {
                // Only update time every 5 seconds
                let current_seconds = self.system_info.get_uptime().as_secs();
                if current_seconds >= self.global_cached_time + 5 {
                    self.global_cached_time = current_seconds;
                    display.fill_rect(235, 5, 65, 20, PRIMARY_BLUE)?;
                    let time_str = self.system_info.format_uptime();
                    display.draw_text(240, 8, &time_str, WHITE, None, 1)?;
                }
                return Ok(());
            }
        }
        
        // Only clear screen when switching to this screen
        if screen_changed {
            log::info!("render_system_screen: Clearing screen for new screen");
            display.clear(BLACK)?;
            display.flush()?; // Flush immediately to clear old content
            
            // Draw static elements that don't change
            // Header (using actual display width)
            display.fill_rect(0, 0, 300, 30, PRIMARY_BLUE)?;
            display.draw_text_centered(8, "System Status", WHITE, None, 2)?;
            
            // Reset initialization flag
            self.system_screen_initialized = false;
            
            // Static labels - cache them instead of redrawing
            let y_start = 45;
            let line_height = 20;
            
            // Add static labels to cache
            self.text_cache.clear();
            self.text_cache.push(TextCache { text: "Uptime:".to_string(), x: 10, y: y_start, color: TEXT_PRIMARY, rendered: false });
            self.text_cache.push(TextCache { text: "Free Heap:".to_string(), x: 10, y: y_start + line_height, color: TEXT_PRIMARY, rendered: false });
            self.text_cache.push(TextCache { text: "CPU Freq:".to_string(), x: 10, y: y_start + line_height * 2, color: TEXT_PRIMARY, rendered: false });
            self.text_cache.push(TextCache { text: "Flash:".to_string(), x: 10, y: y_start + line_height * 3, color: TEXT_PRIMARY, rendered: false });
            self.text_cache.push(TextCache { text: "Temp:".to_string(), x: 10, y: y_start + line_height * 4, color: TEXT_PRIMARY, rendered: false });
            
            // Render cached text
            for cache_entry in &mut self.text_cache {
                if !cache_entry.rendered {
                    display.draw_text(cache_entry.x, cache_entry.y, &cache_entry.text, cache_entry.color, None, 1)?;
                    cache_entry.rendered = true;
                }
            }
            
            // Button hints (moved up to avoid overlap)
            display.draw_text(10, 150, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(200, 150, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        }
        
        // Set screen as initialized after first render
        self.system_screen_initialized = true;
        
        // Update time in header (only update every 5 seconds or on first render)
        let current_seconds = self.system_info.get_uptime().as_secs();
        if current_seconds >= self.global_cached_time + 5 || !self.system_screen_initialized {
            self.global_cached_time = current_seconds;
            display.fill_rect(235, 5, 65, 20, PRIMARY_BLUE)?;
            let time_str = self.system_info.format_uptime();
            display.draw_text(240, 8, &time_str, WHITE, None, 1)?;
        }
        
        // Battery indicator in header (only update if changed)
        if self.sensor_data._battery_percentage != self.cached_battery || self.sensor_data._is_charging {
            display.fill_rect(5, 5, 65, 20, PRIMARY_BLUE)?;
            let battery_color = if self.sensor_data._is_charging { PRIMARY_BLUE }
                               else if self.sensor_data._battery_percentage > 50 { PRIMARY_GREEN } 
                               else if self.sensor_data._battery_percentage > 20 { YELLOW } 
                               else { PRIMARY_RED };
            
            let battery_str = if self.sensor_data._is_charging {
                format!("{}%+", self.sensor_data._battery_percentage)
            } else if self.sensor_data._is_on_usb && self.sensor_data._battery_percentage == 0 {
                "USB".to_string()
            } else {
                format!("{}%", self.sensor_data._battery_percentage)
            };
            display.draw_text(10, 8, &battery_str, battery_color, None, 1)?;
            self.cached_battery = self.sensor_data._battery_percentage;
        }
        
        // Dynamic content - update values by clearing their areas first
        let y_start = 45;
        let line_height = 20;
        
        // Uptime value (only update if changed)
        let uptime = self.system_info.get_uptime();
        let uptime_seconds = uptime.as_secs();
        
        // Use pre-allocated buffer for formatting
        self.string_buffer.clear();
        if uptime_seconds < 3600 {
            use std::fmt::Write;
            let _ = write!(&mut self.string_buffer, "{}m {}s", uptime_seconds / 60, uptime_seconds % 60);
        } else {
            use std::fmt::Write;
            let _ = write!(&mut self.string_buffer, "{}h {}m", uptime_seconds / 3600, (uptime_seconds % 3600) / 60);
        }
        let uptime_str = self.string_buffer.clone();
        if uptime_str != self.cached_uptime {
            display.fill_rect(120, y_start, 120, 16, BLACK)?;
            display.draw_text(120, y_start, &uptime_str, PRIMARY_GREEN, None, 1)?;
            self.cached_uptime = uptime_str;
        }
        
        // Memory value (only update if changed)
        let heap_kb = self.system_info.get_free_heap_kb();
        let heap_str = format!("{} KB", heap_kb);
        if heap_str != self.cached_heap {
            display.fill_rect(120, y_start + line_height, 120, 16, BLACK)?;
            display.draw_text(120, y_start + line_height, &heap_str, PRIMARY_GREEN, None, 1)?;
            self.cached_heap = heap_str;
        }
        
        // CPU value (only update if changed)
        let cpu_freq = self.system_info.get_cpu_freq_mhz();
        let cpu_str = format!("{} MHz", cpu_freq);
        if cpu_str != self.cached_cpu {
            display.fill_rect(120, y_start + line_height * 2, 120, 16, BLACK)?;
            display.draw_text(120, y_start + line_height * 2, &cpu_str, PRIMARY_GREEN, None, 1)?;
            self.cached_cpu = cpu_str;
        }
        
        // Flash storage value (only update if changed)
        let (flash_total, app_size) = self.system_info.get_flash_info();
        let flash_str = format!("{}/{}MB", app_size, flash_total);
        if flash_str != self.cached_flash {
            display.fill_rect(120, y_start + line_height * 3, 120, 16, BLACK)?;
            display.draw_text(120, y_start + line_height * 3, &flash_str, PRIMARY_GREEN, None, 1)?;
            self.cached_flash = flash_str;
        }
        
        // Temperature value (only update if changed)
        let temp_str = format!("{:.1}°C", self.sensor_data._temperature);
        if temp_str != self.cached_temp {
            display.fill_rect(120, y_start + line_height * 4, 120, 16, BLACK)?;
            let temp_color = if self.sensor_data._temperature > 50.0 { PRIMARY_RED } 
                            else if self.sensor_data._temperature > 40.0 { YELLOW } 
                            else { PRIMARY_GREEN };
            display.draw_text(120, y_start + line_height * 4, &temp_str, temp_color, None, 1)?;
            self.cached_temp = temp_str;
        }
        
        // Progress indicator (only update when progress changes)
        static mut LAST_PROGRESS: u8 = 255; // Invalid initial value
        let progress = (self.animation_progress * 100.0) as u8;
        
        unsafe {
            if progress != LAST_PROGRESS {
                LAST_PROGRESS = progress;
                display.draw_progress_bar(10, 138, 280, 8, progress, PRIMARY_GREEN, SURFACE_LIGHT, BORDER_COLOR)?;
            }
        }
        
        Ok(())
    }

    fn render_network_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        if screen_changed {
            // Clear screen
            display.clear(BLACK)?;
            display.flush()?; // Flush immediately to clear old content
            
            // Header
            display.fill_rect(0, 0, 300, 30, PRIMARY_PURPLE)?;
            display.draw_text_centered(8, "Network Status", WHITE, None, 2)?;
            
            // Reset initialization
            self.network_screen_initialized = false;
            
            // Static labels - consistent layout
            let y_start = 38;
            let line_height = 20;
            display.draw_text(10, y_start, "Status:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height, "SSID:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 2, "IP:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 3, "Signal:", TEXT_PRIMARY, None, 1)?;
            
            // Button hints
            display.draw_text(10, 155, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(200, 155, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        }
        
        // Update time in header (every 5 seconds)
        let current_seconds = self.system_info.get_uptime().as_secs();
        if current_seconds >= self.global_cached_time + 5 || screen_changed {
            self.global_cached_time = current_seconds;
            display.fill_rect(240, 5, 75, 20, PRIMARY_PURPLE)?;
            let time_str = self.system_info.format_uptime();
            display.draw_text(245, 8, &time_str, WHITE, None, 1)?;
        }
        
        // Dynamic content - consistent spacing
        let y_start = 38;
        let line_height = 20;
        let value_x = 65; // Better aligned position for values
        
        // WiFi Status
        display.fill_rect(value_x, y_start, 240, 16, BLACK)?;
        let status_text = if self.network_connected { "Connected" } else { "Disconnected" };
        let status_color = if self.network_connected { PRIMARY_GREEN } else { PRIMARY_RED };
        display.draw_text(value_x, y_start, status_text, status_color, None, 1)?;
        
        // SSID
        display.fill_rect(value_x, y_start + line_height, 240, 16, BLACK)?;
        let ssid_color = if self.network_connected { TEXT_PRIMARY } else { TEXT_SECONDARY };
        display.draw_text(value_x, y_start + line_height, &self.network_ssid, ssid_color, None, 1)?;
        
        // IP Address
        let ip_y = y_start + line_height * 2;
        display.fill_rect(value_x, ip_y, 200, 16, BLACK)?;
        
        if let Some(ref ip) = self.network_ip {
            display.draw_text(value_x, ip_y, ip, TEXT_PRIMARY, None, 1)?;
        } else if self.network_ssid.is_empty() || self.network_ssid == "Not connected" {
            // No WiFi credentials configured
            display.draw_text(value_x, ip_y, "No WiFi Config", YELLOW, None, 1)?;
        } else {
            // WiFi configured but no IP yet
            display.draw_text(value_x, ip_y, "Obtaining IP...", YELLOW, None, 1)?;
        }
        
        // Signal strength - just text, no graph
        let signal_y = y_start + line_height * 3;
        display.fill_rect(value_x, signal_y, 200, 16, BLACK)?;
        
        if self.network_connected {
            let signal_quality = match self.network_signal {
                -50..=0 => "Excellent",
                -60..=-51 => "Good",
                -70..=-61 => "Fair",
                -80..=-71 => "Weak",
                _ => "Poor"
            };
            
            let signal_color = match self.network_signal {
                -50..=0 => PRIMARY_GREEN,
                -60..=-51 => PRIMARY_GREEN,
                -70..=-61 => YELLOW,
                -80..=-71 => ACCENT_ORANGE,
                _ => PRIMARY_RED
            };
            
            display.draw_text(value_x, signal_y, &format!("{} dBm ({})", self.network_signal, signal_quality), signal_color, None, 1)?;
        } else {
            display.draw_text(value_x, signal_y, "No signal", TEXT_SECONDARY, None, 1)?;
        }
        
        // Additional network information
        if self.network_connected {
            let info_y = y_start + line_height * 4 + 5;
            
            // MAC Address
            display.draw_text(10, info_y, "MAC:", TEXT_PRIMARY, None, 1)?;
            display.fill_rect(value_x, info_y, 240, 16, BLACK)?;
            display.draw_text(value_x, info_y, &self.network_mac, TEXT_SECONDARY, None, 1)?;
            
            // Gateway
            display.draw_text(10, info_y + line_height, "Gateway:", TEXT_PRIMARY, None, 1)?;
            display.fill_rect(value_x, info_y + line_height, 240, 16, BLACK)?;
            if let Some(ref gateway) = self.network_gateway {
                display.draw_text(value_x, info_y + line_height, gateway, TEXT_SECONDARY, None, 1)?;
            } else {
                display.draw_text(value_x, info_y + line_height, "Not available", TEXT_SECONDARY, None, 1)?;
            }
            
            // Web interface section - ensure no overlap
            let web_section_y = info_y + line_height * 2 + 10; // Dynamic positioning
            display.draw_line(10, web_section_y - 5, 290, web_section_y - 5, BORDER_COLOR)?;
            
            display.draw_text_centered(web_section_y + 5, "Web Configuration", TEXT_SECONDARY, None, 1)?;
            if let Some(ref ip) = self.network_ip {
                display.draw_text_centered(web_section_y + 20, &format!("http://{}", ip), PRIMARY_BLUE, None, 1)?;
            }
        } else {
            // Not connected - show help
            let help_y = y_start + line_height * 4 + 10;
            display.draw_line(10, help_y - 5, 290, help_y - 5, BORDER_COLOR)?;
            
            if self.network_ssid.is_empty() || self.network_ssid == "Not connected" {
                // No WiFi credentials
                display.draw_text_centered(help_y + 10, "WiFi Not Configured", ACCENT_ORANGE, None, 1)?;
                display.draw_text_centered(help_y + 28, "Edit wifi_config.h:", TEXT_PRIMARY, None, 1)?;
                display.draw_text_centered(help_y + 42, "#define WIFI_SSID \"YourSSID\"", PRIMARY_BLUE, None, 1)?;
                display.draw_text_centered(help_y + 56, "#define WIFI_PASSWORD \"YourPass\"", PRIMARY_BLUE, None, 1)?;
                display.draw_text_centered(help_y + 74, "Then rebuild & flash", TEXT_SECONDARY, None, 1)?;
            } else {
                // WiFi configured but not connected
                display.draw_text_centered(help_y + 10, "WiFi Connection Failed", PRIMARY_RED, None, 1)?;
                display.draw_text_centered(help_y + 28, &format!("SSID: {}", self.network_ssid), TEXT_SECONDARY, None, 1)?;
                display.draw_text_centered(help_y + 42, "Check password & signal", TEXT_SECONDARY, None, 1)?;
                display.draw_text_centered(help_y + 65, "Retrying connection...", TEXT_SECONDARY, None, 1)?;
            }
        }
        
        // Mark screen as initialized
        self.network_screen_initialized = true;
        
        Ok(())
    }

    fn render_sensor_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        // Early exit if nothing needs updating
        static mut LAST_BATTERY: u8 = 255;
        static mut LAST_TEMP: f32 = -999.0;
        static mut LAST_LIGHT: u16 = 65535;
        
        unsafe {
            if !screen_changed && self.sensor_screen_initialized &&
               LAST_BATTERY == self.sensor_data._battery_percentage &&
               (LAST_TEMP - self.sensor_data._temperature).abs() < 0.5 &&
               LAST_LIGHT == self.sensor_data._light_level {
                return Ok(());
            }
            
            // Update cached values
            LAST_BATTERY = self.sensor_data._battery_percentage;
            LAST_TEMP = self.sensor_data._temperature;
            LAST_LIGHT = self.sensor_data._light_level;
        }
        
        // Only clear screen when switching to this screen
        if screen_changed {
            display.clear(BLACK)?;
            display.flush()?; // Flush immediately to clear old content
            
            // Header
            display.fill_rect(0, 0, 300, 30, PRIMARY_GREEN)?;
            display.draw_text_centered(8, "Sensor Data", WHITE, None, 2)?;
            
            // Reset initialization
            self.sensor_screen_initialized = false;
            
            // Static labels
            let y_start = 50;
            let line_height = 30;
            display.draw_text(10, y_start, "Battery:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height, "Temp:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 2, "Light:", TEXT_PRIMARY, None, 1)?;
            
            // Button hints (moved up to avoid overlap)
            display.draw_text(10, 150, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(230, 150, "[USER] Next", TEXT_SECONDARY, None, 1)?;
        }
        
        // Mark screen as initialized
        self.sensor_screen_initialized = true;
        
        // Dynamic sensor values (only update if changed)
        let y_start = 50;
        let line_height = 30;
        
        // Battery value and bar with voltage and charging status
        let battery_percent = self.sensor_data._battery_percentage;
        let battery_voltage = self.sensor_data._battery_voltage;
        let is_charging = self.sensor_data._is_charging;
        let is_on_usb = self.sensor_data._is_on_usb;
        
        // Color based on battery state
        let battery_color = if is_charging { PRIMARY_BLUE }
                           else if battery_percent > 50 { PRIMARY_GREEN }
                           else if battery_percent > 20 { YELLOW }
                           else { PRIMARY_RED };
        
        // Draw progress bar
        display.draw_progress_bar(100, y_start, 120, 15, battery_percent, battery_color, SURFACE_LIGHT, BORDER_COLOR)?;
        
        // Draw percentage and voltage
        display.fill_rect(225, y_start, 70, 16, BLACK)?;
        display.draw_text(225, y_start, &format!("{}%", battery_percent), battery_color, None, 1)?;
        
        // Draw voltage below percentage
        display.fill_rect(100, y_start + 18, 195, 14, BLACK)?;
        if battery_voltage > 0 {
            let voltage_str = format!("{:.2}V", battery_voltage as f32 / 1000.0);
            display.draw_text(100, y_start + 18, &voltage_str, TEXT_SECONDARY, None, 1)?;
            
            // Show charging/USB status
            if is_charging {
                display.draw_text(150, y_start + 18, "Charging", PRIMARY_BLUE, None, 1)?;
            } else if is_on_usb {
                display.draw_text(150, y_start + 18, "USB Power", ACCENT_ORANGE, None, 1)?;
            } else {
                display.draw_text(150, y_start + 18, "Battery", TEXT_SECONDARY, None, 1)?;
            }
        } else {
            display.draw_text(100, y_start + 18, "No Battery", TEXT_SECONDARY, None, 1)?;
        }
        
        // Temperature value (adjusted position due to battery info)
        let temp_y = y_start + line_height + 5;
        display.fill_rect(100, temp_y, 100, 20, BLACK)?;
        display.draw_text(100, temp_y, &format!("{:.1}°C", self.sensor_data._temperature), TEXT_PRIMARY, None, 1)?;
        
        // Light level value (adjusted position)
        let light_y = y_start + line_height * 2 + 5;
        display.fill_rect(100, light_y, 100, 20, BLACK)?;
        if self.sensor_data._light_level > 0 {
            display.draw_text(100, light_y, &format!("{} lux", self.sensor_data._light_level), TEXT_PRIMARY, None, 1)?;
        } else {
            display.draw_text(100, light_y, "N/A", TEXT_SECONDARY, None, 1)?;
        }
        
        // Visual indicator
        let radius = 20;
        let cx = 160;
        let cy = 130;
        display.draw_circle(cx, cy, radius, BORDER_COLOR)?;
        let fill_radius = (radius as f32 * self.animation_progress) as u16;
        if fill_radius > 0 {
            display.fill_circle(cx, cy, fill_radius, PRIMARY_GREEN)?;
        }
        
        Ok(())
    }

    fn render_settings_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        // Early exit - settings screen is mostly static
        if !screen_changed && self.settings_screen_initialized {
            return Ok(());
        }
        
        if screen_changed {
            // Clear screen only on screen change
            display.clear(BLACK)?;
            display.flush()?; // Flush immediately to clear old content
            
            // Header
            display.fill_rect(0, 0, 300, 30, ACCENT_ORANGE)?;
            display.draw_text_centered(8, "Settings", WHITE, None, 2)?;
            
            // Reset initialization
            self.settings_screen_initialized = false;
            
            // Settings options
            let y_start = 50;
            let line_height = 30;
            
            // Static labels
            display.draw_text(10, y_start, "Brightness:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height, "Auto-dim:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 2, "Update:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(10, y_start + line_height * 3, "Version:", TEXT_PRIMARY, None, 1)?;
            
            // Button hints (moved up to avoid overlap)
            display.draw_text(10, 150, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(200, 150, "[USER] Select", TEXT_SECONDARY, None, 1)?;
        }
        
        // Dynamic values (always update)
        let y_start = 50;
        let line_height = 30;
        
        // Brightness bar and value
        display.draw_progress_bar(120, y_start, 100, 15, 80, PRIMARY_BLUE, SURFACE_LIGHT, BORDER_COLOR)?;
        display.draw_text(230, y_start, "80%", TEXT_PRIMARY, None, 1)?;
        
        // Auto-dim status
        display.fill_rect(120, y_start + line_height, 60, 20, BLACK)?;
        display.draw_text(120, y_start + line_height, "ON", PRIMARY_GREEN, None, 1)?;
        
        // Update speed
        display.fill_rect(120, y_start + line_height * 2, 80, 20, BLACK)?;
        display.draw_text(120, y_start + line_height * 2, "Normal", TEXT_PRIMARY, None, 1)?;
        
        // Version
        display.fill_rect(120, y_start + line_height * 3, 100, 20, BLACK)?;
        display.draw_text(120, y_start + line_height * 3, crate::version::DISPLAY_VERSION, TEXT_SECONDARY, None, 1)?;
        
        // Mark screen as initialized
        self.settings_screen_initialized = true;
        
        Ok(())
    }
    
    fn render_ota_screen(&mut self, display: &mut DisplayManager, screen_changed: bool) -> Result<()> {
        // Early exit if nothing needs updating
        if !screen_changed && 
           self.cached_ota_status_enum == Some(self.ota_status.clone()) &&
           self.cached_network_ip == self.network_ip &&
           self.ota_screen_initialized {
            // Only update time every 5 seconds to reduce operations
            let current_seconds = self.system_info.get_uptime().as_secs();
            if current_seconds >= self.cached_ota_time + 5 {
                self.cached_ota_time = current_seconds;
                display.fill_rect(240, 5, 60, 20, ACCENT_ORANGE)?;
                let time_str = self.system_info.format_uptime();
                display.draw_text(245, 8, &time_str, WHITE, None, 1)?;
            }
            return Ok(());
        }
        
        if screen_changed {
            // Clear screen
            display.clear(BLACK)?;
            display.flush()?;
            
            // Header
            display.fill_rect(0, 0, 300, 30, ACCENT_ORANGE)?;
            display.draw_text_centered(8, "OTA Updates", WHITE, None, 2)?;
            
            // Button hints
            display.draw_text(10, 155, "[BOOT] Prev", TEXT_SECONDARY, None, 1)?;
            display.draw_text(200, 155, "[USER] Check", TEXT_SECONDARY, None, 1)?;
            
            // Reset initialization flag
            self.ota_screen_initialized = false;
        }
        
        // Update time on first render
        if !self.ota_screen_initialized {
            let current_seconds = self.system_info.get_uptime().as_secs();
            self.cached_ota_time = current_seconds;
            display.fill_rect(240, 5, 60, 20, ACCENT_ORANGE)?;
            let time_str = self.system_info.format_uptime();
            display.draw_text(245, 8, &time_str, WHITE, None, 1)?;
        }
        
        // Main content area - adjusted spacing
        let y_start = 36;
        let line_height = 16;
        
        // Static content - only draw once
        if !self.ota_screen_initialized || screen_changed {
            // Current version info
            display.draw_text(10, y_start, "Firmware:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(80, y_start, crate::version::DISPLAY_VERSION, PRIMARY_BLUE, None, 1)?;
            
            // Partition info
            display.draw_text(180, y_start, "Partition:", TEXT_PRIMARY, None, 1)?;
            display.draw_text(240, y_start, "Factory", TEXT_SECONDARY, None, 1)?;
            
            // Status label
            display.draw_text(10, y_start + line_height, "Status:", TEXT_PRIMARY, None, 1)?;
            
            // Draw separator line once
            let base_server_y = y_start + line_height * 2 + 8;
            display.draw_line(10, base_server_y - 3, 290, base_server_y - 3, BORDER_COLOR)?;
        }
        
        // OTA Status - only format and update if truly changed
        let status_changed = self.cached_ota_status_enum != Some(self.ota_status.clone());
        if status_changed || !self.ota_screen_initialized {
            self.cached_ota_status_enum = Some(self.ota_status.clone());
            
            let (status_text, status_color) = match &self.ota_status {
                OtaStatus::Idle => ("Ready", TEXT_SECONDARY),
                OtaStatus::Downloading { progress } => {
                    self.string_buffer.clear();
                    use std::fmt::Write;
                    let _ = write!(&mut self.string_buffer, "Downloading {}%", progress);
                    (self.string_buffer.as_str(), PRIMARY_BLUE)
                },
                OtaStatus::Verifying => ("Verifying Update", YELLOW),
                OtaStatus::Ready => ("Update Ready - Restart", PRIMARY_GREEN),
                OtaStatus::Failed => ("Update Failed", PRIMARY_RED),
            };
            
            display.fill_rect(80, y_start + line_height, 200, 16, BLACK)?;
            display.draw_text(80, y_start + line_height, status_text, status_color, None, 1)?;
        }
        
        // Progress bar - use fixed position to leverage caching
        let progress_y = y_start + line_height * 2 + 4;
        if let OtaStatus::Downloading { progress } = self.ota_status {
            display.draw_progress_bar(10, progress_y, 280, 10, progress, PRIMARY_BLUE, SURFACE_LIGHT, BORDER_COLOR)?;
        } else if status_changed && matches!(self.cached_ota_status_enum, Some(OtaStatus::Downloading { .. })) {
            // Clear progress bar area when transitioning away from downloading
            display.fill_rect(10, progress_y, 280, 10, BLACK)?;
        }
        
        // Fixed server section position
        let server_section_y = y_start + line_height * 2 + 24;
        
        // Network section - only update on actual changes
        let network_changed = self.cached_network_ip != self.network_ip;
        if network_changed || !self.ota_screen_initialized {
            // Clear the entire network section area
            display.fill_rect(10, server_section_y, 290, 80, BLACK)?;
            
            if self.network_connected {
                display.draw_text_centered(server_section_y + 4, "OTA Endpoints", TEXT_SECONDARY, None, 1)?;
                
                if let Some(ref ip) = self.network_ip {
                    self.cached_network_ip = Some(ip.clone());
                    
                    // Format endpoints using pre-allocated buffer
                    let endpoint_y = server_section_y + 20;
                    display.draw_text(10, endpoint_y, "Upload:", TEXT_PRIMARY, None, 1)?;
                    
                    self.string_buffer.clear();
                    use std::fmt::Write;
                    let _ = write!(&mut self.string_buffer, "http://{}:8080/ota", ip);
                    display.draw_text(60, endpoint_y, &self.string_buffer, PRIMARY_BLUE, None, 1)?;
                    
                    let status_y = endpoint_y + 16;
                    display.draw_text(10, status_y, "Status:", TEXT_PRIMARY, None, 1)?;
                    
                    self.string_buffer.clear();
                    let _ = write!(&mut self.string_buffer, "http://{}:8080/api/ota/status", ip);
                    display.draw_text(60, status_y, &self.string_buffer, PRIMARY_BLUE, None, 1)?;
                    
                    let guide_y = status_y + 20;
                    display.draw_text_centered(guide_y, "Upload .bin file at OTA URL", TEXT_SECONDARY, None, 1)?;
                    display.draw_text_centered(guide_y + 14, "Device auto-restarts after update", TEXT_SECONDARY, None, 1)?;
                } else {
                    self.cached_network_ip = None;
                }
            } else {
                self.cached_network_ip = None;
                display.draw_text_centered(server_section_y + 8, "Network Required", PRIMARY_RED, None, 1)?;
                display.draw_text_centered(server_section_y + 24, "Connect to WiFi to enable OTA", TEXT_SECONDARY, None, 1)?;
            }
        }
        
        // Mark screen as initialized
        self.ota_screen_initialized = true;
        
        Ok(())
    }
    
    fn render_fps_counter(&mut self, display: &mut DisplayManager) -> Result<()> {
        // Only update FPS counter if it changed significantly
        static mut LAST_FPS: f32 = -1.0; // Initialize to -1 to force first render
        
        unsafe {
            if LAST_FPS >= 0.0 && (self.fps - LAST_FPS).abs() < 0.5 {
                return Ok(()); // Skip update if change is less than 0.5 FPS
            }
            LAST_FPS = self.fps;
        }
        
        // Draw FPS in top-right corner, below header
        let fps_text = format!("{:.1} FPS", self.fps);
        let x = 245;
        let y = 32;
        
        // Clear background for FPS counter
        display.fill_rect(x, y, 55, 12, BLACK)?;
        
        // Draw FPS text with smaller font
        let color = if self.fps >= 15.0 { PRIMARY_GREEN } 
                    else if self.fps >= 10.0 { YELLOW } 
                    else { PRIMARY_RED };
        display.draw_text(x, y, &fps_text, color, None, 1)?;
        
        Ok(())
    }
    
    fn render_ota_overlay(&mut self, display: &mut DisplayManager, progress: u8) -> Result<()> {
        // Draw semi-transparent overlay
        let overlay_y = 50;
        let overlay_height = 80;
        
        // Draw background box with border
        display.fill_rect(20, overlay_y, 260, overlay_height, SURFACE_DARK)?;
        display.draw_rect(20, overlay_y, 260, overlay_height, ACCENT_ORANGE)?;
        
        // OTA Update title
        display.draw_text_centered(overlay_y + 10, "OTA UPDATE IN PROGRESS", ACCENT_ORANGE, None, 2)?;
        
        // Progress bar
        let bar_y = overlay_y + 35;
        display.draw_progress_bar(40, bar_y, 220, 20, progress, PRIMARY_BLUE, SURFACE_LIGHT, WHITE)?;
        
        // Progress text
        let progress_text = format!("{}%", progress);
        display.draw_text_centered(bar_y + 25, &progress_text, WHITE, None, 1)?;
        
        // Warning text
        display.draw_text_centered(bar_y + 40, "DO NOT POWER OFF", PRIMARY_RED, None, 1)?;
        
        Ok(())
    }
}