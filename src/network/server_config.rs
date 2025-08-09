use esp_idf_svc::http::server::Configuration;
use esp_idf_sys as _;

pub struct StableServerConfig;

impl StableServerConfig {
    /// Creates an optimized HTTP server configuration for stability
    pub fn create() -> Configuration {
        // Hardened configuration based on ESP32 checklist
        // Note: esp-idf-svc has limited configuration options compared to raw ESP-IDF
        // Be conservative with what we change from defaults
        Configuration {
            // Use our tuned values
            stack_size: Self::stack_size(),
            max_uri_handlers: 80,
            max_open_sockets: Self::max_sockets() as usize,
            max_resp_headers: 12,
            lru_purge_enable: true,
            ..Default::default()
        }
    }
    
    /// Get stack size configuration value
    pub const fn stack_size() -> usize { 24576 }
    
    /// Get max sockets configuration value  
    pub const fn max_sockets() -> u16 { 12 }
}

/// HTTP request instrumentation for diagnostics
pub struct RequestInstrumentation {
    pub request_id: Option<String>,
    pub start_heap_free: u32,
    pub start_heap_largest: u32,
    pub start_psram_free: u32,
    pub start_stack_watermark: u32,
    pub start_time: u64,
}

impl RequestInstrumentation {
    pub fn capture(request_id: Option<String>) -> Self {
        unsafe {
            Self {
                request_id,
                start_heap_free: esp_idf_sys::esp_get_free_heap_size(),
                start_heap_largest: esp_idf_sys::heap_caps_get_largest_free_block(
                    esp_idf_sys::MALLOC_CAP_INTERNAL
                ) as u32,
                start_psram_free: esp_idf_sys::heap_caps_get_free_size(
                    esp_idf_sys::MALLOC_CAP_SPIRAM
                ) as u32,
                start_stack_watermark: esp_idf_sys::uxTaskGetStackHighWaterMark(
                    std::ptr::null_mut()
                ),
                start_time: esp_idf_sys::esp_timer_get_time() as u64,
            }
        }
    }
    
    pub fn log_completion(&self, path: &str, status: u16) {
        unsafe {
            let end_time = esp_idf_sys::esp_timer_get_time() as u64;
            let duration_ms = (end_time - self.start_time) / 1000;
            
            let end_heap_free = esp_idf_sys::esp_get_free_heap_size();
            let end_heap_largest = esp_idf_sys::heap_caps_get_largest_free_block(
                esp_idf_sys::MALLOC_CAP_INTERNAL
            );
            let end_psram_free = esp_idf_sys::heap_caps_get_free_size(
                esp_idf_sys::MALLOC_CAP_SPIRAM
            ) as u32;
            let end_stack_watermark = esp_idf_sys::uxTaskGetStackHighWaterMark(
                std::ptr::null_mut()
            );
            
            let heap_delta = self.start_heap_free as i32 - end_heap_free as i32;
            let psram_delta = self.start_psram_free as i32 - end_psram_free as i32;
            let largest_block_delta = self.start_heap_largest as i32 - end_heap_largest as i32;
            let stack_used = self.start_stack_watermark - end_stack_watermark;
            
            if let Some(ref id) = self.request_id {
                log::info!(
                    "[{}] {} {} - {}ms heap:{} psram:{} largest_blk:{} stack_used:{}",
                    id, path, status, duration_ms, heap_delta, psram_delta, largest_block_delta, stack_used
                );
            } else {
                log::info!(
                    "{} {} - {}ms heap:{} psram:{} largest_blk:{} stack_used:{}",
                    path, status, duration_ms, heap_delta, psram_delta, largest_block_delta, stack_used
                );
            }
            
            // Warn if heap is getting fragmented
            if end_heap_largest < 8192 {
                log::warn!("Heap fragmentation detected! Largest block: {}", end_heap_largest);
            }
            
            // Warn if stack is getting low
            if end_stack_watermark < 1024 {
                log::warn!("Low stack watermark: {} bytes", end_stack_watermark);
            }
        }
    }
}