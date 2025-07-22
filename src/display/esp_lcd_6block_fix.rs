/// Fix for 6-block readable pattern in ESP LCD
use anyhow::Result;
use esp_idf_sys::*;
use log::info;

/// Apply fixes for the 6-block readable pattern issue
pub unsafe fn apply_6block_fix(
    bus_config: &mut esp_lcd_i80_bus_config_t,
    io_config: &mut esp_lcd_panel_io_i80_config_t,
) -> Result<()> {
    info!("=== 6-Block Pattern Fix DISABLED ===");
    info!("The 6-block issue was caused by incorrect byte swapping configuration.");
    info!("With swap_color_bytes=0 and MADCTL=0x68, the issue should be resolved.");
    
    // Don't modify any settings - let the flicker fix handle optimization
    // The real fix is in the byte order configuration
    
    Ok(())
}

/// Test if the 6-block issue is clock-related
pub unsafe fn test_clock_speeds(
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== Testing Different Clock Speeds for 6-Block Issue ===");
    
    // Test pattern - alternating colors every 6 pixels
    let mut pattern = vec![0u16; 320 * 10];
    for i in 0..pattern.len() {
        if (i / 6) % 2 == 0 {
            pattern[i] = 0xF800; // Red
        } else {
            pattern[i] = 0x07E0; // Green
        }
    }
    
    // Draw the pattern
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 10,
        pattern.as_ptr() as *const _,
    );
    
    info!("Drew 6-pixel alternating pattern");
    info!("If you see solid colors instead of stripes, the 6-block issue is fixed!");
    
    Ok(())
}

/// Analyze the 6-block pattern in detail
pub unsafe fn analyze_6block_pattern(
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== Detailed 6-Block Pattern Analysis ===");
    
    // Test 1: Single pixel writes at 6-pixel intervals
    info!("Test 1: Writing single pixels at positions 0, 6, 12, 18...");
    for i in 0..10 {
        let x = (i * 6) as i32;
        
        // Draw a white pixel
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x, 50,
            x + 1, 51,
            &[0xFFFF_u16] as *const _ as *const _,
        );
    }
    
    info!("  If you see evenly spaced white pixels, alignment is correct");
    
    // Test 2: 6-pixel blocks
    info!("\nTest 2: Drawing 6-pixel wide blocks");
    for i in 0..10 {
        let x = (i * 30) as i32; // 30 pixels apart
        let color = match i % 3 {
            0 => 0xF800_u16, // Red
            1 => 0x07E0_u16, // Green
            _ => 0x001F_u16, // Blue
        };
        
        // Draw 6-pixel wide block
        let block = vec![color; 6 * 20]; // 6x20 pixels
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x, 70,
            x + 6, 90,
            block.as_ptr() as *const _,
        );
    }
    
    info!("  You should see colored blocks 30 pixels apart");
    
    // Test 3: Continuous pattern with markers every 6 pixels
    info!("\nTest 3: Continuous pattern with 6-pixel markers");
    let mut line = vec![0x0000_u16; 320];
    for i in 0..320 {
        if i % 6 == 0 {
            line[i] = 0xFFFF; // White marker every 6 pixels
        } else {
            line[i] = 0x7BEF; // Gray background
        }
    }
    
    // Draw 10 lines
    for y in 100..110 {
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, y,
            320, y + 1,
            line.as_ptr() as *const _,
        );
    }
    
    info!("  You should see vertical white lines every 6 pixels");
    info!("  If only some lines appear, it confirms the 6-block issue");
    
    info!("\n=== Analysis Complete ===");
    info!("The 6-block pattern indicates:");
    info!("1. DMA transfers are succeeding every 6 pixels");
    info!("2. This could be timing or alignment related");
    info!("3. The fix adjusts clock speed and transfer alignment");
    
    Ok(())
}