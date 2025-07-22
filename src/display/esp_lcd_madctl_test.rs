use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;

/// Test different MADCTL (Memory Access Control) configurations
pub unsafe fn test_madctl_configurations(
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== ESP LCD MADCTL CONFIGURATION TEST ===");
    info!("Testing different ST7789 memory access control settings...");
    
    // MADCTL register bits:
    // Bit 7: MY (Row Address Order) - 0=Top to Bottom, 1=Bottom to Top  
    // Bit 6: MX (Column Address Order) - 0=Left to Right, 1=Right to Left
    // Bit 5: MV (Row/Column Exchange) - 0=Normal, 1=Exchange
    // Bit 4: ML (Vertical Refresh Order) - 0=Top to Bottom, 1=Bottom to Top
    // Bit 3: BGR - 0=RGB, 1=BGR
    // Bit 2: MH (Horizontal Refresh Order) - 0=Left to Right, 1=Right to Left
    
    let test_configs = [
        // Common configurations for T-Display-S3
        ("Default Portrait", 0x00, "MY=0 MX=0 MV=0 BGR=0"),
        ("Portrait Flipped", 0xC0, "MY=1 MX=1 MV=0 BGR=0"),
        ("Landscape Right", 0x60, "MY=0 MX=1 MV=1 BGR=0"),
        ("Landscape Left", 0xA0, "MY=1 MX=0 MV=1 BGR=0"),
        ("Current GPIO Working", 0x60, "MY=0 MX=1 MV=1 BGR=0 (matches GPIO)"),
        
        // Test BGR mode
        ("Landscape BGR", 0x68, "MY=0 MX=1 MV=1 BGR=1"),
        ("Portrait BGR", 0x08, "MY=0 MX=0 MV=0 BGR=1"),
        
        // Other orientations
        ("Rotated 180", 0xC0, "MY=1 MX=1 MV=0 BGR=0"),
        ("Mirror X", 0x40, "MY=0 MX=1 MV=0 BGR=0"),
        ("Mirror Y", 0x80, "MY=1 MX=0 MV=0 BGR=0"),
        
        // Test with different refresh orders
        ("Landscape + ML", 0x70, "MY=0 MX=1 MV=1 ML=1 BGR=0"),
        ("Landscape + MH", 0x64, "MY=0 MX=1 MV=1 MH=1 BGR=0"),
    ];
    
    for (name, madctl_value, description) in &test_configs {
        info!("\n--- Testing: {} ---", name);
        info!("MADCTL = 0x{:02X} ({})", madctl_value, description);
        
        // Send MADCTL command
        let madctl_data = [*madctl_value];
        esp_lcd_panel_io_tx_param(io_handle, 0x36, madctl_data.as_ptr() as *const _, 1);
        
        // Wait for command to take effect
        Ets::delay_ms(10);
        
        // Clear screen to black
        clear_screen(panel_handle)?;
        
        // Draw test pattern to verify orientation
        draw_orientation_test_pattern(panel_handle)?;
        
        // Wait to observe
        info!("Pattern drawn. Observe display for 2 seconds...");
        Ets::delay_ms(2000);
    }
    
    // Final test: Set back to known working configuration
    info!("\n--- Restoring Working Configuration ---");
    info!("Setting MADCTL = 0x60 (Landscape Right, RGB)");
    
    let madctl_data = [0x60u8];
    esp_lcd_panel_io_tx_param(io_handle, 0x36, madctl_data.as_ptr() as *const _, 1);
    Ets::delay_ms(10);
    
    // Clear and draw final test
    clear_screen(panel_handle)?;
    draw_final_test_pattern(panel_handle)?;
    
    info!("\n=== MADCTL TEST COMPLETE ===");
    info!("Results:");
    info!("- If any configuration showed correct image: MADCTL is the issue");
    info!("- If all black: Display connection or initialization issue");
    info!("- If garbled: Data format or timing issue");
    
    Ok(())
}

fn clear_screen(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    let black_buffer = vec![0u16; 320 * 170];
    unsafe {
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            320, 170,
            black_buffer.as_ptr() as *const _,
        );
    }
    Ok(())
}

