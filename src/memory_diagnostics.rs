/// Memory diagnostics utilities for debugging heap fragmentation issues
use esp_idf_sys::*;

/// Log current memory state with detailed breakdown
pub fn log_memory_state(label: &str) {
    unsafe {
        let internal_free = heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32);
        let internal_largest = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL as u32);
        let internal_min = heap_caps_get_minimum_free_size(MALLOC_CAP_INTERNAL as u32);
        
        let psram_free = heap_caps_get_free_size(MALLOC_CAP_SPIRAM as u32);
        let psram_largest = heap_caps_get_largest_free_block(MALLOC_CAP_SPIRAM as u32);
        
        let total_free = heap_caps_get_free_size(MALLOC_CAP_DEFAULT as u32);
        
        // Get current task stack watermark
        let stack_watermark = uxTaskGetStackHighWaterMark(std::ptr::null_mut());
        
        log::warn!("📊 Memory [{}]:", label);
        log::warn!("  Internal DRAM: free={} KB, largest={} KB, min={} KB", 
                  internal_free / 1024, internal_largest / 1024, internal_min / 1024);
        log::warn!("  PSRAM: free={} KB, largest={} KB", 
                  psram_free / 1024, psram_largest / 1024);
        log::warn!("  Total free: {} KB, Stack remaining: {} bytes", 
                  total_free / 1024, stack_watermark);
        
        // Critical warnings
        if internal_largest < 4096 {
            log::error!("⚠️  CRITICAL: Internal DRAM largest block < 4KB!");
        }
        if stack_watermark < 1024 {
            log::error!("⚠️  CRITICAL: Stack watermark < 1KB!");
        }
    }
}

/// Check if memory is critically low
pub fn is_memory_critical() -> bool {
    unsafe {
        let internal_largest = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL as u32);
        let stack_watermark = uxTaskGetStackHighWaterMark(std::ptr::null_mut());
        
        internal_largest < 4096 || stack_watermark < 1024
    }
}

/// Get memory statistics for JSON response
pub struct MemoryStats {
    pub internal_free_kb: u32,
    pub internal_largest_kb: u32,
    pub psram_free_kb: u32,
}

impl MemoryStats {
    pub fn current() -> Self {
        unsafe {
            Self {
                internal_free_kb: (heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32) / 1024) as u32,
                internal_largest_kb: (heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL as u32) / 1024) as u32,
                psram_free_kb: (heap_caps_get_free_size(MALLOC_CAP_SPIRAM as u32) / 1024) as u32,
            }
        }
    }
}