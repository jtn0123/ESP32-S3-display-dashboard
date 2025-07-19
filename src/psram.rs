// PSRAM (External SPI RAM) management for ESP32-S3
// The T-Display-S3 has 8MB of PSRAM for extended memory

use esp_idf_sys::*;
use std::alloc::Layout;
use std::ptr;
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

/// Smart buffer that automatically uses PSRAM for large allocations
#[allow(dead_code)]
pub struct PsramBuffer<T> {
    data: *mut T,
    len: usize,
    capacity: usize,
    in_psram: bool,
}

impl<T> PsramBuffer<T> {
    /// Create a new buffer with specified capacity
    #[allow(dead_code)]
    pub fn with_capacity(capacity: usize) -> Self {
        let size = capacity * std::mem::size_of::<T>();
        let (data, in_psram) = if size > 1024 && PsramAllocator::is_available() {
            let ptr = unsafe { heap_caps_malloc(size, MALLOC_CAP_SPIRAM) as *mut T };
            if !ptr.is_null() {
                debug!("Allocated {} bytes in PSRAM", size);
                (ptr, true)
            } else {
                warn!("PSRAM allocation failed, falling back to internal RAM");
                let layout = Layout::from_size_align(size, std::mem::align_of::<T>())
                    .expect("Invalid layout");
                (unsafe { std::alloc::alloc(layout) as *mut T }, false)
            }
        } else {
            let layout = Layout::from_size_align(size, std::mem::align_of::<T>())
                .expect("Invalid layout");
            (unsafe { std::alloc::alloc(layout) as *mut T }, false)
        };
        
        Self {
            data,
            len: 0,
            capacity,
            in_psram,
        }
    }
    
    /// Push an element to the buffer
    pub fn push(&mut self, value: T) {
        if self.len >= self.capacity {
            panic!("PsramBuffer capacity exceeded");
        }
        unsafe {
            ptr::write(self.data.add(self.len), value);
        }
        self.len += 1;
    }
    
    /// Get a slice of the buffer
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data, self.len) }
    }
    
    /// Get a mutable slice of the buffer
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.data, self.len) }
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        // Drop all elements
        for i in 0..self.len {
            unsafe {
                ptr::drop_in_place(self.data.add(i));
            }
        }
        self.len = 0;
    }
    
    /// Get the length of the buffer
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if buffer is in PSRAM
    pub fn is_in_psram(&self) -> bool {
        self.in_psram
    }
}

impl<T> Drop for PsramBuffer<T> {
    fn drop(&mut self) {
        // Drop all elements
        self.clear();
        
        // Free the memory
        if self.in_psram {
            unsafe {
                heap_caps_free(self.data as *mut _);
            }
            debug!("Freed {} bytes from PSRAM", self.capacity * std::mem::size_of::<T>());
        } else {
            let layout = Layout::from_size_align(
                self.capacity * std::mem::size_of::<T>(),
                std::mem::align_of::<T>()
            ).expect("Invalid layout");
            unsafe {
                std::alloc::dealloc(self.data as *mut u8, layout);
            }
        }
    }
}

/// Example usage: Large framebuffer in PSRAM
#[allow(dead_code)]
pub fn create_psram_framebuffer(width: usize, height: usize) -> Option<PsramBuffer<u16>> {
    if !PsramAllocator::is_available() {
        warn!("PSRAM not available for framebuffer");
        return None;
    }
    
    let size = width * height;
    let mut buffer = PsramBuffer::with_capacity(size);
    
    // Initialize to black
    for _ in 0..size {
        buffer.push(0);
    }
    
    info!("Created {}x{} framebuffer in PSRAM ({} KB)", 
        width, height, size * 2 / 1024);
    
    Some(buffer)
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
    
    #[test]
    fn test_psram_buffer() {
        let mut buffer: PsramBuffer<u32> = PsramBuffer::with_capacity(1024);
        
        for i in 0..100 {
            buffer.push(i);
        }
        
        assert_eq!(buffer.len(), 100);
        assert_eq!(buffer.as_slice()[0], 0);
        assert_eq!(buffer.as_slice()[99], 99);
    }
}