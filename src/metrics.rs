// Global metrics collection for the dashboard
use std::sync::{Arc, Mutex};

// Global metrics instance - use a static with manual initialization
static mut METRICS: Option<Arc<Mutex<MetricsData>>> = None;
static METRICS_INIT: std::sync::Once = std::sync::Once::new();

// Initialize metrics
pub fn init_metrics() {
    unsafe {
        METRICS_INIT.call_once(|| {
            METRICS = Some(Arc::new(Mutex::new(MetricsData::default())));
        });
    }
}

// Get the metrics instance - panics if not initialized
pub fn metrics() -> &'static Arc<Mutex<MetricsData>> {
    unsafe {
        METRICS.as_ref().expect("Metrics not initialized! Call init_metrics() first")
    }
}

#[derive(Default, Clone)]
pub struct MetricsData {
    // CPU metrics
    pub cpu_usage: u8,
    pub cpu_freq_mhz: u16,
    pub cpu0_usage: u8,
    pub cpu1_usage: u8,
    
    // Temperature
    pub temperature: f32,
    
    // WiFi
    pub wifi_rssi: i8,
    pub wifi_connected: bool,
    pub wifi_ssid: String,
    
    // Display
    pub display_brightness: u8,
    
    // Performance
    pub fps_actual: f32,
    pub fps_target: f32,
    pub render_time_ms: u32,
    pub flush_time_ms: u32,
    
    // Battery
    pub battery_voltage_mv: u16,
    pub battery_percentage: u8,
    pub battery_charging: bool,
    
    // Frame statistics
    pub frame_count: u64,
    pub skip_count: u64,
    
    // PSRAM
    pub psram_free: u32,
    pub psram_total: u32,
    
    // Button metrics
    pub button_avg_response_ms: f32,
    pub button_max_response_ms: f32,
    pub button_events_total: u64,
    pub button_events_per_second: f32,
}

impl MetricsData {
    pub fn update_cpu(&mut self, usage: u8, freq_mhz: u16) {
        self.cpu_usage = usage;
        self.cpu_freq_mhz = freq_mhz;
    }
    
    pub fn update_cpu_cores(&mut self, cpu0: u8, cpu1: u8) {
        self.cpu0_usage = cpu0;
        self.cpu1_usage = cpu1;
        // Also update overall CPU usage as average
        self.cpu_usage = (cpu0 + cpu1) / 2;
    }
    
    pub fn update_temperature(&mut self, temp: f32) {
        self.temperature = temp;
    }
    
    pub fn update_wifi_signal(&mut self, rssi: i8) {
        self.wifi_rssi = rssi;
    }
    
    pub fn update_wifi_status(&mut self, connected: bool, ssid: String) {
        self.wifi_connected = connected;
        self.wifi_ssid = ssid;
    }
    
    pub fn update_display(&mut self, brightness: u8) {
        self.display_brightness = brightness;
    }
    
    pub fn update_battery(&mut self, voltage_mv: u16, percentage: u8, is_charging: bool) {
        self.battery_voltage_mv = voltage_mv;
        self.battery_percentage = percentage;
        self.battery_charging = is_charging;
    }
    
    pub fn update_timings(&mut self, render_ms: u32, flush_ms: u32) {
        self.render_time_ms = render_ms;
        self.flush_time_ms = flush_ms;
    }
    
    pub fn update_frame_stats(&mut self, total: u64, skipped: u64) {
        self.frame_count = total;
        self.skip_count = skipped;
    }
    
    pub fn update_psram(&mut self, free: u32, total: u32) {
        self.psram_free = free;
        self.psram_total = total;
    }
    
    pub fn update_button_metrics(&mut self, avg_ms: f32, max_ms: f32, total_events: u64, events_per_sec: f32) {
        self.button_avg_response_ms = avg_ms;
        self.button_max_response_ms = max_ms;
        self.button_events_total = total_events;
        self.button_events_per_second = events_per_sec;
    }
    
    pub fn update_fps(&mut self, actual: f32, target: f32) {
        self.fps_actual = actual;
        self.fps_target = target;
    }
}