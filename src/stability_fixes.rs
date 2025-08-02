// Stability improvements module
use esp_idf_hal::delay::FreeRtos;
use log::{info, warn, error};

/// Initialize watchdog with proper timeout and monitoring
pub fn init_watchdog() -> anyhow::Result<()> {
    unsafe {
        // Initialize with 30 second timeout for boot
        esp_idf_sys::esp_task_wdt_init(30, true);
        
        // Add current task to watchdog
        esp_idf_sys::esp_task_wdt_add(std::ptr::null_mut());
        
        info!("Watchdog initialized with 30s timeout");
    }
    Ok(())
}

/// Feed watchdog during long operations
pub fn feed_watchdog() {
    unsafe {
        esp_idf_sys::esp_task_wdt_reset();
    }
}

/// Perform WiFi scan with watchdog feeding
pub fn safe_wifi_scan<F, R>(scan_fn: F) -> anyhow::Result<R>
where
    F: FnOnce() -> anyhow::Result<R>,
{
    // Don't disable watchdog, just feed it periodically
    std::thread::spawn(|| {
        for _ in 0..30 {  // 30 seconds max
            FreeRtos::delay_ms(1000);
            feed_watchdog();
        }
    });
    
    scan_fn()
}

/// Recovery mechanism for critical failures  
pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        error!("PANIC: {}", info);
        
        // Try to log to telnet if available
        if let Some(telnet) = crate::logging::get_telnet_server() {
            let _ = telnet.broadcast(&format!("PANIC: {}", info));
        }
        
        // Give time for logs to flush
        FreeRtos::delay_ms(1000);
        
        // Restart
        unsafe { esp_idf_sys::esp_restart(); }
    }));
}

/// Monitor heap and restart if critically low
pub fn monitor_heap_health() -> bool {
    let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
    let min_heap = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
    
    if free_heap < 50_000 {  // Less than 50KB
        error!("CRITICAL: Heap exhaustion! Free: {}, Min: {}", free_heap, min_heap);
        return false;
    }
    
    if free_heap < 100_000 {  // Warning threshold
        warn!("Low heap warning: {} bytes free", free_heap);
    }
    
    true
}

/// Startup delay reduction - parallel initialization
pub struct FastBoot;

impl FastBoot {
    pub fn init_parallel() -> anyhow::Result<()> {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let errors = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];
        
        // Initialize display in parallel
        let errors_clone = errors.clone();
        handles.push(thread::spawn(move || {
            if let Err(e) = init_display_fast() {
                errors_clone.lock().unwrap().push(format!("Display: {}", e));
            }
        }));
        
        // Initialize sensors in parallel  
        let errors_clone = errors.clone();
        handles.push(thread::spawn(move || {
            if let Err(e) = init_sensors_fast() {
                errors_clone.lock().unwrap().push(format!("Sensors: {}", e));
            }
        }));
        
        // Wait for all to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Check for errors
        let errors = errors.lock().unwrap();
        if !errors.is_empty() {
            for err in errors.iter() {
                error!("Parallel init error: {}", err);
            }
            anyhow::bail!("Parallel initialization failed");
        }
        
        Ok(())
    }
}

fn init_display_fast() -> anyhow::Result<()> {
    // Placeholder - move display init here
    Ok(())
}

fn init_sensors_fast() -> anyhow::Result<()> {
    // Placeholder - move sensor init here  
    Ok(())
}