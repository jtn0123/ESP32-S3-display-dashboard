use log::{error, warn, info};
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

// Global diagnostic counters
static MALLOC_FAILURES: AtomicU32 = AtomicU32::new(0);
static NETWORK_ERRORS: AtomicU32 = AtomicU32::new(0);
static DISPLAY_ERRORS: AtomicU32 = AtomicU32::new(0);
static SENSOR_ERRORS: AtomicU32 = AtomicU32::new(0);
static POWER_ISSUES: AtomicU32 = AtomicU32::new(0);
static CRITICAL_ERROR: AtomicBool = AtomicBool::new(false);

// SSE diagnostics
static SSE_CONNECTIONS: AtomicU32 = AtomicU32::new(0);
static SSE_DISCONNECTS: AtomicU32 = AtomicU32::new(0);
static SSE_TIMEOUTS: AtomicU32 = AtomicU32::new(0);
static SSE_HEAP_REJECTIONS: AtomicU32 = AtomicU32::new(0);
static SSE_LIMIT_REJECTIONS: AtomicU32 = AtomicU32::new(0);
static SSE_BYTES_SENT: AtomicU32 = AtomicU32::new(0);
static SSE_EVENTS_SENT: AtomicU32 = AtomicU32::new(0);

// WebSocket diagnostics
static WS_CONNECTIONS: AtomicU32 = AtomicU32::new(0);
static WS_DISCONNECTS: AtomicU32 = AtomicU32::new(0);
static WS_PRUNES: AtomicU32 = AtomicU32::new(0);
static WS_SEND_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Memory allocation diagnostics
pub fn log_allocation_failure(size: usize, context: &str) {
    MALLOC_FAILURES.fetch_add(1, Ordering::Relaxed);
    
    unsafe {
        let free_heap = esp_idf_sys::esp_get_free_heap_size();
        let largest_block = esp_idf_sys::heap_caps_get_largest_free_block(
            esp_idf_sys::MALLOC_CAP_DEFAULT
        );
        let internal_free = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_INTERNAL
        );
        
        error!("MEMORY ALLOCATION FAILED!");
        error!("  Context: {}", context);
        error!("  Requested: {} bytes", size);
        error!("  Free heap: {} bytes", free_heap);
        error!("  Largest block: {} bytes", largest_block);
        error!("  Internal free: {} bytes", internal_free);
        error!("  Total failures: {}", MALLOC_FAILURES.load(Ordering::Relaxed));
        
        // Check if critically low
        if largest_block < 4096 {
            CRITICAL_ERROR.store(true, Ordering::Relaxed);
            error!("CRITICAL: Heap fragmentation severe!");
        }
    }
}

/// Network operation diagnostics
pub fn log_network_error(operation: &str, error: &str) {
    NETWORK_ERRORS.fetch_add(1, Ordering::Relaxed);
    
    error!("NETWORK ERROR: {} - {}", operation, error);
    error!("  Total network errors: {}", NETWORK_ERRORS.load(Ordering::Relaxed));
    
    // Log current network state
    unsafe {
        let mut ap_info = esp_idf_sys::wifi_ap_record_t::default();
        if esp_idf_sys::esp_wifi_sta_get_ap_info(&mut ap_info) == esp_idf_sys::ESP_OK {
            error!("  Current RSSI: {} dBm", ap_info.rssi);
            error!("  Channel: {}", ap_info.primary);
        }
    }
}

/// Display operation diagnostics
pub fn log_display_error(operation: &str, error: &anyhow::Error) {
    DISPLAY_ERRORS.fetch_add(1, Ordering::Relaxed);
    
    error!("DISPLAY ERROR: {} - {:?}", operation, error);
    error!("  Total display errors: {}", DISPLAY_ERRORS.load(Ordering::Relaxed));
}

/// Sensor reading diagnostics
pub fn log_sensor_error(sensor: &str, error: &str) {
    SENSOR_ERRORS.fetch_add(1, Ordering::Relaxed);
    
    warn!("SENSOR ERROR: {} - {}", sensor, error);
    warn!("  Total sensor errors: {}", SENSOR_ERRORS.load(Ordering::Relaxed));
}

