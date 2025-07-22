/// Enhanced fix for 6-block readable pattern - Version 2
use anyhow::Result;
use esp_idf_sys::*;
use log::{info, warn};

/// Apply more aggressive fixes for the 6-block pattern issue
pub unsafe fn apply_6block_fix_v2(
    bus_config: &mut esp_lcd_i80_bus_config_t,
    io_config: &mut esp_lcd_panel_io_i80_config_t,
) -> Result<()> {
    info!("=== Applying Enhanced 6-Block Pattern Fix V2 ===");
    
    // Fix 1: Even slower clock speed - try 2 MHz
    let original_clock = io_config.pclk_hz;
    io_config.pclk_hz = 2_000_000; // 2 MHz - very slow
    warn!("  AGGRESSIVE: Reduced clock from {} to {} Hz", original_clock, io_config.pclk_hz);
    
    // Fix 2: Ensure transfer size is exactly one line
    // 170 pixels * 2 bytes = 340 bytes per line
    // Round up to next multiple of 64 for cache alignment
    let line_bytes = 170 * 2; // 340 bytes
    let aligned_line = ((line_bytes + 63) / 64) * 64; // 384 bytes
    
    bus_config.max_transfer_bytes = aligned_line;
    warn!("  Set transfer size to exactly {} bytes (one line)", aligned_line);
    
    // Fix 3: Force synchronous single-line transfers
    io_config.trans_queue_depth = 1;
    
    // Fix 4: Try different DC timing levels
    // Some displays need inverted DC levels
    io_config.dc_levels._bitfield_1 = esp_lcd_panel_io_i80_config_t__bindgen_ty_1::new_bitfield_1(
        1, // dc_idle_level - try HIGH when idle
        0, // dc_cmd_level - LOW for commands
        0, // dc_dummy_level
        1, // dc_data_level - HIGH for data
    );
    warn!("  Modified DC signal levels for better compatibility");
    
    // Fix 5: Adjust bus width confirmation
    // Ensure we're really in 8-bit mode
    if bus_config.bus_width != 8 {
        warn!("  WARNING: Bus width was {}, forcing to 8", bus_config.bus_width);
        bus_config.bus_width = 8;
    }
    
    // Fix 6: Try different pixel clock polarity
    io_config.flags._bitfield_1 = esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
        0, // cs_active_high
        0, // reverse_color_bits
        1, // swap_color_bytes - keep this for RGB565
        1, // pclk_active_neg - try negative edge
        0, // pclk_idle_low
    );
    warn!("  Set pixel clock to negative edge");
    
    // Fix 7: Ensure proper alignment for all transfers
    bus_config.__bindgen_anon_1.psram_trans_align = 64;
    bus_config.sram_trans_align = 8; // Try 8-byte alignment
    
    info!("=== Enhanced 6-Block Fix V2 Applied ===");
    info!("Key changes:");
    info!("- Clock: 2 MHz (very slow)");
    info!("- Transfer: {} bytes (one line at a time)", aligned_line);
    info!("- DC levels: Modified for compatibility");
    info!("- Pixel clock: Negative edge");
    info!("- Alignment: 8-byte SRAM, 64-byte PSRAM");
    
    Ok(())
}

/// Test pattern specifically for debugging 6-block issue
pub unsafe fn draw_6block_test_pattern(
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== Drawing 6-Block Debug Pattern ===");
    
    // Pattern 1: Solid color test
    info!("Test 1: Solid colors (should fill entire screen)");
    
    // Red screen
    let red_screen = vec![0xF800u16; 320 * 170];
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        red_screen.as_ptr() as *const _,
    );
    esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    
    // Green screen
    let green_screen = vec![0x07E0u16; 320 * 170];
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        green_screen.as_ptr() as *const _,
    );
    esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    
    // Blue screen
    let blue_screen = vec![0x001Fu16; 320 * 170];
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        blue_screen.as_ptr() as *const _,
    );
    esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    
    // Pattern 2: Gradient to show 6-block boundaries
    info!("Test 2: Gradient pattern");
    let mut gradient = vec![0u16; 320 * 170];
    for y in 0..170 {
        for x in 0..320 {
            let idx = y * 320 + x;
            // Create gradient that changes every pixel
            let intensity = ((x * 31) / 320) as u16;
            gradient[idx] = intensity << 11; // Red gradient
        }
    }
    
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        gradient.as_ptr() as *const _,
    );
    esp_idf_hal::delay::FreeRtos::delay_ms(2000);
    
    // Pattern 3: Text overlay
    info!("Test 3: Version text");
    
    // Clear to black first
    let black = vec![0x0000u16; 320 * 170];
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        black.as_ptr() as *const _,
    );
    
    // Draw "v5.40-6blkfix" in large blocks
    // This is a simple version - just draws rectangles where text would be
    let text_color = 0xFFFF; // White
    
    // Draw 'v' shape
    for y in 0..20 {
        let x = y / 2;
        draw_rect(panel_handle, 10 + x, 10 + y, 4, 4, text_color)?;
        draw_rect(panel_handle, 30 - x, 10 + y, 4, 4, text_color)?;
    }
    
    // Draw version number area
    draw_rect(panel_handle, 50, 10, 100, 30, text_color)?;
    
    info!("=== Test Pattern Complete ===");
    info!("You should see:");
    info!("1. Full screen colors (red, green, blue)");
    info!("2. Smooth gradient");
    info!("3. Version text");
    info!("If you only see every 6th pixel/block, the issue persists");
    
    Ok(())
}

/// Helper to draw a filled rectangle
unsafe fn draw_rect(
    panel_handle: esp_lcd_panel_handle_t,
    x: i32, y: i32, w: i32, h: i32,
    color: u16,
) -> Result<()> {
    let pixels = vec![color; (w * h) as usize];
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        x, y,
        x + w, y + h,
        pixels.as_ptr() as *const _,
    );
    Ok(())
}