/// Streaming metrics endpoint to prevent memory fragmentation
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;

/// Handle metrics endpoint with streaming response
pub fn handle_metrics_streaming(req: Request<&mut EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Metrics endpoint called - streaming version");
    
    // Log memory state
    crate::memory_diagnostics::log_memory_state("Streaming metrics - start");
    
    // Get system metrics
    let uptime_seconds = unsafe { esp_idf_sys::esp_timer_get_time() / 1_000_000 } as u64;
    let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
    let heap_total = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
    
    // Get device info for labels
    let version = crate::version::DISPLAY_VERSION;
    let board_type = "ESP32-S3";
    let chip_model = "T-Display-S3";
    
    // Get memory stats
    let mem_stats = crate::memory_diagnostics::MemoryStats::current();
    
    // Create response
    let mut response = req.into_response(
        200,
        Some("OK"),
        &[
            ("Content-Type", "text/plain; version=0.0.4"),
            ("Connection", "close"),
        ]
    )?;
    
    // Use a small stack buffer for formatting
    let mut buffer = [0u8; 256];
    
    // Stream device info
    response.write_all(b"# HELP esp32_device_info Device information\n")?;
    response.write_all(b"# TYPE esp32_device_info gauge\n")?;
    response.write_all(b"esp32_device_info{version=\"")?;
    response.write_all(version.as_bytes())?;
    response.write_all(b"\",board=\"")?;
    response.write_all(board_type.as_bytes())?;
    response.write_all(b"\",model=\"")?;
    response.write_all(chip_model.as_bytes())?;
    response.write_all(b"\"} 1\n\n")?;
    
    // Stream uptime
    response.write_all(b"# HELP esp32_uptime_seconds Total uptime in seconds\n")?;
    response.write_all(b"# TYPE esp32_uptime_seconds counter\n")?;
    response.write_all(b"esp32_uptime_seconds ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", uptime_seconds).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Stream heap metrics
    response.write_all(b"# HELP esp32_heap_free_bytes Current free heap memory in bytes\n")?;
    response.write_all(b"# TYPE esp32_heap_free_bytes gauge\n")?;
    response.write_all(b"esp32_heap_free_bytes ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", heap_free).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Stream total heap
    response.write_all(b"# HELP esp32_heap_total_bytes Total heap memory in bytes\n")?;
    response.write_all(b"# TYPE esp32_heap_total_bytes gauge\n")?;
    response.write_all(b"esp32_heap_total_bytes ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", heap_total).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Stream internal DRAM metrics
    response.write_all(b"# HELP esp32_internal_dram_free_kb Free internal DRAM in KB\n")?;
    response.write_all(b"# TYPE esp32_internal_dram_free_kb gauge\n")?;
    response.write_all(b"esp32_internal_dram_free_kb ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", mem_stats.internal_free_kb).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Stream internal DRAM largest block
    response.write_all(b"# HELP esp32_internal_dram_largest_kb Largest free internal DRAM block in KB\n")?;
    response.write_all(b"# TYPE esp32_internal_dram_largest_kb gauge\n")?;
    response.write_all(b"esp32_internal_dram_largest_kb ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", mem_stats.internal_largest_kb).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Stream PSRAM metrics
    response.write_all(b"# HELP esp32_psram_free_kb Free PSRAM in KB\n")?;
    response.write_all(b"# TYPE esp32_psram_free_kb gauge\n")?;
    response.write_all(b"esp32_psram_free_kb ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", mem_stats.psram_free_kb).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Stream stack watermark
    response.write_all(b"# HELP esp32_stack_remaining_bytes Remaining stack space in bytes\n")?;
    response.write_all(b"# TYPE esp32_stack_remaining_bytes gauge\n")?;
    response.write_all(b"esp32_stack_remaining_bytes ")?;
    let len = {
        use core::fmt::Write;
        let mut cursor = heapless::String::<32>::new();
        write!(&mut cursor, "{}", mem_stats.stack_remaining).ok();
        let bytes = cursor.as_bytes();
        buffer[..bytes.len()].copy_from_slice(bytes);
        bytes.len()
    };
    response.write_all(&buffer[..len])?;
    response.write_all(b"\n\n")?;
    
    // Try to add sensor metrics if available
    if let Ok(metrics_guard) = crate::metrics::metrics().try_lock() {
        // Temperature
        response.write_all(b"# HELP esp32_temperature_celsius Current temperature in Celsius\n")?;
        response.write_all(b"# TYPE esp32_temperature_celsius gauge\n")?;
        response.write_all(b"esp32_temperature_celsius ")?;
        let len = {
            use core::fmt::Write;
            let mut cursor = heapless::String::<32>::new();
            write!(&mut cursor, "{:.2}", metrics_guard.temperature).ok();
            let bytes = cursor.as_bytes();
            buffer[..bytes.len()].copy_from_slice(bytes);
            bytes.len()
        };
        response.write_all(&buffer[..len])?;
        response.write_all(b"\n\n")?;
        
        // CPU usage
        response.write_all(b"# HELP esp32_cpu_usage_percent CPU usage percentage\n")?;
        response.write_all(b"# TYPE esp32_cpu_usage_percent gauge\n")?;
        response.write_all(b"esp32_cpu_usage_percent{core=\"0\"} ")?;
        let len = {
            use core::fmt::Write;
            let mut cursor = heapless::String::<32>::new();
            write!(&mut cursor, "{}", metrics_guard.cpu_usage).ok();
            let bytes = cursor.as_bytes();
            buffer[..bytes.len()].copy_from_slice(bytes);
            bytes.len()
        };
        response.write_all(&buffer[..len])?;
        response.write_all(b"\n\n")?;
    } else {
        // Metrics unavailable
        response.write_all(b"# HELP esp32_metrics_unavailable Metrics temporarily unavailable\n")?;
        response.write_all(b"# TYPE esp32_metrics_unavailable gauge\n")?;
        response.write_all(b"esp32_metrics_unavailable 1\n")?;
    }
    
    crate::memory_diagnostics::log_memory_state("Streaming metrics - complete");
    
    Ok(())
}