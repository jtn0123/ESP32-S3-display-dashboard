/// Memory diagnostics utilities for debugging heap fragmentation issues
use esp_idf_sys::*;
use core::sync::atomic::{AtomicU8, Ordering};

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
#[allow(dead_code)]
pub fn is_memory_critical() -> bool {
    unsafe {
        let internal_largest = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL as u32);
        let stack_watermark = uxTaskGetStackHighWaterMark(std::ptr::null_mut());
        
        internal_largest < 4096 || stack_watermark < 1024
    }
}

// Global heap pressure state
static HEAP_PRESSURE_LEVEL: AtomicU8 = AtomicU8::new(0); // 0=Normal,1=Warn,2=Critical

pub fn heap_pressure_level() -> u8 {
    HEAP_PRESSURE_LEVEL.load(Ordering::Relaxed)
}

pub fn start_heap_pressure_monitor() {
    std::thread::spawn(|| loop {
        unsafe {
            let internal_free = heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32);
            let largest = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL as u32);
            let level = if largest < 6 * 1024 || internal_free < 40 * 1024 {
                2
            } else if largest < 12 * 1024 || internal_free < 80 * 1024 {
                1
            } else {
                0
            };
            HEAP_PRESSURE_LEVEL.store(level, Ordering::Relaxed);
        }
        std::thread::sleep(core::time::Duration::from_millis(500));
    });
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