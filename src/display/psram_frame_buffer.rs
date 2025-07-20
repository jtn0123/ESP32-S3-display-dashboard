// PSRAM-backed frame buffer for optimized display updates
// Implements double buffering with differential updates

use super::{DISPLAY_WIDTH, DISPLAY_HEIGHT};
use crate::psram::PsramAllocator;
use esp_idf_sys::*;
use log::*;
use std::ptr;

const BUFFER_SIZE: usize = DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize;

/// PSRAM-backed double buffer for display
pub struct PsramFrameBuffer {
    // Two frame buffers for double buffering
    front_buffer: *mut u16,
    back_buffer: *mut u16,
    // Track which buffer is currently being displayed
    current_front: bool,
    // Dimensions
    width: usize,
    height: usize,
    // Statistics
    total_pixels: usize,
    pixels_changed: usize,
}

impl PsramFrameBuffer {
    /// Create a new PSRAM frame buffer
    pub fn new(width: u16, height: u16) -> Result<Self, &'static str> {
        if !PsramAllocator::is_available() {
            return Err("PSRAM not available");
        }
        
        let buffer_size = (width as usize) * (height as usize) * std::mem::size_of::<u16>();
        let total_size = buffer_size * 2; // Two buffers
        
        // Check if we have enough PSRAM
        if PsramAllocator::get_free_size() < total_size {
            return Err("Insufficient PSRAM for frame buffers");
        }
        
        // Allocate front buffer with 16-byte alignment for better performance
        let front_buffer = unsafe {
            heap_caps_aligned_alloc(16, buffer_size, MALLOC_CAP_SPIRAM) as *mut u16
        };
        
        if front_buffer.is_null() {
            return Err("Failed to allocate front buffer in PSRAM");
        }
        
        // Allocate back buffer
        let back_buffer = unsafe {
            heap_caps_aligned_alloc(16, buffer_size, MALLOC_CAP_SPIRAM) as *mut u16
        };
        
        if back_buffer.is_null() {
            unsafe { heap_caps_free(front_buffer as *mut _); }
            return Err("Failed to allocate back buffer in PSRAM");
        }
        
        // Initialize both buffers to black for now
        unsafe {
            let buffer_size = (width as usize) * (height as usize);
            for i in 0..buffer_size {
                *front_buffer.add(i) = 0x0000; // Black
                *back_buffer.add(i) = 0x0000;
            }
        }
        
        info!("Created PSRAM frame buffer: {}x{} ({} KB total)", 
            width, height, total_size / 1024);
        info!("Front buffer: {:p}, Back buffer: {:p}", front_buffer, back_buffer);
        
