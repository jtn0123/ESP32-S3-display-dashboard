use std::sync::{Arc, Mutex};
use static_cell::StaticCell;

// Global metrics storage using static_cell - won't initialize until we explicitly call init()
static METRICS_CELL: StaticCell<Arc<Mutex<MetricsData>>> = StaticCell::new();
static mut METRICS_REF: Option<&'static Arc<Mutex<MetricsData>>> = None;

// Initialize the metrics system - call this AFTER display is initialized
pub fn init_metrics() {
    let metrics_ref = METRICS_CELL.init(Arc::new(Mutex::new(MetricsData::default())));
    unsafe {
        METRICS_REF = Some(metrics_ref);
    }
}

// Get the metrics instance - panics if not initialized
pub fn metrics() -> &'static Arc<Mutex<MetricsData>> {
    unsafe {
        METRICS_REF.expect("Metrics not initialized! Call init_metrics() first")
    }
}

#[derive(Default, Clone)]
pub struct MetricsData {
    pub fps_actual: f32,
    pub fps_target: f32,
    pub cpu_usage_percent: u8,
    pub cpu_freq_mhz: u16,
    pub cpu0_usage_percent: u8,
    pub cpu1_usage_percent: u8,
    pub temperature_celsius: f32,
    pub wifi_rssi_dbm: i8,
    pub display_brightness: u8,
    pub battery_voltage_mv: u16,
    pub battery_percentage: u8,
    pub is_charging: bool,
    pub render_time_ms: u32,
    pub flush_time_ms: u32,
    pub frame_count: u64,
    pub skip_count: u64,
}

impl MetricsData {
    pub fn update_fps(&mut self, actual: f32, target: f32) {
        self.fps_actual = actual;
        self.fps_target = target;
    }
    
    pub fn update_cpu(&mut self, usage: u8, freq_mhz: u16) {
        self.cpu_usage_percent = usage;
        self.cpu_freq_mhz = freq_mhz;
    }
    
    pub fn update_cpu_cores(&mut self, cpu0: u8, cpu1: u8) {
        self.cpu0_usage_percent = cpu0;
        self.cpu1_usage_percent = cpu1;
        self.cpu_usage_percent = (cpu0 + cpu1) / 2; // Keep average for backward compatibility
    }
    
    pub fn update_temperature(&mut self, temp: f32) {
        self.temperature_celsius = temp;
    }
    
    pub fn update_wifi_signal(&mut self, rssi: i8) {
        self.wifi_rssi_dbm = rssi;
    }
    
    pub fn update_display(&mut self, brightness: u8) {
        self.display_brightness = brightness;
    }
    
    pub fn update_battery(&mut self, voltage_mv: u16, percentage: u8, is_charging: bool) {
        self.battery_voltage_mv = voltage_mv;
        self.battery_percentage = percentage;
        self.is_charging = is_charging;
    }
    
    pub fn update_timings(&mut self, render_ms: u32, flush_ms: u32) {
        self.render_time_ms = render_ms;
        self.flush_time_ms = flush_ms;
    }
    
    pub fn update_frame_stats(&mut self, total: u64, skipped: u64) {
        self.frame_count = total;
        self.skip_count = skipped;
    }
    
    pub fn format_prometheus(&self, uptime_seconds: u64, heap_free: u32, heap_total: u32) -> String {
        // Calculate skip rate percentage
        let skip_rate = if self.frame_count > 0 {
            (self.skip_count as f32 / self.frame_count as f32 * 100.0) as u8
        } else {
            0
        };
        
        format!(
            "# HELP esp32_uptime_seconds Total uptime in seconds\n\
             # TYPE esp32_uptime_seconds counter\n\
             esp32_uptime_seconds {}\n\
             \n\
             # HELP esp32_heap_free_bytes Current free heap memory in bytes\n\
             # TYPE esp32_heap_free_bytes gauge\n\
             esp32_heap_free_bytes {}\n\
             \n\
             # HELP esp32_heap_total_bytes Total heap memory in bytes\n\
             # TYPE esp32_heap_total_bytes gauge\n\
             esp32_heap_total_bytes {}\n\
             \n\
             # HELP esp32_fps_actual Current actual frames per second\n\
             # TYPE esp32_fps_actual gauge\n\
             esp32_fps_actual {:.1}\n\
             \n\
             # HELP esp32_fps_target Target frames per second\n\
             # TYPE esp32_fps_target gauge\n\
             esp32_fps_target {:.1}\n\
             \n\
             # HELP esp32_cpu_usage_percent CPU usage percentage\n\
             # TYPE esp32_cpu_usage_percent gauge\n\
             esp32_cpu_usage_percent {}\n\
             \n\
             # HELP esp32_cpu_freq_mhz CPU frequency in MHz\n\
             # TYPE esp32_cpu_freq_mhz gauge\n\
             esp32_cpu_freq_mhz {}\n\
             \n\
             # HELP esp32_temperature_celsius Internal temperature in Celsius\n\
             # TYPE esp32_temperature_celsius gauge\n\
             esp32_temperature_celsius {:.1}\n\
             \n\
             # HELP esp32_wifi_rssi_dbm WiFi signal strength in dBm\n\
             # TYPE esp32_wifi_rssi_dbm gauge\n\
             esp32_wifi_rssi_dbm {}\n\
             \n\
             # HELP esp32_display_brightness Display brightness level\n\
             # TYPE esp32_display_brightness gauge\n\
             esp32_display_brightness {}\n\
             \n\
             # HELP esp32_render_time_milliseconds Display render time in milliseconds\n\
             # TYPE esp32_render_time_milliseconds gauge\n\
             esp32_render_time_milliseconds {}\n\
             \n\
             # HELP esp32_flush_time_milliseconds Display flush time in milliseconds\n\
             # TYPE esp32_flush_time_milliseconds gauge\n\
             esp32_flush_time_milliseconds {}\n\
             \n\
             # HELP esp32_frame_skip_rate_percent Percentage of frames skipped\n\
             # TYPE esp32_frame_skip_rate_percent gauge\n\
             esp32_frame_skip_rate_percent {}\n\
             \n\
             # HELP esp32_total_frames_count Total number of frames processed\n\
             # TYPE esp32_total_frames_count counter\n\
             esp32_total_frames_count {}\n\
             \n\
             # HELP esp32_skipped_frames_count Number of frames skipped\n\
             # TYPE esp32_skipped_frames_count counter\n\
             esp32_skipped_frames_count {}",
            uptime_seconds,
            heap_free,
            heap_total,
            self.fps_actual,
            self.fps_target,
            self.cpu_usage_percent,
            self.cpu_freq_mhz,
            self.temperature_celsius,
            self.wifi_rssi_dbm,
            self.display_brightness,
            self.render_time_ms,
            self.flush_time_ms,
            skip_rate,
            self.frame_count,
            self.skip_count
        )
    }
}