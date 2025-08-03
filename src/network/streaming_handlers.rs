/// Streaming HTTP handlers to prevent memory fragmentation
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;
use core::fmt::Write as FmtWrite;
use crate::network::streaming::{StreamingResponse, stream_card, format_value};

/// Handle home page with streaming response
pub fn handle_home_streaming<'a>(req: Request<&'a mut EspHttpConnection<'a>>) -> Result<(), Box<dyn std::error::Error>> {
    // Log memory state
    crate::memory_diagnostics::log_memory_state("Streaming home - start");
    
    // Check if memory is critical
    if crate::memory_diagnostics::is_memory_critical() {
        log::error!("Memory critical - refusing request");
        let mut response = req.into_status_response(503)?;
        response.write_all(b"Service temporarily unavailable - low memory")?;
        return Ok(());
    }
    
    // Create streaming response
    let mut stream = StreamingResponse::new(req)?;
    
    // Stream HTML header
    stream.write_str(r#"<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 Dashboard</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: -apple-system, system-ui, sans-serif; background: #f9fafb; margin: 0; padding: 20px; color: #111827; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { background: white; border-radius: 12px; padding: 24px; margin-bottom: 20px; box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07); }
        .header h1 { margin: 0 0 8px 0; font-size: 2rem; color: #111827; }
        .header p { margin: 0; color: #6b7280; }
        .card { background: white; border-radius: 12px; padding: 24px; margin-bottom: 20px; box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07); }
        .card h2 { margin: 0 0 16px 0; font-size: 1.5rem; color: #111827; }
        .metric { display: flex; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid #e5e7eb; }
        .metric:last-child { border-bottom: none; }
        .metric-label { font-weight: 500; color: #111827; }
        .metric-value { color: #3b82f6; font-family: monospace; }
        .status-healthy { color: #10b981; }
        .status-warning { color: #f59e0b; }
        .status-critical { color: #ef4444; }
        .button { display: inline-block; background: #3b82f6; color: white; padding: 10px 20px; border-radius: 8px; text-decoration: none; margin-top: 16px; }
        .button:hover { background: #2563eb; }
        .form-group { margin-bottom: 16px; }
        .form-group label { display: block; margin-bottom: 4px; font-weight: 500; }
        .form-group input, .form-group select { width: 100%; padding: 8px 12px; border: 1px solid #e5e7eb; border-radius: 8px; }
    </style>
</head>
<body>
    <div class="container">"#)?;
    
    // Get system info
    let version = crate::version::DISPLAY_VERSION;
    let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
    let uptime_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
    
    // Format uptime
    let uptime_secs = uptime_ms / 1000;
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;
    
    // Stream header card
    stream.write_str(r#"<div class="header"><h1>ESP32-S3 Dashboard</h1><p>Version "#)?;
    stream.write_str(version)?;
    stream.write_str(r#"</p></div>"#)?;
    
    // Stream system status card
    stream.write_str(r#"<div class="card"><h2>System Status</h2>"#)?;
    
    // Uptime
    stream.write_str(r#"<div class="metric"><span class="metric-label">Uptime</span><span class="metric-value">"#)?;
    let uptime_str = format_value(format_args!("{}h {}m {}s", hours, minutes, seconds));
    stream.write_str(&uptime_str)?;
    stream.write_str(r#"</span></div>"#)?;
    
    // Free heap
    stream.write_str(r#"<div class="metric"><span class="metric-label">Free Memory</span><span class="metric-value">"#)?;
    let heap_str = format_value(format_args!("{} KB", free_heap / 1024));
    stream.write_str(&heap_str)?;
    stream.write_str(r#"</span></div>"#)?;
    
    // Get memory stats
    let mem_stats = crate::memory_diagnostics::MemoryStats::current();
    
    // Internal DRAM
    stream.write_str(r#"<div class="metric"><span class="metric-label">Internal DRAM</span><span class="metric-value"#)?;
    if mem_stats.internal_largest_kb < 4 {
        stream.write_str(r#" class="status-critical""#)?;
    }
    stream.write_str(r#">"#)?;
    let dram_str = format_value(format_args!("{} KB (largest: {} KB)", 
                                            mem_stats.internal_free_kb, 
                                            mem_stats.internal_largest_kb));
    stream.write_str(&dram_str)?;
    stream.write_str(r#"</span></div>"#)?;
    
    // PSRAM
    stream.write_str(r#"<div class="metric"><span class="metric-label">PSRAM</span><span class="metric-value">"#)?;
    let psram_str = format_value(format_args!("{} KB", mem_stats.psram_free_kb));
    stream.write_str(&psram_str)?;
    stream.write_str(r#"</span></div>"#)?;
    
    // Stack
    stream.write_str(r#"<div class="metric"><span class="metric-label">Stack Free</span><span class="metric-value"#)?;
    if mem_stats.stack_remaining < 1024 {
        stream.write_str(r#" class="status-warning""#)?;
    }
    stream.write_str(r#">"#)?;
    let stack_str = format_value(format_args!("{} bytes", mem_stats.stack_remaining));
    stream.write_str(&stack_str)?;
    stream.write_str(r#"</span></div>"#)?;
    
    stream.write_str(r#"</div>"#)?; // Close system status card
    
    // Stream network status card
    stream.write_str(r#"<div class="card"><h2>Network Status</h2>"#)?;
    
    // Check WiFi status
    let wifi_sta = unsafe { 
        let key = b"WIFI_STA_DEF\0";
        esp_idf_sys::esp_netif_get_handle_from_ifkey(key.as_ptr() as *const ::core::ffi::c_char) 
    };
    
    if !wifi_sta.is_null() {
        unsafe {
            let mut ip_info = esp_idf_sys::esp_netif_ip_info_t::default();
            if esp_idf_sys::esp_netif_get_ip_info(wifi_sta, &mut ip_info) == esp_idf_sys::ESP_OK {
                // IP Address
                stream.write_str(r#"<div class="metric"><span class="metric-label">IP Address</span><span class="metric-value">"#)?;
                let ip_str = format_value(format_args!("{}.{}.{}.{}", 
                    ip_info.ip.addr & 0xff,
                    (ip_info.ip.addr >> 8) & 0xff,
                    (ip_info.ip.addr >> 16) & 0xff,
                    (ip_info.ip.addr >> 24) & 0xff));
                stream.write_str(&ip_str)?;
                stream.write_str(r#"</span></div>"#)?;
            }
        }
    } else {
        stream.write_str(r#"<div class="metric"><span class="metric-label">Status</span><span class="metric-value status-critical">Not Connected</span></div>"#)?;
    }
    
    stream.write_str(r#"</div>"#)?; // Close network card
    
    // Stream quick links card
    stream.write_str(r#"<div class="card"><h2>Quick Links</h2>"#)?;
    stream.write_str(r#"<a href="/api/metrics" class="button">View Metrics</a> "#)?;
    stream.write_str(r#"<a href="/api/system" class="button">System Info</a> "#)?;
    stream.write_str(r#"<a href="/api/config" class="button">Configuration</a>"#)?;
    stream.write_str(r#"</div>"#)?;
    
    // Stream footer
    stream.write_str(r#"
    </div>
</body>
</html>"#)?;
    
    // Finish streaming
    stream.finish()?;
    
    crate::memory_diagnostics::log_memory_state("Streaming home - complete");
    
    Ok(())
}