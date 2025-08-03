// Global metrics collection for the dashboard
use std::sync::{Arc, OnceLock};

// Import the optimized store
use crate::metrics_rwlock::{self, MetricsStore};

// Global metrics instance - use OnceLock for safe one-time initialization
static METRICS: OnceLock<Arc<MetricsWrapper>> = OnceLock::new();

// Initialize metrics
pub fn init_metrics() {
    metrics_rwlock::init_metrics();
    METRICS.get_or_init(|| Arc::new(MetricsWrapper::new()));
}

// Get the metrics instance - panics if not initialized
pub fn metrics() -> &'static Arc<MetricsWrapper> {
    METRICS.get().expect("Metrics not initialized! Call init_metrics() first")
}

/// Wrapper that provides Mutex-like interface but uses optimized backend
pub struct MetricsWrapper {
    store: &'static Arc<MetricsStore>,
}

impl MetricsWrapper {
    fn new() -> Self {
        Self {
            store: metrics_rwlock::metrics(),
        }
    }
    
    /// Lock for reading - returns a snapshot
    pub fn lock(&self) -> Result<MetricsGuard, std::sync::PoisonError<MetricsGuard>> {
        Ok(MetricsGuard {
            data: self.store.snapshot(),
            store: self.store,
        })
    }
    
    /// Try to lock for reading - returns a snapshot
    pub fn try_lock(&self) -> Result<MetricsGuard, std::sync::TryLockError<MetricsGuard>> {
        Ok(MetricsGuard {
            data: self.store.snapshot(),
            store: self.store,
        })
    }
}

/// Guard that provides mutable access to metrics
pub struct MetricsGuard {
    data: MetricsData,
    store: &'static Arc<MetricsStore>,
}

impl std::ops::Deref for MetricsGuard {
    type Target = MetricsData;
    
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for MetricsGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Drop for MetricsGuard {
    fn drop(&mut self) {
        // Update the store with any changes made to the data
        self.store.update_cpu(self.data.cpu_usage, self.data.cpu_freq_mhz);
        self.store.update_cpu_cores(self.data.cpu0_usage, self.data.cpu1_usage);
        self.store.update_temperature(self.data.temperature);
        self.store.update_wifi_signal(self.data.wifi_rssi);
        self.store.update_wifi_status(self.data.wifi_connected, self.data.wifi_ssid.clone());
        self.store.update_display(self.data.display_brightness);
        self.store.update_battery(self.data.battery_voltage_mv, self.data.battery_percentage, self.data.battery_charging);
        self.store.update_timings(self.data.render_time_ms, self.data.flush_time_ms);
        self.store.update_frame_stats(self.data.frame_count, self.data.skip_count);
        self.store.update_psram(self.data.psram_free, self.data.psram_total);
        self.store.update_button_metrics(
            self.data.button_avg_response_ms,
            self.data.button_max_response_ms,
            self.data.button_events_total,
            self.data.button_events_per_second,
        );
        self.store.update_fps(self.data.fps_actual, self.data.fps_target);
        self.store.update_http_connections(self.data.http_connections_active, self.data.http_connections_total);
        self.store.update_telnet_connections(self.data.telnet_connections_active, self.data.telnet_connections_total);
        self.store.update_wifi_reconnects(self.data.wifi_disconnects, self.data.wifi_reconnects);
        self.store.update_uptime(self.data.uptime_seconds);
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsData {
    // Timestamp for the metrics
    pub timestamp: u64,
    // Heap memory
    pub heap_free: u32,
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
    
    // Connection monitoring
    pub http_connections_active: u32,
    pub http_connections_total: u64,
    pub telnet_connections_active: u32,
    pub telnet_connections_total: u64,
    pub wifi_disconnects: u32,
    pub wifi_reconnects: u32,
    pub uptime_seconds: u64,
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
    
    pub fn update_http_connections(&mut self, active: u32, total: u64) {
        self.http_connections_active = active;
        self.http_connections_total = total;
    }
    
    pub fn update_telnet_connections(&mut self, active: u32, total: u64) {
        self.telnet_connections_active = active;
        self.telnet_connections_total = total;
    }
    
    pub fn update_wifi_reconnects(&mut self, disconnects: u32, reconnects: u32) {
        self.wifi_disconnects = disconnects;
        self.wifi_reconnects = reconnects;
    }
    
    pub fn update_uptime(&mut self, seconds: u64) {
        self.uptime_seconds = seconds;
    }
}