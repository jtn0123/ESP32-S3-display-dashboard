// Debug test to figure out why ESP LCD isn't showing anything
use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;
use super::debug_trace::{traced_lcd_panel_io_tx_param, traced_lcd_panel_io_tx_color};

pub fn debug_esp_lcd_raw_commands(panel_io: esp_lcd_panel_io_handle_t) -> Result<()> {
    info!("[DEBUG] Starting raw command test...");
    
    unsafe {
        // Software reset
        info!("[DEBUG] Sending SWRESET...");
        traced_lcd_panel_io_tx_param(panel_io, 0x01, core::ptr::null(), 0);
        Ets::delay_ms(150);
        
        // Sleep out
        info!("[DEBUG] Sending SLPOUT...");
        traced_lcd_panel_io_tx_param(panel_io, 0x11, core::ptr::null(), 0);
        Ets::delay_ms(120);
        
        // MADCTL - Memory Access Control
        info!("[DEBUG] Sending MADCTL (0x60 for landscape)...");
        let madctl: [u8; 1] = [0x60];
        traced_lcd_panel_io_tx_param(panel_io, 0x36, madctl.as_ptr() as *const _, 1);
        
        // COLMOD - Interface Pixel Format
        info!("[DEBUG] Sending COLMOD (16-bit)...");
        let colmod: [u8; 1] = [0x55];
        traced_lcd_panel_io_tx_param(panel_io, 0x3A, colmod.as_ptr() as *const _, 1);
        
        // Inversion on
        info!("[DEBUG] Sending INVON...");
        traced_lcd_panel_io_tx_param(panel_io, 0x21, core::ptr::null(), 0);
        
        // Normal display mode on
        info!("[DEBUG] Sending NORON...");
        traced_lcd_panel_io_tx_param(panel_io, 0x13, core::ptr::null(), 0);
        
        // Display on
        info!("[DEBUG] Sending DISPON...");
        traced_lcd_panel_io_tx_param(panel_io, 0x29, core::ptr::null(), 0);
        Ets::delay_ms(100);
        
        // Try to fill screen with red
        info!("[DEBUG] Setting up window for full screen...");
        
        // CASET - columns 0-319 (landscape)
        let caset: [u8; 4] = [0x00, 0x00, 0x01, 0x3F]; // 0-319
        traced_lcd_panel_io_tx_param(panel_io, 0x2A, caset.as_ptr() as *const _, 4);
        
        // RASET - rows 35-204 (170 rows starting at offset 35)
        let raset: [u8; 4] = [0x00, 0x23, 0x00, 0xCC]; // 35-204
        traced_lcd_panel_io_tx_param(panel_io, 0x2B, raset.as_ptr() as *const _, 4);
        
        // RAMWR - Memory Write
        info!("[DEBUG] Starting RAMWR...");
        traced_lcd_panel_io_tx_param(panel_io, 0x2C, core::ptr::null(), 0);
        
        // Send red pixels (RGB565: 0xF800)
        info!("[DEBUG] Sending red pixel data...");
        let red_pixels = vec![0xF8, 0x00]; // Red in RGB565 big endian
        for _ in 0..1000 {
            traced_lcd_panel_io_tx_color(
                panel_io, 
                u32::MAX,  // Continue previous command (-1 as u32)
                red_pixels.as_ptr() as *const _,
                red_pixels.len()
            );
        }
        
        info!("[DEBUG] Raw command test complete!");
    }
    
    Ok(())
}