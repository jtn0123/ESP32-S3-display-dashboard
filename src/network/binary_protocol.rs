
/// Binary metrics packet format for efficient transmission
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct MetricsBinaryPacket {
    pub version: u8,           // Protocol version
    pub timestamp: u64,        // Unix timestamp in seconds
    pub temperature: i16,      // Temperature * 10 (e.g., 235 = 23.5°C)
    pub battery_percentage: u8,
    pub battery_voltage_mv: u16,
    pub battery_charging: u8,  // 0 or 1
    pub fps_actual: u16,       // FPS * 10
    pub fps_target: u8,
    pub cpu_usage: u8,         // Percentage
    pub cpu0_usage: u8,        
    pub cpu1_usage: u8,
    pub cpu_freq_mhz: u16,
    pub heap_free: u32,
    pub heap_total: u32,
    pub wifi_rssi: i8,
    pub wifi_connected: u8,    // 0 or 1
    pub display_brightness: u8,
    pub frame_count: u32,
    pub skip_count: u32,
    pub render_time_ms: u16,
    pub flush_time_ms: u16,
}

impl MetricsBinaryPacket {
    pub const VERSION: u8 = 1;
    pub const SIZE: usize = std::mem::size_of::<Self>();
    
    /// Convert from MetricsData
    pub fn from_metrics(metrics: &crate::metrics::MetricsData) -> Self {
        Self {
            version: Self::VERSION,
            timestamp: metrics.timestamp,
            temperature: (metrics.temperature * 10.0) as i16,
            battery_percentage: metrics.battery_percentage,
            battery_voltage_mv: metrics.battery_voltage_mv,
            battery_charging: if metrics.battery_charging { 1 } else { 0 },
            fps_actual: (metrics.fps_actual * 10.0) as u16,
            fps_target: metrics.fps_target as u8,
            cpu_usage: metrics.cpu_usage,
            cpu0_usage: metrics.cpu0_usage,
            cpu1_usage: metrics.cpu1_usage,
            cpu_freq_mhz: metrics.cpu_freq_mhz,
            heap_free: metrics.heap_free,
            heap_total: unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() },
            wifi_rssi: metrics.wifi_rssi,
            wifi_connected: if metrics.wifi_connected { 1 } else { 0 },
            display_brightness: metrics.display_brightness,
            frame_count: metrics.frame_count as u32,
            skip_count: metrics.skip_count as u32,
            render_time_ms: metrics.render_time_ms as u16,
            flush_time_ms: metrics.flush_time_ms as u16,
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let ptr = self as *const Self as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(ptr, Self::SIZE) };
        slice.to_vec()
    }
    
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_packet_size() {
        assert_eq!(MetricsBinaryPacket::SIZE, 63);
    }

}