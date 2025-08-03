/// Simplified streaming home page handler
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;
use core::fmt::Write as FmtWrite;

/// Handle home page with direct streaming (no intermediate buffers)
pub fn handle_home_streaming(req: Request<&mut EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    // Log memory state
    crate::memory_diagnostics::log_memory_state("Streaming home - start");
    
    // Check if memory is critical
    if crate::memory_diagnostics::is_memory_critical() {
        log::error!("Memory critical - refusing request");
        let mut response = req.into_status_response(503)?;
        response.write_all(b"Service temporarily unavailable - low memory")?;
        return Ok(());
    }
    
    // Get system info
    let version = crate::version::DISPLAY_VERSION;
    let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
    let uptime_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
    
    // Format uptime
    let uptime_secs = uptime_ms / 1000;
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;
    
    // Get memory stats
    let mem_stats = crate::memory_diagnostics::MemoryStats::current();
    
    // Create response without chunked encoding (we'll stream but not chunk)
    let mut response = req.into_response(
        200,
        Some("OK"),
        &[
            ("Content-Type", "text/html; charset=utf-8"),
            ("Connection", "close"),
        ]
    )?;
    
    // Use a small stack buffer for formatting
    let mut buffer = [0u8; 256];
    
    // Stream HTML header
    response.write_all(br#"<!DOCTYPE html>
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
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>ESP32-S3 Dashboard</h1>
            <p>Version "#)?;
    
    response.write_all(version.as_bytes())?;
    response.write_all(br#"</p>
        </div>
        
        <div class="card">
            <h2>System Status</h2>"#)?;
    
    // Uptime metric
    response.write_all(br#"<div class="metric">
                <span class="metric-label">Uptime</span>
                <span class="metric-value">"#)?;
    
    // Format uptime into buffer
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<64>::new();
        write!(&mut cursor, "{}h {}m {}s", hours, minutes, seconds).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    
    response.write_all(br#"</span>
            </div>"#)?;
    
    // Free heap metric
    response.write_all(br#"<div class="metric">
                <span class="metric-label">Free Memory</span>
                <span class="metric-value">"#)?;
    
    // Format heap
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{} KB", free_heap / 1024).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    
    response.write_all(br#"</span>
            </div>"#)?;
    
    // Internal DRAM metric
    response.write_all(br#"<div class="metric">
                <span class="metric-label">Internal DRAM</span>
                <span class="metric-value"#)?;
    
    if mem_stats.internal_largest_kb < 4 {
        response.write_all(br#" style="color: #ef4444""#)?;
    }
    response.write_all(b">")?;
    
    // Format DRAM
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<64>::new();
        write!(&mut cursor, "{} KB (largest: {} KB)", 
               mem_stats.internal_free_kb, 
               mem_stats.internal_largest_kb).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    
    response.write_all(br#"</span>
            </div>"#)?;
    
    // PSRAM metric
    response.write_all(br#"<div class="metric">
                <span class="metric-label">PSRAM</span>
                <span class="metric-value">"#)?;
    
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{} KB", mem_stats.psram_free_kb).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    
    response.write_all(br#"</span>
            </div>
        </div>
        
        <div class="card">
            <h2>Quick Links</h2>
            <a href="/api/metrics" class="button">View Metrics</a>
            <a href="/api/system" class="button">System Info</a>
            <a href="/api/config" class="button">Configuration</a>
        </div>
    </div>
</body>
</html>"#)?;
    
    crate::memory_diagnostics::log_memory_state("Streaming home - complete");
    
    Ok(())
}