/// Boot sequence diagnostics
pub fn log_boot_stage(stage: &str) {
    unsafe {
        let free_heap = esp_idf_sys::esp_get_free_heap_size();
        let uptime = esp_idf_sys::esp_timer_get_time() / 1000; // Convert to ms
        
        info!("BOOT STAGE: {} @ {}ms", stage, uptime);
        info!("  Free heap: {} KB", free_heap / 1024);
        
        // Voltage monitoring disabled
    }
}

/// Performance diagnostics
pub fn log_performance_drop(metric: &str, expected: f32, actual: f32) {
    let drop_percent = ((expected - actual) / expected * 100.0).abs();
    
    if drop_percent > 50.0 {
        error!("SEVERE PERFORMANCE DROP: {}", metric);
        error!("  Expected: {:.1}, Actual: {:.1} ({:.0}% drop)", expected, actual, drop_percent);
    } else if drop_percent > 20.0 {
        warn!("Performance degradation: {}", metric);
        warn!("  Expected: {:.1}, Actual: {:.1} ({:.0}% drop)", expected, actual, drop_percent);
    }
}

/// System health summary
pub fn log_system_health() {
    let malloc_fails = MALLOC_FAILURES.load(Ordering::Relaxed);
    let net_errors = NETWORK_ERRORS.load(Ordering::Relaxed);
    let disp_errors = DISPLAY_ERRORS.load(Ordering::Relaxed);
    let sensor_errors = SENSOR_ERRORS.load(Ordering::Relaxed);
    let power_issues = POWER_ISSUES.load(Ordering::Relaxed);
    let is_critical = CRITICAL_ERROR.load(Ordering::Relaxed);
    
    info!("=== SYSTEM HEALTH SUMMARY ===");
    info!("Memory failures: {}", malloc_fails);
    info!("Network errors: {}", net_errors);
    info!("Display errors: {}", disp_errors);
    info!("Sensor errors: {}", sensor_errors);
    info!("Power issues: {}", power_issues);
    
    // SSE stats
    let sse_stats = get_sse_stats();
    info!("SSE connections: {} (disconnects: {})", 
        sse_stats.total_connections, sse_stats.total_disconnects);
    info!("SSE rejections: {} heap, {} limit", 
        sse_stats.heap_rejections, sse_stats.limit_rejections);
    info!("SSE data sent: {} events, {} KB", 
        sse_stats.events_sent, sse_stats.bytes_sent / 1024);
    let ws_stats = get_ws_stats();
    info!("WS connections: {} (disconnects: {}, prunes: {}) | send failures: {}", 
        ws_stats.total_connections, ws_stats.total_disconnects, ws_stats.prunes, ws_stats.send_failures);
    
    if is_critical {
        error!("CRITICAL ERROR STATE ACTIVE!");
    }
    
    unsafe {
        let free_heap = esp_idf_sys::esp_get_free_heap_size();
        let min_heap = esp_idf_sys::esp_get_minimum_free_heap_size();
        info!("Current heap: {} KB (min: {} KB)", free_heap / 1024, min_heap / 1024);
    }
}

/// Panic diagnostics - log critical info before panic
pub fn log_panic_info(panic_msg: &str) {
    error!("=== PANIC DIAGNOSTICS ===");
    error!("Panic: {}", panic_msg);
    
    unsafe {
        error!("Free heap at panic: {} bytes", esp_idf_sys::esp_get_free_heap_size());
        error!("Stack watermark: {} bytes", esp_idf_sys::uxTaskGetStackHighWaterMark(core::ptr::null_mut()));
    }
    
    // Log error counts
    error!("Error counts before panic:");
    error!("  Memory: {}", MALLOC_FAILURES.load(Ordering::Relaxed));
    error!("  Network: {}", NETWORK_ERRORS.load(Ordering::Relaxed));
    error!("  Display: {}", DISPLAY_ERRORS.load(Ordering::Relaxed));
    error!("  Sensor: {}", SENSOR_ERRORS.load(Ordering::Relaxed));
}

