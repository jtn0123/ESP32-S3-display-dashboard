use crate::metrics::MetricsData;
use std::fmt::Write;

/// Efficient metrics formatter for Prometheus format
pub struct MetricsFormatter {
    buffer: String,
}

impl MetricsFormatter {
    /// Create a new formatter with pre-allocated buffer
    pub fn new() -> Self {
        Self {
            // Pre-allocate based on typical metrics size (~2KB)
            buffer: String::with_capacity(2048),
        }
    }

    /// Format all metrics into Prometheus format
    pub fn format_metrics(
        &mut self,
        metrics_data: &MetricsData,
        version: &str,
        board_type: &str,
        chip_model: &str,
        uptime_seconds: u64,
        heap_free: u32,
        heap_total: u32,
    ) -> Result<String, std::fmt::Error> {
        self.buffer.clear();

        // Device info
        self.write_metric(
            "esp32_device_info",
            "Device information",
            "gauge",
            &format!("version=\"{}\",board=\"{}\",model=\"{}\"", version, board_type, chip_model),
            1.0,
        )?;

        // System metrics
        self.write_simple_metric("esp32_uptime_seconds", "Total uptime in seconds", "counter", uptime_seconds as f64)?;
        self.write_simple_metric("esp32_heap_free_bytes", "Current free heap memory in bytes", "gauge", heap_free as f64)?;
        self.write_simple_metric("esp32_heap_total_bytes", "Total heap memory in bytes", "gauge", heap_total as f64)?;

        // Performance metrics
        self.write_simple_metric("esp32_fps_actual", "Current actual frames per second", "gauge", metrics_data.fps_actual as f64)?;
        self.write_simple_metric("esp32_fps_target", "Target frames per second", "gauge", metrics_data.fps_target as f64)?;

        // CPU metrics
        self.write_simple_metric("esp32_cpu_usage_percent", "CPU usage percentage (average)", "gauge", metrics_data.cpu_usage as f64)?;
        self.write_simple_metric("esp32_cpu0_usage_percent", "CPU Core 0 usage percentage", "gauge", metrics_data.cpu0_usage as f64)?;
        self.write_simple_metric("esp32_cpu1_usage_percent", "CPU Core 1 usage percentage", "gauge", metrics_data.cpu1_usage as f64)?;
        self.write_simple_metric("esp32_cpu_freq_mhz", "CPU frequency in MHz", "gauge", metrics_data.cpu_freq_mhz as f64)?;

        // Temperature
        self.write_simple_metric("esp32_temperature_celsius", "Internal temperature in Celsius", "gauge", metrics_data.temperature as f64)?;

        // WiFi metrics
        self.write_simple_metric("esp32_wifi_rssi_dbm", "WiFi signal strength in dBm", "gauge", metrics_data.wifi_rssi as f64)?;
        
        let wifi_ssid = if metrics_data.wifi_connected { 
            &metrics_data.wifi_ssid 
        } else { 
            "_disconnected" 
        };
        self.write_metric(
            "esp32_wifi_connected",
            "WiFi connection status (0=disconnected, 1=connected)",
            "gauge",
            &format!("ssid=\"{}\"", wifi_ssid),
            if metrics_data.wifi_connected { 1.0 } else { 0.0 },
        )?;

        // Display metrics
        self.write_simple_metric("esp32_display_brightness", "Display brightness level (0-255)", "gauge", metrics_data.display_brightness as f64)?;

        // Battery metrics
        self.write_simple_metric("esp32_battery_voltage_mv", "Battery voltage in millivolts", "gauge", metrics_data.battery_voltage_mv as f64)?;
        self.write_simple_metric("esp32_battery_percentage", "Battery charge percentage", "gauge", metrics_data.battery_percentage as f64)?;
        self.write_simple_metric("esp32_battery_charging", "Battery charging status (0=not charging, 1=charging)", "gauge", 
            if metrics_data.battery_charging { 1.0 } else { 0.0 })?;

        // Timing metrics
        self.write_simple_metric("esp32_render_time_milliseconds", "Display render time in milliseconds", "gauge", metrics_data.render_time_ms as f64)?;
        self.write_simple_metric("esp32_flush_time_milliseconds", "Display flush time in milliseconds", "gauge", metrics_data.flush_time_ms as f64)?;

        // Frame statistics
        let skip_rate = if metrics_data.frame_count > 0 {
            metrics_data.skip_count as f64 / metrics_data.frame_count as f64 * 100.0
        } else {
            0.0
        };
        self.write_simple_metric("esp32_frame_skip_rate_percent", "Percentage of frames skipped", "gauge", skip_rate)?;
        self.write_simple_metric("esp32_total_frames_count", "Total number of frames processed", "counter", metrics_data.frame_count as f64)?;
        self.write_simple_metric("esp32_skipped_frames_count", "Number of frames skipped", "counter", metrics_data.skip_count as f64)?;

        // PSRAM metrics
        self.write_simple_metric("esp32_psram_free_bytes", "Free PSRAM memory in bytes", "gauge", metrics_data.psram_free as f64)?;
        self.write_simple_metric("esp32_psram_total_bytes", "Total PSRAM memory in bytes", "gauge", metrics_data.psram_total as f64)?;
        
        let psram_usage = if metrics_data.psram_total > 0 {
            (metrics_data.psram_total - metrics_data.psram_free) as f64 / metrics_data.psram_total as f64 * 100.0
        } else {
            0.0
        };
        self.write_simple_metric("esp32_psram_used_percent", "PSRAM usage percentage", "gauge", psram_usage)?;

        // Button metrics (if available)
        if metrics_data.button_events_total > 0 {
            self.write_simple_metric("esp32_button_avg_response_ms", "Average button response time in milliseconds", "gauge", metrics_data.button_avg_response_ms as f64)?;
            self.write_simple_metric("esp32_button_max_response_ms", "Maximum button response time in milliseconds", "gauge", metrics_data.button_max_response_ms as f64)?;
            self.write_simple_metric("esp32_button_events_total", "Total button events", "counter", metrics_data.button_events_total as f64)?;
            self.write_simple_metric("esp32_button_events_per_second", "Button events per second", "gauge", metrics_data.button_events_per_second as f64)?;
        }

        Ok(self.buffer.clone())
    }