fn draw_orientation_test_pattern(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    // Draw pattern that shows orientation clearly
    // Assuming 320x170 display in landscape
    
    unsafe {
        // Red rectangle in top-left corner
        let red_buffer = vec![0xF800u16; 50 * 50];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            50, 50,
            red_buffer.as_ptr() as *const _,
        );
        
        // Green rectangle in top-right corner
        let green_buffer = vec![0x07E0u16; 50 * 50];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            270, 0,
            320, 50,
            green_buffer.as_ptr() as *const _,
        );
        
        // Blue rectangle in bottom-left corner
        let blue_buffer = vec![0x001Fu16; 50 * 50];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 120,
            50, 170,
            blue_buffer.as_ptr() as *const _,
        );
        
        // White rectangle in bottom-right corner
        let white_buffer = vec![0xFFFFu16; 50 * 50];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            270, 120,
            320, 170,
            white_buffer.as_ptr() as *const _,
        );
        
        // Yellow line across top
        let yellow_buffer = vec![0xFFE0u16; 320 * 5];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            320, 5,
            yellow_buffer.as_ptr() as *const _,
        );
        
        // Cyan line down left side
        let cyan_buffer = vec![0x07FFu16; 5];
        for y in 0..170 {
            esp_lcd_panel_draw_bitmap(
                panel_handle,
                0, y,
                5, y + 1,
                cyan_buffer.as_ptr() as *const _,
            );
        }
    }
    
    Ok(())
}

fn draw_final_test_pattern(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    info!("Drawing final validation pattern...");
    
    unsafe {
        // Draw "ESP32" text pattern using colored blocks
        // E
        let red = vec![0xF800u16; 10 * 50];
        esp_lcd_panel_draw_bitmap(panel_handle, 10, 60, 20, 110, red.as_ptr() as *const _);
        let red2 = vec![0xF800u16; 30 * 10];
        esp_lcd_panel_draw_bitmap(panel_handle, 10, 60, 40, 70, red2.as_ptr() as *const _);
        esp_lcd_panel_draw_bitmap(panel_handle, 10, 80, 40, 90, red2.as_ptr() as *const _);
        esp_lcd_panel_draw_bitmap(panel_handle, 10, 100, 40, 110, red2.as_ptr() as *const _);
        
        // S
        let green = vec![0x07E0u16; 30 * 10];
        esp_lcd_panel_draw_bitmap(panel_handle, 50, 60, 80, 70, green.as_ptr() as *const _);
        let green2 = vec![0x07E0u16; 10 * 20];
        esp_lcd_panel_draw_bitmap(panel_handle, 50, 60, 60, 80, green2.as_ptr() as *const _);
        esp_lcd_panel_draw_bitmap(panel_handle, 50, 80, 80, 90, green.as_ptr() as *const _);
        esp_lcd_panel_draw_bitmap(panel_handle, 70, 90, 80, 110, green2.as_ptr() as *const _);
        esp_lcd_panel_draw_bitmap(panel_handle, 50, 100, 80, 110, green.as_ptr() as *const _);
        
        // P
        let blue = vec![0x001Fu16; 10 * 50];
        esp_lcd_panel_draw_bitmap(panel_handle, 90, 60, 100, 110, blue.as_ptr() as *const _);
        let blue2 = vec![0x001Fu16; 30 * 10];
        esp_lcd_panel_draw_bitmap(panel_handle, 90, 60, 120, 70, blue2.as_ptr() as *const _);
        esp_lcd_panel_draw_bitmap(panel_handle, 90, 80, 120, 90, blue2.as_ptr() as *const _);
        let blue3 = vec![0x001Fu16; 10 * 20];
        esp_lcd_panel_draw_bitmap(panel_handle, 110, 60, 120, 80, blue3.as_ptr() as *const _);
        
        // Draw frame around entire display
        let white = vec![0xFFFFu16; 320 * 5];
        // Top
        esp_lcd_panel_draw_bitmap(panel_handle, 0, 0, 320, 5, white.as_ptr() as *const _);
        // Bottom
        esp_lcd_panel_draw_bitmap(panel_handle, 0, 165, 320, 170, white.as_ptr() as *const _);
        // Left
        for y in 0..170 {
            let white_vert = vec![0xFFFFu16; 5];
            esp_lcd_panel_draw_bitmap(panel_handle, 0, y, 5, y + 1, white_vert.as_ptr() as *const _);
        }
        // Right
        for y in 0..170 {
            let white_vert = vec![0xFFFFu16; 5];
            esp_lcd_panel_draw_bitmap(panel_handle, 315, y, 320, y + 1, white_vert.as_ptr() as *const _);
        }
    }
    
    info!("Final pattern shows:");
    info!("- 'ESP' letters in Red, Green, Blue");
    info!("- White frame around display edge");
    info!("- Verifies correct orientation and color");
    
    Ok(())
}