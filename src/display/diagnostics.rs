/// Memory and stack diagnostics for ESP_LCD debugging

use esp_idf_sys::*;
use log::info;

pub fn print_memory_stats(label: &str) {
    unsafe {
        let free_heap = esp_get_free_heap_size();
        let min_free_heap = esp_get_minimum_free_heap_size();
        let largest_block = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL);
        
        info!("[MEM] {} - Free: {}KB, Min: {}KB, Largest: {}KB", 
            label, 
            free_heap / 1024,
            min_free_heap / 1024,
            largest_block / 1024
        );
        
        // Check PSRAM if available
        let psram_free = heap_caps_get_free_size(MALLOC_CAP_SPIRAM);
        if psram_free > 0 {
            let psram_largest = heap_caps_get_largest_free_block(MALLOC_CAP_SPIRAM);
            info!("[MEM] {} - PSRAM Free: {}KB, Largest: {}KB",
                label,
                psram_free / 1024,
                psram_largest / 1024
            );
        }
    }
}

pub fn print_stack_watermark(label: &str) {
    unsafe {
        let watermark = uxTaskGetStackHighWaterMark(std::ptr::null_mut());
        let current_task = pcTaskGetName(std::ptr::null_mut());
        let task_name = std::ffi::CStr::from_ptr(current_task).to_string_lossy();
        
        info!("[STACK] {} - Task: {}, Free stack: {} bytes", 
            label, 
            task_name,
            watermark * 4  // Each stack word is 4 bytes
        );
    }
}

pub fn check_dma_capable_memory() -> (usize, usize) {
    unsafe {
        let internal_dma = heap_caps_get_free_size(MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA);
        let largest_dma = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL | MALLOC_CAP_DMA);
        
        info!("[DMA] Internal DMA memory - Free: {}KB, Largest: {}KB",
            internal_dma / 1024,
            largest_dma / 1024
        );
        
        (internal_dma, largest_dma)
    }
}