    /// Write a simple metric without labels
    fn write_simple_metric(&mut self, name: &str, help: &str, metric_type: &str, value: f64) -> Result<(), std::fmt::Error> {
        writeln!(&mut self.buffer, "# HELP {} {}", name, help)?;
        writeln!(&mut self.buffer, "# TYPE {} {}", name, metric_type)?;
        writeln!(&mut self.buffer, "{} {}", name, value)?;
        writeln!(&mut self.buffer)?;
        Ok(())
    }

    /// Write a metric with labels
    fn write_metric(&mut self, name: &str, help: &str, metric_type: &str, labels: &str, value: f64) -> Result<(), std::fmt::Error> {
        writeln!(&mut self.buffer, "# HELP {} {}", name, help)?;
        writeln!(&mut self.buffer, "# TYPE {} {}", name, metric_type)?;
        writeln!(&mut self.buffer, "{}{{{}}} {}", name, labels, value)?;
        writeln!(&mut self.buffer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_formatting() {
        let mut formatter = MetricsFormatter::new();
        let metrics = MetricsData {
            cpu_usage: 50,
            fps_actual: 30.5,
            wifi_connected: true,
            wifi_ssid: "TestNetwork".to_string(),
            ..Default::default()
        };

        let result = formatter.format_metrics(
            &metrics,
            "1.0.0",
            "ESP32-S3",
            "T-Display",
            100,
            1024,
            2048,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("esp32_device_info"));
        assert!(output.contains("esp32_cpu_usage_percent 50"));
        assert!(output.contains("esp32_fps_actual 30.5"));
    }
}