        Ok(Self {
            front_buffer,
            back_buffer,
            current_front: true,
            width: width as usize,
            height: height as usize,
            total_pixels: (width as usize) * (height as usize),
            pixels_changed: 0,
        })
    }
    
    /// Get the current back buffer for drawing
    pub fn get_draw_buffer(&mut self) -> &mut [u16] {
        let buffer = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        unsafe {
            std::slice::from_raw_parts_mut(buffer, self.total_pixels)
        }
    }
    
    /// Set a pixel in the back buffer
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) {
        if (x as usize) >= self.width || (y as usize) >= self.height {
            return;
        }
        
        let index = (y as usize) * self.width + (x as usize);
        let buffer = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        unsafe {
            *buffer.add(index) = color;
        }
    }
    
    /// Fill a rectangle in the back buffer
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        let x_end = ((x as usize) + (w as usize)).min(self.width);
        let y_end = ((y as usize) + (h as usize)).min(self.height);
        
        let buffer = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        for row in (y as usize)..y_end {
            let start_idx = row * self.width + (x as usize);
            let count = x_end - (x as usize);
            
            unsafe {
                // Use optimized fill for better performance
                let row_ptr = buffer.add(start_idx);
                for i in 0..count {
                    *row_ptr.add(i) = color;
                }
            }
        }
    }
    
    /// Clear the entire back buffer
    pub fn clear(&mut self, color: u16) {
        let buffer = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        // Always use proper color filling to avoid endianness issues
        unsafe {
            let buffer_slice = std::slice::from_raw_parts_mut(buffer, self.total_pixels);
            for pixel in buffer_slice.iter_mut() {
                *pixel = color;
            }
        }
        
        debug!("Cleared buffer to color 0x{:04x}", color);
    }
    
    /// Compare buffers and get regions that changed
    pub fn get_dirty_regions(&mut self) -> Vec<DirtyRegion> {
        let front = if self.current_front {
            self.front_buffer
        } else {
            self.back_buffer
        };
        
        let back = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        let mut dirty_regions: Vec<DirtyRegion> = Vec::new();
        self.pixels_changed = 0;
        
        // Simple block-based dirty detection (16x16 blocks)
        const BLOCK_SIZE: usize = 16;
        let blocks_x = (self.width + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let blocks_y = (self.height + BLOCK_SIZE - 1) / BLOCK_SIZE;
        
        for by in 0..blocks_y {
            for bx in 0..blocks_x {
                let mut block_dirty = false;
                let x_start = bx * BLOCK_SIZE;
                let y_start = by * BLOCK_SIZE;
                let x_end = (x_start + BLOCK_SIZE).min(self.width);
                let y_end = (y_start + BLOCK_SIZE).min(self.height);
                
                // Check if any pixel in this block changed
                'outer: for y in y_start..y_end {
                    let row_offset = y * self.width;
                    for x in x_start..x_end {
                        let idx = row_offset + x;
                        unsafe {
                            if *front.add(idx) != *back.add(idx) {
                                block_dirty = true;
                                self.pixels_changed += 1;
                                break 'outer;
                            }
                        }
                    }
                }
                
                if block_dirty {
                    // Try to merge with adjacent dirty regions
                    let new_region = DirtyRegion {
                        x: x_start as u16,
                        y: y_start as u16,
                        width: (x_end - x_start) as u16,
                        height: (y_end - y_start) as u16,
                    };
                    
                    let mut merged = false;
                    for region in &mut dirty_regions {
                        if region.can_merge_with(&new_region) {
                            region.merge_with(&new_region);
                            merged = true;
                            break;
                        }
                    }
                    
                    if !merged {
                        dirty_regions.push(new_region);
                    }
                }
            }
        }
        
        // Further merge overlapping regions
        self.merge_dirty_regions(&mut dirty_regions);
        
        if !dirty_regions.is_empty() {
            debug!("Found {} dirty regions covering {} pixels", 
                dirty_regions.len(), self.pixels_changed);
        }
        
        dirty_regions
    }
    
    /// Merge overlapping dirty regions
    fn merge_dirty_regions(&self, regions: &mut Vec<DirtyRegion>) {
        let mut changed = true;
        while changed {
            changed = false;
            
            for i in 0..regions.len() {
                for j in (i + 1)..regions.len() {
                    if regions[i].overlaps_with(&regions[j]) {
                        let merged = regions[i].merge_with(&regions[j]);
                        regions[i] = merged;
                        regions.remove(j);
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }
        }
    }
    
    /// Swap front and back buffers
    pub fn swap_buffers(&mut self) {
        self.current_front = !self.current_front;
        
        // Force cache writeback for PSRAM coherency
        // Use memory barrier to ensure writes are visible
        unsafe {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            // Also try the ESP-IDF cache writeback if available
            // Note: This function might not be in our bindings
            #[cfg(feature = "esp32s3")]
            {
                extern "C" {
                    fn Cache_WriteBack_All() -> i32;
                }
                Cache_WriteBack_All();
            }
        }
    }
    
    /// Get pixels from a specific region of the back buffer (the one we just drew to)
    pub fn get_region(&self, x: u16, y: u16, width: u16, height: u16) -> Vec<u16> {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        
        // Get from the back buffer (the one we just drew to)
        let buffer = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        for row in 0..height {
            let y_pos = y + row;
            if (y_pos as usize) >= self.height {
                break;
            }
            
            for col in 0..width {
                let x_pos = x + col;
                if (x_pos as usize) >= self.width {
                    break;
                }
                
                let idx = (y_pos as usize) * self.width + (x_pos as usize);
                unsafe {
                    pixels.push(*buffer.add(idx));
                }
            }
        }
        
        pixels
    }
    
    /// Get the entire back buffer for full screen update (debugging)
    pub fn get_full_buffer(&self) -> &[u16] {
        let buffer = if self.current_front {
            self.back_buffer
        } else {
            self.front_buffer
        };
        
        unsafe {
            std::slice::from_raw_parts(buffer, self.total_pixels)
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> (usize, f32) {
        let change_percent = if self.total_pixels > 0 {
            (self.pixels_changed as f32 / self.total_pixels as f32) * 100.0
        } else {
            0.0
        };
        
        (self.pixels_changed, change_percent)
    }
}

impl Drop for PsramFrameBuffer {
    fn drop(&mut self) {
        unsafe {
            heap_caps_free(self.front_buffer as *mut _);
            heap_caps_free(self.back_buffer as *mut _);
        }
        info!("Freed PSRAM frame buffers");
    }
}

/// Represents a dirty region that needs updating
#[derive(Debug, Clone, Copy)]
pub struct DirtyRegion {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl DirtyRegion {
    /// Check if this region can be merged with another
    fn can_merge_with(&self, other: &DirtyRegion) -> bool {
        // Check if regions are adjacent or overlapping
        let self_right = self.x + self.width;
        let self_bottom = self.y + self.height;
        let other_right = other.x + other.width;
        let other_bottom = other.y + other.height;
        
        // Adjacent horizontally
        if self.y == other.y && self.height == other.height {
            if self_right == other.x || other_right == self.x {
                return true;
            }
        }
        
        // Adjacent vertically
        if self.x == other.x && self.width == other.width {
            if self_bottom == other.y || other_bottom == self.y {
                return true;
            }
        }
        
        // Overlapping
        self.overlaps_with(other)
    }
    
    /// Check if this region overlaps with another
    fn overlaps_with(&self, other: &DirtyRegion) -> bool {
        let self_right = self.x + self.width;
        let self_bottom = self.y + self.height;
        let other_right = other.x + other.width;
        let other_bottom = other.y + other.height;
        
        !(self.x >= other_right || other.x >= self_right ||
          self.y >= other_bottom || other.y >= self_bottom)
    }
    
    /// Merge this region with another
    fn merge_with(&self, other: &DirtyRegion) -> DirtyRegion {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        
        DirtyRegion {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }
    }
}