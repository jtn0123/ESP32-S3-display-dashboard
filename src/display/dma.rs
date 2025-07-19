// DMA-accelerated display driver with frame buffer
// Uses PSRAM for frame buffer to enable fast parallel transfers

use esp_idf_sys::*;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use log::*;
use anyhow::Result;
use crate::psram::PsramAllocator;

const DISPLAY_WIDTH: usize = 320;
const DISPLAY_HEIGHT: usize = 170; 
const FRAMEBUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;
const BYTES_PER_PIXEL: usize = 2; // RGB565

/// High-performance frame buffer using PSRAM
pub struct DmaDisplay {
    framebuffer: *mut u16,
    shadow_buffer: Option<*mut u16>, // Secondary buffer for double buffering
    transfer_active: AtomicBool,
    frames_rendered: AtomicU32,
    use_psram: bool,
}

impl DmaDisplay {
    pub unsafe fn new() -> Result<Self> {
        info!("Initializing DMA-optimized display driver");
        
        // Allocate framebuffer in PSRAM if available
        let (framebuffer, use_psram) = if PsramAllocator::is_available() {
            let size = FRAMEBUFFER_SIZE * BYTES_PER_PIXEL;
            let ptr = heap_caps_malloc(size, MALLOC_CAP_SPIRAM) as *mut u16;
            if ptr.is_null() {
                error!("Failed to allocate PSRAM framebuffer, falling back to internal RAM");
                let ptr = heap_caps_malloc(size, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL) as *mut u16;
                (ptr, false)
            } else {
                info!("Allocated {}KB framebuffer in PSRAM", size / 1024);
                (ptr, true)
            }
        } else {
            let size = FRAMEBUFFER_SIZE * BYTES_PER_PIXEL;
            let ptr = heap_caps_malloc(size, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL) as *mut u16;
            (ptr, false)
        };
        
        if framebuffer.is_null() {
            return Err(anyhow::anyhow!("Failed to allocate framebuffer"));
        }
        
        // Clear framebuffer
        ptr::write_bytes(framebuffer, 0, FRAMEBUFFER_SIZE);
        
        // Optionally allocate shadow buffer for double buffering
        let shadow_buffer = if use_psram && PsramAllocator::get_free_size() > FRAMEBUFFER_SIZE * BYTES_PER_PIXEL * 2 {
            let size = FRAMEBUFFER_SIZE * BYTES_PER_PIXEL;
            let ptr = heap_caps_malloc(size, MALLOC_CAP_SPIRAM) as *mut u16;
            if !ptr.is_null() {
                info!("Allocated shadow buffer for double buffering");
                Some(ptr)
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Self {
            framebuffer,
            shadow_buffer,
            transfer_active: AtomicBool::new(false),
            frames_rendered: AtomicU32::new(0),
            use_psram,
        })
    }
    
    pub fn get_framebuffer(&self) -> &mut [u16] {
        unsafe {
            core::slice::from_raw_parts_mut(self.framebuffer, FRAMEBUFFER_SIZE)
        }
    }
    
    pub fn get_shadow_buffer(&self) -> Option<&mut [u16]> {
        self.shadow_buffer.map(|ptr| unsafe {
            core::slice::from_raw_parts_mut(ptr, FRAMEBUFFER_SIZE)
        })
    }
    
    pub fn swap_buffers(&mut self) {
        if let Some(_shadow) = self.shadow_buffer {
            core::mem::swap(&mut self.framebuffer, &mut self.shadow_buffer.unwrap());
        }
    }
    
    pub fn start_transfer(&self) -> Result<()> {
        if self.transfer_active.load(Ordering::Acquire) {
            return Ok(()); // Transfer already in progress
        }
        
        self.transfer_active.store(true, Ordering::Release);
        self.frames_rendered.fetch_add(1, Ordering::Relaxed);
        
        // In a real implementation, this would trigger actual DMA transfer
        // For now, we just mark it as complete immediately
        self.transfer_active.store(false, Ordering::Release);
        
        Ok(())
    }
    
    pub fn wait_transfer(&self) {
        while self.transfer_active.load(Ordering::Acquire) {
            unsafe { vTaskDelay(1); }
        }
    }
    
    pub fn is_transfer_active(&self) -> bool {
        self.transfer_active.load(Ordering::Acquire)
    }
    
    pub fn get_frames_rendered(&self) -> u32 {
        self.frames_rendered.load(Ordering::Relaxed)
    }
    
    pub fn is_using_psram(&self) -> bool {
        self.use_psram
    }
    
    /// Convert RGB888 to RGB565
    pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
        ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
    }
    
