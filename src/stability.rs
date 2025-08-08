// Stability and recovery mechanisms
use esp_idf_hal::delay::FreeRtos;
use log::{info, warn, error};

/// Initialize panic handler for automatic recovery
pub fn init_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        // Extract panic message
        let panic_msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };
        
        // Log the panic information
        error!("PANIC OCCURRED: {}", panic_msg);
        
        // Try to get location information
        if let Some(location) = panic_info.location() {
            error!("Panic location: {}:{}:{}", 
                location.file(), 
                location.line(), 
                location.column()
            );
        }
        
        // Log diagnostics before restart
        crate::diagnostics::log_panic_info(&panic_msg);
        
        // Try to broadcast to telnet if available
        if let Some(telnet) = crate::logging::get_telnet_server() {
            telnet.log_message("ERROR", &format!("PANIC: {}", panic_msg));
        }
        
        // Give time for logs to be written
        FreeRtos::delay_ms(1000);
        
        // Restart the device automatically
        error!("Restarting device due to panic...");
        unsafe { esp_idf_sys::esp_restart(); }
    }));
    
    info!("Panic handler installed - device will auto-restart on panic");
}

/// Monitor heap health and log warnings
pub struct HeapMonitor {
    min_heap_seen: u32,
    warning_threshold: u32,
    critical_threshold: u32,
}

impl HeapMonitor {
    pub fn new() -> Self {
        let initial_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
        Self {
            min_heap_seen: initial_heap,
            warning_threshold: 150_000,  // 150KB warning
            critical_threshold: 50_000,  // 50KB critical
        }
    }
    
    /// Check heap and return true if healthy, false if critical
    pub fn check(&mut self) -> bool {
        let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
        let min_heap = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
        
        // Update minimum seen
        if free_heap < self.min_heap_seen {
            self.min_heap_seen = free_heap;
        }
        
        // Log status based on thresholds
        if free_heap < self.critical_threshold {
            error!("CRITICAL: Heap exhaustion! Free: {} bytes, Min ever: {} bytes", 
                free_heap, min_heap);
            
            // Try to free some memory by forcing garbage collection
            // In embedded Rust, we can't force GC, but we can log to telnet
            if let Some(telnet) = crate::logging::get_telnet_server() {
                telnet.log_message("ERROR", &format!(
                    "CRITICAL: Low heap! {} bytes free", free_heap
                ));
            }
            
            false // Indicate critical state
        } else if free_heap < self.warning_threshold {
            warn!("Low heap warning: {} bytes free (min ever: {})", 
                free_heap, min_heap);
            true
        } else {
            // Log periodically for monitoring
            if free_heap > self.warning_threshold * 2 {
                info!("Heap healthy: {} bytes free", free_heap);
            }
            true
        }
    }
    
    pub fn get_min_heap(&self) -> u32 {
        self.min_heap_seen
    }
}

/// Get heap statistics for metrics
pub fn get_heap_stats() -> (u32, u32, u32) {
    unsafe {
        let free = esp_idf_sys::esp_get_free_heap_size();
        let min = esp_idf_sys::esp_get_minimum_free_heap_size();
        let largest = esp_idf_sys::heap_caps_get_largest_free_block(
            esp_idf_sys::MALLOC_CAP_DEFAULT
        );
        (free, min, largest as u32)
    }
}