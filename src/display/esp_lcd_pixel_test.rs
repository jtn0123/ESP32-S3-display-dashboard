use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;

/// Test pixel data format and byte order
pub unsafe fn test_pixel_formats(
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== ESP LCD PIXEL FORMAT TEST ===");
    
    // Test 1: Raw pixel data in different formats
    info!("Test 1: Testing RGB565 byte orders...");
    
    // Pure red in RGB565 = R(31) G(0) B(0) = 0xF800
    // In little endian: 0x00 0xF8
    // In big endian: 0xF8 0x00
    
    let test_cases = [
        ("RGB565 Big Endian (0xF800)", vec![0xF8u8, 0x00], "RED"),
        ("RGB565 Little Endian (0x00F8)", vec![0x00u8, 0xF8], "RED"),
        ("RGB565 Green (0x07E0)", vec![0x07u8, 0xE0], "GREEN"),
        ("RGB565 Green LE", vec![0xE0u8, 0x07], "GREEN"),
        ("RGB565 Blue (0x001F)", vec![0x00u8, 0x1F], "BLUE"),
        ("RGB565 Blue LE", vec![0x1Fu8, 0x00], "BLUE"),
    ];
    
    for (name, data, expected_color) in &test_cases {
        info!("  Testing {}: expecting {}", name, expected_color);
        
        // Reset watchdog to prevent timeout
        esp_task_wdt_reset();
        
        // Set window to 10x10 at position (50, 50)
        set_window(io_handle, 50, 50, 60, 60)?;
        
        // Send RAMWR command
        esp_lcd_panel_io_tx_param(io_handle, 0x2C, std::ptr::null(), 0);
        
        // Send pixel data (100 pixels = 10x10)
        let mut pixel_data = Vec::new();
        for _ in 0..100 {
            pixel_data.extend_from_slice(data);
        }
        
        // Send as color data (not command)
        esp_lcd_panel_io_tx_color(
            io_handle,
            -1i32,  // No command, just data
            pixel_data.as_ptr() as *const _,
            pixel_data.len(),
        );
        
        info!("    Drew 10x10 square at (50,50)");
        Ets::delay_ms(500);  // Reduced delay
    }
    
    // Test 2: Using panel draw bitmap with u16 values
    info!("\nTest 2: Testing u16 pixel values...");
    
    let u16_tests = [
        ("u16 Red (0xF800)", 0xF800u16, "RED"),
        ("u16 Green (0x07E0)", 0x07E0u16, "GREEN"),
        ("u16 Blue (0x001F)", 0x001Fu16, "BLUE"),
        ("u16 White (0xFFFF)", 0xFFFFu16, "WHITE"),
        ("u16 Yellow (0xFFE0)", 0xFFE0u16, "YELLOW"),
        ("u16 Cyan (0x07FF)", 0x07FFu16, "CYAN"),
    ];
    
    for (i, (name, color, expected)) in u16_tests.iter().enumerate() {
        info!("  Testing {}: expecting {}", name, expected);
        
        // Reset watchdog
        esp_task_wdt_reset();
        
        let x = (i as i32 % 3) * 70 + 10;
        let y = (i as i32 / 3) * 70 + 10;
        
        // Create buffer with single color
        let buffer: Vec<u16> = vec![*color; 50 * 50];
        
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x, y,
            x + 50, y + 50,
            buffer.as_ptr() as *const _,
        );
        
        info!("    Drew 50x50 square at ({},{})", x, y);
    }
    
    // Reset watchdog before delay
    esp_task_wdt_reset();
    Ets::delay_ms(1000);  // Reduced delay
    
    // Test 3: Swap bytes test
    info!("\nTest 3: Testing byte-swapped values...");
    
    let swap_tests = [
        ("Normal Red", 0xF800u16, 0xF800u16),
        ("Swapped Red", 0xF800u16, 0x00F8u16),
        ("Normal Green", 0x07E0u16, 0x07E0u16),
        ("Swapped Green", 0x07E0u16, 0xE007u16),
    ];
    
    for (i, (name, original, value)) in swap_tests.iter().enumerate() {
        info!("  {} (0x{:04X} -> 0x{:04X})", name, original, value);
        
        // Reset watchdog
        esp_task_wdt_reset();
        
        let y = i as i32 * 40;
        let buffer: Vec<u16> = vec![*value; 320 * 30];
        
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, y,
            320, y + 30,
            buffer.as_ptr() as *const _,
        );
    }
    
    // Reset watchdog before delay
    esp_task_wdt_reset();
    Ets::delay_ms(1000);  // Reduced delay
    
    // Test 4: Direct I80 data endian test
    info!("\nTest 4: Testing I80 data endian configuration...");
    
    // Try to modify the I80 bus data endian setting
    // This would require access to the bus config, which we don't have after creation
    // So we'll test with raw data instead
    
    // Clear screen first
    clear_screen(panel_handle)?;
    
    // Reset watchdog
    esp_task_wdt_reset();
    
    // Draw test pattern with raw bytes
    set_window(io_handle, 0, 0, 320, 170)?;
    esp_lcd_panel_io_tx_param(io_handle, 0x2C, std::ptr::null(), 0);
    
    // Create alternating red/blue pattern
    let mut pattern = Vec::new();
    for y in 0..170 {
        // Reset watchdog periodically during pattern creation
        if y % 50 == 0 {
            esp_task_wdt_reset();
        }
        
        for x in 0..320 {
            if (x / 20) % 2 == 0 {
                // Red: RGB565 = 0xF800
                pattern.push(0xF8u8);
                pattern.push(0x00u8);
            } else {
                // Blue: RGB565 = 0x001F  
                pattern.push(0x00u8);
                pattern.push(0x1Fu8);
            }
        }
    }
    
    esp_lcd_panel_io_tx_color(
        io_handle,
        -1i32,
        pattern.as_ptr() as *const _,
        pattern.len(),
    );
    
    info!("  Drew alternating red/blue stripes");
    info!("  If colors are correct: data format is correct");
    info!("  If colors are wrong/garbled: endian issue");
    
    info!("\n=== PIXEL FORMAT TEST COMPLETE ===");
    info!("Check display for:");
    info!("- Test 1: Various 10x10 colored squares");
    info!("- Test 2: Grid of 50x50 colored squares");
    info!("- Test 3: Horizontal color bars");
    info!("- Test 4: Alternating red/blue vertical stripes");
    
    Ok(())
}

unsafe fn set_window(
    io_handle: *mut esp_lcd_panel_io_t,
    x1: u16, y1: u16, x2: u16, y2: u16
) -> Result<()> {
    // CASET
    let caset: [u8; 4] = [
        (x1 >> 8) as u8, (x1 & 0xFF) as u8,
        (x2 >> 8) as u8, (x2 & 0xFF) as u8,
    ];
    esp_lcd_panel_io_tx_param(io_handle, 0x2A, caset.as_ptr() as *const _, 4);
    
    // RASET - add Y_GAP offset
    let y1_offset = y1 + 35;
    let y2_offset = y2 + 35;
    let raset: [u8; 4] = [
        (y1_offset >> 8) as u8, (y1_offset & 0xFF) as u8,
        (y2_offset >> 8) as u8, (y2_offset & 0xFF) as u8,
    ];
    esp_lcd_panel_io_tx_param(io_handle, 0x2B, raset.as_ptr() as *const _, 4);
    
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