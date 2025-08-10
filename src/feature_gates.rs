/// Feature gates based on heap availability
use esp_idf_sys::{esp_get_free_heap_size, esp_get_minimum_free_heap_size};
use log::{info, warn};

/// Feature requirements in KB
pub const STATIC_FILES_MIN_HEAP_KB: u32 = 100;  // 100KB minimum for static files
pub const SSE_MIN_HEAP_KB: u32 = 80;            // 80KB minimum for SSE
pub const SSE_PER_CLIENT_KB: u32 = 10;          // 10KB per SSE client

#[derive(Debug, Clone, Copy)]
pub struct FeatureStatus {
    pub sse_enabled: bool,
    pub max_sse_clients: usize,
}

impl FeatureStatus {
    pub fn check() -> Self {
        let free_kb = unsafe { esp_get_free_heap_size() } / 1024;
        let min_kb = unsafe { esp_get_minimum_free_heap_size() } / 1024;
        
        info!("FEATURES: Checking heap - Free: {} KB, Min: {} KB", free_kb, min_kb);
        
        // Check static files
        let _static_files_enabled = free_kb >= STATIC_FILES_MIN_HEAP_KB && min_kb >= 80;
        
        // Check SSE
        let sse_enabled = free_kb >= SSE_MIN_HEAP_KB && min_kb >= 60;
        let max_sse_clients = if sse_enabled {
            // Calculate how many SSE clients we can support
            let available_for_sse = (free_kb - SSE_MIN_HEAP_KB) / SSE_PER_CLIENT_KB;
            let clients = available_for_sse.min(3) as usize; // Cap at 3
            info!("FEATURES: SSE - ENABLED with {} max clients", clients);
            clients
        } else {
            warn!("FEATURES: SSE - DISABLED (need {} KB, have {} KB)", 
                  SSE_MIN_HEAP_KB, free_kb);
            0
        };
        
        Self {
            sse_enabled,
            max_sse_clients,
        }
    }
    
    // Removed: verbose status logger
}