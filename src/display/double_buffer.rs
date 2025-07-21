/// Double buffering implementation for ESP LCD
use anyhow::Result;
use esp_idf_sys::*;
use core::ptr;
use log::info;

pub struct DoubleBuffer {
    buffer_a: Vec<u16>,
    buffer_b: Vec<u16>,
    active_buffer: u8,
    buffer_size: usize,
    width: usize,
    height: usize,
}

impl DoubleBuffer {
    pub fn new(width: usize, height: usize) -> Result<Self> {
        let buffer_size = width * height;
        
        info!("Allocating double buffer: {}x{} = {} pixels ({} KB)", 
              width, height, buffer_size, buffer_size * 2 / 1024);
        
        // Allocate two buffers
        let buffer_a = vec![0u16; buffer_size];
        let buffer_b = vec![0u16; buffer_size];
        
        Ok(Self {
            buffer_a,
            buffer_b,
            active_buffer: 0,
            buffer_size,
            width,
            height,
        })
    }
    
    /// Get the currently active buffer for drawing
    pub fn active_buffer(&mut self) -> &mut [u16] {
        if self.active_buffer == 0 {
            &mut self.buffer_a
        } else {
            &mut self.buffer_b
        }
    }
    
    /// Get the inactive buffer for DMA transfer
    pub fn inactive_buffer(&self) -> &[u16] {
        if self.active_buffer == 0 {
            &self.buffer_b
        } else {
            &self.buffer_a
        }
    }
    
    /// Swap buffers after DMA completes
    pub fn swap(&mut self) {
        self.active_buffer = 1 - self.active_buffer;
    }
    
    /// Get buffer info
    pub fn info(&self) -> DoubleBufferInfo {
        DoubleBufferInfo {
            width: self.width,
            height: self.height,
            buffer_size: self.buffer_size,
            active_buffer: self.active_buffer,
        }
    }
}

pub struct DoubleBufferInfo {
    pub width: usize,
    pub height: usize,
    pub buffer_size: usize,
    pub active_buffer: u8,
}

/// DMA transfer completion callback context
pub struct DmaContext {
    pub buffer_ready: bool,
    pub transfer_count: u32,
}

/// Callback function for DMA completion
pub unsafe extern "C" fn on_color_trans_done(
    _panel_io: esp_lcd_panel_io_handle_t,
    _event_data: *mut esp_lcd_panel_io_event_data_t,
    user_ctx: *mut core::ffi::c_void,
) -> bool {
    if !user_ctx.is_null() {
        let ctx = &mut *(user_ctx as *mut DmaContext);
        ctx.buffer_ready = true;
        ctx.transfer_count += 1;
    }
    false // Don't need to yield
}