/// Boot diagnostics with heap monitoring for safe feature rollout
use esp_idf_sys::{esp_get_free_heap_size, esp_get_minimum_free_heap_size};
use log::info;

pub struct BootStage {
    name: &'static str,
    heap_before: u32,
    min_heap_before: u32,
}

impl BootStage {
    pub fn new(name: &'static str) -> Self {
        let heap_before = unsafe { esp_get_free_heap_size() };
        let min_heap_before = unsafe { esp_get_minimum_free_heap_size() };
        
        info!("BOOT: Starting {} - Free: {} KB, Min: {} KB", 
              name, heap_before / 1024, min_heap_before / 1024);
        
        Self {
            name,
            heap_before,
            min_heap_before,
        }
    }
    
    pub fn complete(self) {
        let heap_after = unsafe { esp_get_free_heap_size() };
        let min_heap_after = unsafe { esp_get_minimum_free_heap_size() };
        let used = self.heap_before.saturating_sub(heap_after);
        
        info!("BOOT: Completed {} - Used: {} bytes, Free: {} KB, Min: {} KB", 
              self.name, used, heap_after / 1024, min_heap_after / 1024);
              
        // Alert if heap is getting low
        if heap_after < 80 * 1024 {
            log::warn!("BOOT: Low heap warning after {}: {} KB free", self.name, heap_after / 1024);
        }
        
        if min_heap_after < 60 * 1024 {
            log::error!("BOOT: Critical min heap after {}: {} KB", self.name, min_heap_after / 1024);
        }
    }
}

/// Check if we have enough heap to enable a feature
pub fn can_enable_feature(feature_name: &str, required_kb: u32) -> bool {
    let free = unsafe { esp_get_free_heap_size() } / 1024;
    let min = unsafe { esp_get_minimum_free_heap_size() } / 1024;
    
    if free >= required_kb && min >= required_kb - 20 {
        info!("BOOT: Feature '{}' enabled - {} KB free (required: {} KB)", 
              feature_name, free, required_kb);
        true
    } else {
        log::warn!("BOOT: Feature '{}' disabled - {} KB free (required: {} KB)", 
                   feature_name, free, required_kb);
        false
    }
}

/// Monitor heap during runtime
pub fn log_heap_stats(context: &str) {
    let free = unsafe { esp_get_free_heap_size() };
    let min = unsafe { esp_get_minimum_free_heap_size() };
    info!("HEAP [{}]: Free: {} KB, Min: {} KB", context, free / 1024, min / 1024);
}