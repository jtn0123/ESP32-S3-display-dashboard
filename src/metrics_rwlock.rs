// Optimized metrics collection with RwLock for reduced contention
use std::sync::{Arc, RwLock, OnceLock};
use std::sync::atomic::{AtomicU32, AtomicU16, AtomicU8, AtomicBool, Ordering};

// Global metrics instance - use OnceLock for safe one-time initialization
static METRICS: OnceLock<Arc<MetricsStore>> = OnceLock::new();

// Initialize metrics
pub fn init_metrics() {
    METRICS.get_or_init(|| Arc::new(MetricsStore::new()));
}

// Get the metrics instance - panics if not initialized
pub fn metrics() -> &'static Arc<MetricsStore> {
    METRICS.get().expect("Metrics not initialized! Call init_metrics() first")
}

/// Optimized metrics storage with atomic types and RwLock
pub struct MetricsStore {
    // Atomic types for lock-free updates (when possible)
    cpu_usage: AtomicU8,
    cpu_freq_mhz: AtomicU16,
    cpu0_usage: AtomicU8,
    cpu1_usage: AtomicU8,
    
    // WiFi (atomic where possible)
    wifi_rssi: AtomicI8,
    wifi_connected: AtomicBool,
    
    // Display
    display_brightness: AtomicU8,
    
    // Battery
    battery_voltage_mv: AtomicU16,
    battery_percentage: AtomicU8,
    battery_charging: AtomicBool,
    
    // Performance counters (using u32 for compatibility)
    frame_count: AtomicU32,
    skip_count: AtomicU32,
    
    // PSRAM
    psram_free: AtomicU32,
    psram_total: AtomicU32,
    
    // Button metrics
    button_events_total: AtomicU32,
    
    // Connection monitoring
    http_connections_active: AtomicU32,
    http_connections_total: AtomicU32,
    telnet_connections_active: AtomicU32,
    telnet_connections_total: AtomicU32,
    wifi_disconnects: AtomicU32,
    wifi_reconnects: AtomicU32,
    uptime_seconds: AtomicU32,
    
    // Complex data that requires locking
    complex_data: RwLock<ComplexMetrics>,
}

#[derive(Clone)]
pub struct ComplexMetrics {
    // Temperature (f32 can't be atomic)
    pub temperature: f32,
    
    // WiFi SSID (String requires locking)
    pub wifi_ssid: String,
    
    // Performance metrics (f32)
    pub fps_actual: f32,
    pub fps_target: f32,
    pub render_time_ms: u32,
    pub flush_time_ms: u32,
    
    // Button metrics (f32)
    pub button_avg_response_ms: f32,
    pub button_max_response_ms: f32,
    pub button_events_per_second: f32,
}

impl Default for ComplexMetrics {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            wifi_ssid: String::new(),
            fps_actual: 0.0,
            fps_target: 30.0,
            render_time_ms: 0,
            flush_time_ms: 0,
            button_avg_response_ms: 0.0,
            button_max_response_ms: 0.0,
            button_events_per_second: 0.0,
        }
    }
}

// Custom atomic for i8 since std doesn't provide AtomicI8
struct AtomicI8(AtomicU8);

impl AtomicI8 {
    fn new(val: i8) -> Self {
        Self(AtomicU8::new(val as u8))
    }
    
    fn store(&self, val: i8, ordering: Ordering) {
        self.0.store(val as u8, ordering);
    }
    
