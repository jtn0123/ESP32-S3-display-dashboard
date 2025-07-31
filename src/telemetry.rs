use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub operation_name: String,
    pub start_time: u64,
    pub duration_ms: u32,
    pub tags: SpanTags,
}

#[derive(Serialize, Clone)]
pub struct SpanTags {
    pub component: String,
    pub fps_actual: f32,
    pub fps_target: f32,
    pub cpu_usage: u8,
    pub memory_free: u32,
    pub temperature: f32,
}

#[derive(Serialize)]
pub struct TelemetryExport {
    pub traces: Vec<Span>,
    pub metrics: Metrics,
}

#[derive(Serialize)]
pub struct Metrics {
    pub timestamp: u64,
    pub performance: PerformanceMetrics,
    pub system: SystemMetrics,
    pub display: DisplayMetrics,
}

#[derive(Serialize)]
pub struct PerformanceMetrics {
    pub fps_actual: f32,
    pub fps_target: f32,
    pub frame_time_ms: u32,
    pub render_time_ms: u32,
    pub flush_time_ms: u32,
    pub skip_rate: u8,
}

#[derive(Serialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: u8,
    pub cpu_freq_mhz: u16,
    pub heap_free_bytes: u32,
    pub heap_total_bytes: u32,
    pub psram_free_bytes: u32,
    pub temperature_celsius: f32,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct DisplayMetrics {
    pub brightness: u8,
    pub dirty_rect_count: u8,
    pub total_pixels_updated: u32,
    pub backlight_on: bool,
}

pub struct TelemetryCollector {
    traces: Vec<Span>,
    trace_counter: u64,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self {
            traces: Vec::new(),
            trace_counter: 0,
        }
    }

    pub fn start_span(&mut self, operation: &str) -> String {
        self.trace_counter += 1;
        format!("esp32-trace-{}", self.trace_counter)
    }

    pub fn end_span(
        &mut self,
        trace_id: String,
        operation_name: String,
        duration_ms: u32,
        fps_actual: f32,
        cpu_usage: u8,
        memory_free: u32,
        temperature: f32,
    ) {
        let span = Span {
            trace_id: trace_id.clone(),
            span_id: format!("{}-span-1", trace_id),
            operation_name,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis() as u64
                - duration_ms as u64,
            duration_ms,
            tags: SpanTags {
                component: "esp32-dashboard".to_string(),
                fps_actual,
                fps_target: 60.0,
                cpu_usage,
                memory_free,
                temperature,
            },
        };
        
        self.traces.push(span);
        
        // Keep only last 100 traces
        if self.traces.len() > 100 {
            self.traces.remove(0);
        }
    }

    pub fn export_json(&self, metrics: Metrics) -> String {
        let export = TelemetryExport {
            traces: self.traces.clone(),
            metrics,
        };
        
        serde_json::to_string(&export).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn clear_traces(&mut self) {
        self.traces.clear();
    }
}

// Helper to format OpenTelemetry compatible traces for telnet export
pub fn format_otlp_trace(span: &Span) -> String {
    format!(
        "{{\"resourceSpans\":[{{\"resource\":{{\"attributes\":[{{\"key\":\"service.name\",\"value\":{{\"stringValue\":\"esp32-dashboard\"}}}}]}},\"scopeSpans\":[{{\"spans\":[{{\"traceId\":\"{}\",\"spanId\":\"{}\",\"name\":\"{}\",\"startTimeUnixNano\":{},\"endTimeUnixNano\":{},\"attributes\":[{{\"key\":\"fps\",\"value\":{{\"doubleValue\":{}}}}}]}}]}}]}}]}}",
        span.trace_id,
        span.span_id,
        span.operation_name,
        span.start_time * 1_000_000,
        (span.start_time + span.duration_ms as u64) * 1_000_000,
        span.tags.fps_actual
    )
}