/// Clean, reusable diagnostic probes for performance analysis
/// Only compiled when "diag" feature is enabled
#[cfg(feature = "diag")]
pub mod probes {
    use esp_idf_sys::*;
    use log::{info, warn, error};
    
    /// Log heap statistics with a descriptive tag
    pub fn heap(tag: &str) {
        unsafe {
            let free = esp_get_free_heap_size();
            let min = esp_get_minimum_free_heap_size();
            let internal = heap_caps_get_free_size(MALLOC_CAP_INTERNAL);
            let dma = heap_caps_get_free_size(MALLOC_CAP_DMA);
            
            info!("HEAP[{}]: free={} min={} internal={} dma={}", 
                  tag, free, min, internal, dma);
        }
    }
    
    /// Log which core is executing with a descriptive tag
    pub fn core(tag: &str) {
        unsafe {
            info!("CORE[{}]: running on core {}", tag, xTaskGetCoreID(std::ptr::null_mut()));
        }
    }
    
    /// Start a timing measurement, returns timestamp
    pub fn timing_start() -> u64 {
        unsafe { esp_timer_get_time() as u64 }
    }
    
    /// End timing measurement and log results
    /// Warns if >10ms, errors if >50ms
    pub fn timing_end(tag: &str, start: u64) {
        unsafe {
            let elapsed = (esp_timer_get_time() as u64) - start;
            
            if elapsed > 50_000 {  // >50ms is critical
                error!("TIMING[{}]: CRITICAL {}µs ({}ms)", tag, elapsed, elapsed / 1000);
            } else if elapsed > 10_000 {  // >10ms is warning
                warn!("TIMING[{}]: SLOW {}µs ({}ms)", tag, elapsed, elapsed / 1000);
            } else {
                info!("TIMING[{}]: {}µs", tag, elapsed);
            }
        }
    }
    
    /// Log binary size sections (call once at boot)
    pub fn binary_size() {
        info!("BINARY: Check size with: xtensa-esp32s3-elf-size -A target/.../esp32-s3-dashboard");
    }
    
    /// Track heap delta between two points
    pub struct HeapTracker {
        tag: String,
        start_free: u32,
        start_min: u32,
    }
    
    impl HeapTracker {
        pub fn new(tag: &str) -> Self {
            unsafe {
                let start_free = esp_get_free_heap_size();
                let start_min = esp_get_minimum_free_heap_size();
                info!("HEAP_TRACK[{}]: start free={} min={}", tag, start_free, start_min);
                
                Self {
                    tag: tag.to_string(),
                    start_free,
                    start_min,
                }
            }
        }
        
        pub fn end(self) {
            unsafe {
                let end_free = esp_get_free_heap_size();
                let end_min = esp_get_minimum_free_heap_size();
                
                let delta_free = self.start_free as i32 - end_free as i32;
                let delta_min = self.start_min as i32 - end_min as i32;
                
                if delta_free > 20_000 {  // >20KB drop
                    error!("HEAP_TRACK[{}]: LARGE DROP free_delta={} min_delta={}", 
                           self.tag, delta_free, delta_min);
                } else if delta_free > 5_000 {  // >5KB drop
                    warn!("HEAP_TRACK[{}]: free_delta={} min_delta={}", 
                          self.tag, delta_free, delta_min);
                } else {
                    info!("HEAP_TRACK[{}]: free_delta={} min_delta={}", 
                          self.tag, delta_free, delta_min);
                }
            }
        }
    }
}

// Empty module when feature is disabled to avoid compilation errors
#[cfg(not(feature = "diag"))]
pub mod probes {
    pub fn heap(_tag: &str) {}
    pub fn core(_tag: &str) {}
    pub fn timing_start() -> u64 { 0 }
    pub fn timing_end(_tag: &str, _start: u64) {}
    pub fn binary_size() {}
    
    pub struct HeapTracker;
    impl HeapTracker {
        pub fn new(_tag: &str) -> Self { HeapTracker }
        pub fn end(self) {}
    }
}