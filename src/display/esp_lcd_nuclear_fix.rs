/// Nuclear fix - try everything to fix the display
use anyhow::Result;
use esp_idf_sys::*;
use log::{info, error};

pub unsafe fn apply_nuclear_fix(
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    error!("=== APPLYING NUCLEAR DISPLAY FIX ===");
    
    // Test 1: Direct command mode test
    info!("Test 1: Sending direct test commands");
    
    // Try to reset the display controller
    esp_lcd_panel_reset(panel_handle);
    esp_idf_hal::delay::FreeRtos::delay_ms(100);
    
    // Re-initialize
    esp_lcd_panel_init(panel_handle);
    esp_idf_hal::delay::FreeRtos::delay_ms(100);
    
    // Set gap again
    esp_lcd_panel_set_gap(panel_handle, 0, 35);
    esp_idf_hal::delay::FreeRtos::delay_ms(10);
    
    // Try different orientations
    info!("Test 2: Testing different orientations");
    
    // Normal orientation
    esp_lcd_panel_swap_xy(panel_handle, false);
    esp_lcd_panel_mirror(panel_handle, false, false);
    draw_test_square(panel_handle, 0xFF00)?; // Red
    esp_idf_hal::delay::FreeRtos::delay_ms(500);
    
    // Landscape (what we want)
    esp_lcd_panel_swap_xy(panel_handle, true);
    esp_lcd_panel_mirror(panel_handle, false, true);
    draw_test_square(panel_handle, 0x07E0)?; // Green
    esp_idf_hal::delay::FreeRtos::delay_ms(500);
    
    // Test 3: Different pixel formats
    info!("Test 3: Drawing with different methods");
    
    // Method 1: Full screen at once
    let full_buffer = vec![0x001Fu16; 320 * 170]; // Blue
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        full_buffer.as_ptr() as *const _,
    );
    esp_idf_hal::delay::FreeRtos::delay_ms(500);
    
    // Method 2: Line by line
    for y in 0..170 {
        let line_color = if y % 2 == 0 { 0xFFFF } else { 0x0000 };
        let line = vec![line_color; 320];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, y as i32,
            320, (y + 1) as i32,
            line.as_ptr() as *const _,
        );
    }
    esp_idf_hal::delay::FreeRtos::delay_ms(500);
    
    // Method 3: Small blocks
    for y in (0..170).step_by(10) {
        for x in (0..320).step_by(10) {
            let color = ((x + y) / 10 * 1000) as u16;
            let block = vec![color; 100];
            esp_lcd_panel_draw_bitmap(
                panel_handle,
                x as i32, y as i32,
                (x + 10) as i32, (y + 10) as i32,
                block.as_ptr() as *const _,
            );
        }
    }
    
    error!("=== NUCLEAR FIX COMPLETE ===");
    error!("If display is still broken after this, it's likely:");
    error!("1. Hardware connection issue");
    error!("2. Power supply problem");
    error!("3. Display controller incompatibility");
    
    Ok(())
}

fn draw_test_square(panel_handle: esp_lcd_panel_handle_t, color: u16) -> Result<()> {
    unsafe {
        // Draw a 100x100 square in the center
        let square = vec![color; 100 * 100];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            110, 35,
            210, 135,
            square.as_ptr() as *const _,
        );
    }
    Ok(())
}

/// Last resort - try raw I80 commands
pub unsafe fn try_raw_i80_fix(
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    error!("=== TRYING RAW I80 COMMANDS ===");
    
    // Send SLPOUT command directly
    esp_lcd_panel_io_tx_param(io_handle, 0x11, core::ptr::null(), 0);
    esp_idf_hal::delay::FreeRtos::delay_ms(120);
    
    // Send DISPON command
    esp_lcd_panel_io_tx_param(io_handle, 0x29, core::ptr::null(), 0);
    esp_idf_hal::delay::FreeRtos::delay_ms(20);
    
    // Try to write a test pattern directly
    // Set window to small area
    let caset: [u8; 4] = [0, 0, 0, 100];
    esp_lcd_panel_io_tx_param(io_handle, 0x2A, caset.as_ptr() as *const _, 4);
    
    let raset: [u8; 4] = [0, 35, 0, 135];  
    esp_lcd_panel_io_tx_param(io_handle, 0x2B, raset.as_ptr() as *const _, 4);
    
    // Send RAMWR
    esp_lcd_panel_io_tx_param(io_handle, 0x2C, core::ptr::null(), 0);
    
    // Send color data - alternating red and blue
    let mut test_data = Vec::new();
    for i in 0..10000 {
        if i % 2 == 0 {
            test_data.push(0xF8); // Red high
            test_data.push(0x00); // Red low
        } else {
            test_data.push(0x00); // Blue high
            test_data.push(0x1F); // Blue low
        }
    }
    
    esp_lcd_panel_io_tx_color(
        io_handle,
        -1,
        test_data.as_ptr() as *const _,
        test_data.len(),
    );
    
    error!("Raw commands sent - check display");
    
    Ok(())
}