    fn load(&self, ordering: Ordering) -> i8 {
        self.0.load(ordering) as i8
    }
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            cpu_usage: AtomicU8::new(0),
            cpu_freq_mhz: AtomicU16::new(240),
            cpu0_usage: AtomicU8::new(0),
            cpu1_usage: AtomicU8::new(0),
            wifi_rssi: AtomicI8::new(0),
            wifi_connected: AtomicBool::new(false),
            display_brightness: AtomicU8::new(255),
            battery_voltage_mv: AtomicU16::new(0),
            battery_percentage: AtomicU8::new(0),
            battery_charging: AtomicBool::new(false),
            frame_count: AtomicU32::new(0),
            skip_count: AtomicU32::new(0),
            psram_free: AtomicU32::new(0),
            psram_total: AtomicU32::new(0),
            button_events_total: AtomicU32::new(0),
            http_connections_active: AtomicU32::new(0),
            http_connections_total: AtomicU32::new(0),
            telnet_connections_active: AtomicU32::new(0),
            telnet_connections_total: AtomicU32::new(0),
            wifi_disconnects: AtomicU32::new(0),
            wifi_reconnects: AtomicU32::new(0),
            uptime_seconds: AtomicU32::new(0),
            complex_data: RwLock::new(ComplexMetrics::default()),
        }
    }
    
    // Fast atomic updates (no locking required)
    pub fn update_cpu(&self, usage: u8, freq_mhz: u16) {
        self.cpu_usage.store(usage, Ordering::Relaxed);
        self.cpu_freq_mhz.store(freq_mhz, Ordering::Relaxed);
    }
    
    pub fn update_cpu_cores(&self, cpu0: u8, cpu1: u8) {
        self.cpu0_usage.store(cpu0, Ordering::Relaxed);
        self.cpu1_usage.store(cpu1, Ordering::Relaxed);
        // Also update overall CPU usage as average
        self.cpu_usage.store((cpu0 + cpu1) / 2, Ordering::Relaxed);
    }
    
    pub fn update_wifi_signal(&self, rssi: i8) {
        self.wifi_rssi.store(rssi, Ordering::Relaxed);
    }
    
    pub fn update_display(&self, brightness: u8) {
        self.display_brightness.store(brightness, Ordering::Relaxed);
    }
    
    pub fn update_battery(&self, voltage_mv: u16, percentage: u8, is_charging: bool) {
        self.battery_voltage_mv.store(voltage_mv, Ordering::Relaxed);
        self.battery_percentage.store(percentage, Ordering::Relaxed);
        self.battery_charging.store(is_charging, Ordering::Relaxed);
    }
    
    pub fn update_frame_stats(&self, total: u64, skipped: u64) {
        self.frame_count.store(total as u32, Ordering::Relaxed);
        self.skip_count.store(skipped as u32, Ordering::Relaxed);
    }
    
    pub fn update_psram(&self, free: u32, total: u32) {
        self.psram_free.store(free, Ordering::Relaxed);
        self.psram_total.store(total, Ordering::Relaxed);
    }
    
    // Complex updates that require write lock
    pub fn update_temperature(&self, temp: f32) {
        if let Ok(mut data) = self.complex_data.write() {
            data.temperature = temp;
        }
    }
    
    pub fn update_wifi_status(&self, connected: bool, ssid: String) {
        self.wifi_connected.store(connected, Ordering::Relaxed);
        if let Ok(mut data) = self.complex_data.write() {
            data.wifi_ssid = ssid;
        }
    }
    
    pub fn update_timings(&self, render_ms: u32, flush_ms: u32) {
        if let Ok(mut data) = self.complex_data.write() {
            data.render_time_ms = render_ms;
            data.flush_time_ms = flush_ms;
        }
    }
    
    pub fn update_fps(&self, actual: f32, target: f32) {
        if let Ok(mut data) = self.complex_data.write() {
            data.fps_actual = actual;
            data.fps_target = target;
        }
    }
    
    pub fn update_button_metrics(&self, avg_ms: f32, max_ms: f32, total_events: u64, events_per_sec: f32) {
        self.button_events_total.store(total_events as u32, Ordering::Relaxed);
        if let Ok(mut data) = self.complex_data.write() {
            data.button_avg_response_ms = avg_ms;
            data.button_max_response_ms = max_ms;
            data.button_events_per_second = events_per_sec;
        }
    }
    
    
    pub fn update_telnet_connections(&self, active: u32, total: u64) {
        self.telnet_connections_active.store(active, Ordering::Relaxed);
        self.telnet_connections_total.store(total as u32, Ordering::Relaxed);
    }
    
    pub fn update_wifi_reconnects(&self, disconnects: u32, reconnects: u32) {
        self.wifi_disconnects.store(disconnects, Ordering::Relaxed);
        self.wifi_reconnects.store(reconnects, Ordering::Relaxed);
    }
    
    pub fn update_uptime(&self, seconds: u64) {
        self.uptime_seconds.store(seconds as u32, Ordering::Relaxed);
    }
    
    /// Get a snapshot of all metrics for export
    pub fn snapshot(&self) -> crate::metrics::MetricsData {
        let complex = self.complex_data.read().unwrap_or_else(|e| e.into_inner()).clone();
        
        crate::metrics::MetricsData {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            heap_free: unsafe { esp_idf_sys::esp_get_free_heap_size() },
            cpu_usage: self.cpu_usage.load(Ordering::Relaxed),
            cpu_freq_mhz: self.cpu_freq_mhz.load(Ordering::Relaxed),
            cpu0_usage: self.cpu0_usage.load(Ordering::Relaxed),
            cpu1_usage: self.cpu1_usage.load(Ordering::Relaxed),
            temperature: complex.temperature,
            wifi_rssi: self.wifi_rssi.load(Ordering::Relaxed),
            wifi_connected: self.wifi_connected.load(Ordering::Relaxed),
            wifi_ssid: complex.wifi_ssid,
            display_brightness: self.display_brightness.load(Ordering::Relaxed),
            fps_actual: complex.fps_actual,
            fps_target: complex.fps_target,
            render_time_ms: complex.render_time_ms,
            flush_time_ms: complex.flush_time_ms,
            battery_voltage_mv: self.battery_voltage_mv.load(Ordering::Relaxed),
            battery_percentage: self.battery_percentage.load(Ordering::Relaxed),
            battery_charging: self.battery_charging.load(Ordering::Relaxed),
            frame_count: self.frame_count.load(Ordering::Relaxed) as u64,
            skip_count: self.skip_count.load(Ordering::Relaxed) as u64,
            psram_free: self.psram_free.load(Ordering::Relaxed),
            psram_total: self.psram_total.load(Ordering::Relaxed),
            button_avg_response_ms: complex.button_avg_response_ms,
            button_max_response_ms: complex.button_max_response_ms,
            button_events_total: self.button_events_total.load(Ordering::Relaxed) as u64,
            button_events_per_second: complex.button_events_per_second,
            http_connections_active: self.http_connections_active.load(Ordering::Relaxed),
            http_connections_total: self.http_connections_total.load(Ordering::Relaxed) as u64,
            telnet_connections_active: self.telnet_connections_active.load(Ordering::Relaxed),
            telnet_connections_total: self.telnet_connections_total.load(Ordering::Relaxed) as u64,
            wifi_disconnects: self.wifi_disconnects.load(Ordering::Relaxed),
            wifi_reconnects: self.wifi_reconnects.load(Ordering::Relaxed),
            uptime_seconds: self.uptime_seconds.load(Ordering::Relaxed) as u64,
        }
    }
}