/// Watchdog timer diagnostics
pub fn log_watchdog_feed(_task_name: &str) {
    #[cfg(debug_assertions)]
    {
        use std::sync::{OnceLock, Mutex};
        static LAST_FEED: OnceLock<Mutex<i64>> = OnceLock::new();
        let now = unsafe { esp_idf_sys::esp_timer_get_time() };
        let lock = LAST_FEED.get_or_init(|| Mutex::new(0));
        if let Ok(mut prev) = lock.lock() {
            if *prev != 0 {
                let delta = now - *prev;
                if delta > 5_000_000 { // More than 5 seconds
                    warn!("WATCHDOG: Long gap in {} task: {}ms", task_name, delta / 1000);
                }
            }
            *prev = now;
        }
    }
}

/// Power supply diagnostics
// Removed: voltage monitor report logging (module trimmed)

/// SSE connection diagnostics
pub fn log_sse_event(event: &str, connection_id: Option<u32>) {
    match event {
        "connect" => {
            SSE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
            info!("SSE: New connection {} (total: {})", 
                connection_id.unwrap_or(0), 
                SSE_CONNECTIONS.load(Ordering::Relaxed));
        }
        "disconnect" => {
            SSE_DISCONNECTS.fetch_add(1, Ordering::Relaxed);
            info!("SSE: Connection {} disconnected", connection_id.unwrap_or(0));
        }
        "timeout" => {
            SSE_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            warn!("SSE: Connection {} timed out", connection_id.unwrap_or(0));
        }
        "heap_reject" => {
            SSE_HEAP_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            warn!("SSE: Connection rejected - insufficient heap");
        }
        "limit_reject" => {
            SSE_LIMIT_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            warn!("SSE: Connection rejected - limit reached");
        }
        _ => {}
    }
}

/// Log SSE data transmission
pub fn log_sse_data(bytes: usize, events: u32) {
    SSE_BYTES_SENT.fetch_add(bytes as u32, Ordering::Relaxed);
    SSE_EVENTS_SENT.fetch_add(events, Ordering::Relaxed);
}

/// Get SSE statistics for monitoring
pub fn get_sse_stats() -> SseStats {
    SseStats {
        total_connections: SSE_CONNECTIONS.load(Ordering::Relaxed),
        total_disconnects: SSE_DISCONNECTS.load(Ordering::Relaxed),
        timeout_disconnects: SSE_TIMEOUTS.load(Ordering::Relaxed),
        heap_rejections: SSE_HEAP_REJECTIONS.load(Ordering::Relaxed),
        limit_rejections: SSE_LIMIT_REJECTIONS.load(Ordering::Relaxed),
        bytes_sent: SSE_BYTES_SENT.load(Ordering::Relaxed),
        events_sent: SSE_EVENTS_SENT.load(Ordering::Relaxed),
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SseStats {
    pub total_connections: u32,
    pub total_disconnects: u32,
    pub timeout_disconnects: u32,
    pub heap_rejections: u32,
    pub limit_rejections: u32,
    pub bytes_sent: u32,
    pub events_sent: u32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WsStats {
    pub total_connections: u32,
    pub total_disconnects: u32,
    pub prunes: u32,
    pub send_failures: u32,
}

pub fn get_ws_stats() -> WsStats {
    WsStats {
        total_connections: WS_CONNECTIONS.load(Ordering::Relaxed),
        total_disconnects: WS_DISCONNECTS.load(Ordering::Relaxed),
        prunes: WS_PRUNES.load(Ordering::Relaxed),
        send_failures: WS_SEND_FAILURES.load(Ordering::Relaxed),
    }
}

pub fn log_ws_event(event: &str, _connection_id: Option<u32>) {
    match event {
        "connect" => { WS_CONNECTIONS.fetch_add(1, Ordering::Relaxed); }
        "disconnect" => { WS_DISCONNECTS.fetch_add(1, Ordering::Relaxed); }
        "prune" => { WS_PRUNES.fetch_add(1, Ordering::Relaxed); }
        "send_failure" => { WS_SEND_FAILURES.fetch_add(1, Ordering::Relaxed); }
        _ => {}
    }
}