// PSRAM (External SPI RAM) management for ESP32-S3
// The T-Display-S3 has 8MB of PSRAM for extended memory

use esp_idf_sys::*;
use log::*;

/// PSRAM memory allocator that prefers external memory for large allocations
pub struct PsramAllocator;


impl PsramAllocator {
    /// Check if PSRAM is available and initialized
    pub fn is_available() -> bool {
        unsafe { esp_psram_is_initialized() }
    }
    
    /// Get total PSRAM size in bytes
    pub fn get_size() -> usize {
        unsafe { esp_psram_get_size() }
    }
    
    /// Get free PSRAM in bytes
    pub fn get_free_size() -> usize {
        unsafe { heap_caps_get_free_size(MALLOC_CAP_SPIRAM) }
    }
    
    /// Get largest free PSRAM block
    pub fn get_largest_free_block() -> usize {
        unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_SPIRAM) }
    }
    
    
    /// Get memory info for diagnostics
    pub fn get_info() -> PsramInfo {
        PsramInfo {
            available: Self::is_available(),
            total_size: Self::get_size(),
            free_size: Self::get_free_size(),
            largest_block: Self::get_largest_free_block(),
            internal_free: unsafe { heap_caps_get_free_size(MALLOC_CAP_INTERNAL) },
            internal_largest: unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL) },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PsramInfo {
    pub available: bool,
    pub total_size: usize,
    pub free_size: usize,
    pub largest_block: usize,
    pub internal_free: usize,
    pub internal_largest: usize,
}

impl PsramInfo {
    pub fn log_info(&self) {
        if self.available {
            info!("PSRAM Status: Available");
            info!("  Total: {} MB", self.total_size / 1024 / 1024);
            info!("  Free: {} KB", self.free_size / 1024);
            info!("  Largest block: {} KB", self.largest_block / 1024);
            info!("Internal RAM:");
            info!("  Free: {} KB", self.internal_free / 1024);
            info!("  Largest block: {} KB", self.internal_largest / 1024);
        } else {
            warn!("PSRAM Status: Not available");
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_psram_detection() {
        let info = PsramAllocator::get_info();
        println!("PSRAM info: {:?}", info);
        
        if info.available {
            assert!(info.total_size > 0);
            assert!(info.free_size > 0);
        }
    }
}