    /// Fill a rectangle in the framebuffer
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        let fb = self.get_framebuffer();
        
        // Optimize for full-width rectangles
        if x == 0 && w as usize == DISPLAY_WIDTH {
            let start_idx = y as usize * DISPLAY_WIDTH;
            let end_idx = ((y + h) as usize * DISPLAY_WIDTH).min(FRAMEBUFFER_SIZE);
            for idx in start_idx..end_idx {
                fb[idx] = color;
            }
        } else {
            // General rectangle fill
            for dy in 0..h {
                let y_pos = (y + dy) as usize;
                if y_pos >= DISPLAY_HEIGHT { break; }
                
                let row_start = y_pos * DISPLAY_WIDTH + x as usize;
                let row_end = (row_start + w as usize).min(y_pos * DISPLAY_WIDTH + DISPLAY_WIDTH);
                
                for idx in row_start..row_end {
                    if idx < FRAMEBUFFER_SIZE {
                        fb[idx] = color;
                    }
                }
            }
        }
    }
    
    /// Draw a pixel in the framebuffer
    #[inline(always)]
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) {
        if x as usize >= DISPLAY_WIDTH || y as usize >= DISPLAY_HEIGHT {
            return;
        }
        
        let idx = y as usize * DISPLAY_WIDTH + x as usize;
        self.get_framebuffer()[idx] = color;
    }
    
    /// Clear the entire framebuffer
    pub fn clear(&mut self, color: u16) {
        let fb = self.get_framebuffer();
        // Use optimized fill for better performance
        unsafe {
            let ptr = fb.as_mut_ptr();
            for i in 0..FRAMEBUFFER_SIZE {
                ptr.add(i).write(color);
            }
        }
    }
    
    /// Copy framebuffer data to display buffer
    pub fn copy_to_display(&self, display_buffer: &mut [u8]) -> Result<()> {
        if display_buffer.len() < FRAMEBUFFER_SIZE * BYTES_PER_PIXEL {
            return Err(anyhow::anyhow!("Display buffer too small"));
        }
        
        unsafe {
            ptr::copy_nonoverlapping(
                self.framebuffer as *const u8,
                display_buffer.as_mut_ptr(),
                FRAMEBUFFER_SIZE * BYTES_PER_PIXEL
            );
        }
        
        Ok(())
    }
}

impl Drop for DmaDisplay {
    fn drop(&mut self) {
        unsafe {
            // Free allocated memory
            if !self.framebuffer.is_null() {
                heap_caps_free(self.framebuffer as *mut _);
            }
            
            if let Some(shadow) = self.shadow_buffer {
                if !shadow.is_null() {
                    heap_caps_free(shadow as *mut _);
                }
            }
        }
    }
}

// Safe wrapper for using DMA display
pub struct DmaDisplayWrapper {
    inner: Option<DmaDisplay>,
}

impl DmaDisplayWrapper {
    pub fn new() -> Result<Self> {
        unsafe {
            match DmaDisplay::new() {
                Ok(display) => Ok(Self { inner: Some(display) }),
                Err(e) => {
                    warn!("DMA display initialization failed: {}", e);
                    Ok(Self { inner: None })
                }
            }
        }
    }
    
    pub fn is_available(&self) -> bool {
        self.inner.is_some()
    }
    
    pub fn with_display<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut DmaDisplay) -> R,
    {
        self.inner.as_mut().map(f)
    }
}