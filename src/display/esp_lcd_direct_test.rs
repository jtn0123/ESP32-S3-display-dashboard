use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;
use core::ptr;

/// Test direct I80 communication without panel abstraction
pub unsafe fn test_direct_i80(
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== ESP LCD DIRECT I80 TEST ===");
    info!("Bypassing panel abstraction for direct control");
    
    // Test 1: Software reset with maximum delays
    info!("\nTest 1: Software reset with long delays");
    send_command(io_handle, 0x01, None)?; // SWRESET
    info!("  Sent SWRESET, waiting 200ms...");
    Ets::delay_ms(200); // Longer than spec
    
    // Test 2: Sleep out with long delay
    info!("\nTest 2: Sleep out");
    send_command(io_handle, 0x11, None)?; // SLPOUT
    info!("  Sent SLPOUT, waiting 200ms...");
    Ets::delay_ms(200);
    
    // Test 3: Essential init commands only
    info!("\nTest 3: Minimal initialization");
    
    // Pixel format
    send_command(io_handle, 0x3A, Some(&[0x55]))?; // COLMOD - 16-bit
    Ets::delay_ms(10);
    
    // Memory access control - try different values
    let madctl_values = [0x00, 0x60, 0x70, 0xA0];
    for madctl in &madctl_values {
        info!("  Testing MADCTL = 0x{:02X}", madctl);
        send_command(io_handle, 0x36, Some(&[*madctl]))?;
        Ets::delay_ms(10);
        
        // Display on
        send_command(io_handle, 0x29, None)?; // DISPON
        Ets::delay_ms(10);
        
        // Draw test pattern
        draw_direct_pattern(io_handle)?;
        
        info!("  Pattern sent. Waiting 1 second...");
        Ets::delay_ms(1000);
    }
    
    // Test 4: Direct pixel push with raw commands
    info!("\nTest 4: Direct pixel data push");
    
    // Set window to full screen
    set_window_direct(io_handle, 0, 0, 319, 169)?;
    
    // Start memory write
    esp_lcd_panel_io_tx_param(io_handle, 0x2C, ptr::null(), 0);
    
    // Send raw pixel data in chunks
    info!("  Sending raw pixel data...");
    let chunk_size = 1000;
    let total_pixels = 320 * 170;
    let red_pixels: Vec<u8> = vec![0xF8, 0x00].repeat(chunk_size / 2);
    
    for i in 0..(total_pixels / (chunk_size / 2)) {
        esp_lcd_panel_io_tx_color(
            io_handle,
            -1,
            red_pixels.as_ptr() as *const _,
            red_pixels.len(),
        );
        
        if i % 10 == 0 {
            info!("    Progress: {}%", (i * chunk_size / 2 * 100) / total_pixels);
        }
    }
    
    info!("  Full screen red data sent");
    
    // Test 5: Inversion and color tests
    info!("\nTest 5: Display inversion test");
    
    // Normal display
    send_command(io_handle, 0x20, None)?; // INVOFF
    Ets::delay_ms(1000);
    
    // Inverted display
    send_command(io_handle, 0x21, None)?; // INVON
    Ets::delay_ms(1000);
    
    info!("\n=== DIRECT I80 TEST COMPLETE ===");
    info!("Results:");
    info!("- If display shows anything: Panel communication works");
    info!("- If still black: Hardware interface or power issue");
    
    Ok(())
}

/// Send command with optional parameters
unsafe fn send_command(
    io: *mut esp_lcd_panel_io_t,
    cmd: u8,
    params: Option<&[u8]>,
) -> Result<()> {
    if let Some(data) = params {
        esp_lcd_panel_io_tx_param(
            io,
            cmd as i32,
            data.as_ptr() as *const _,
            data.len(),
        );
        info!("  CMD 0x{:02X} with {} params sent", cmd, data.len());
    } else {
        esp_lcd_panel_io_tx_param(
            io,
            cmd as i32,
            ptr::null(),
            0,
        );
        info!("  CMD 0x{:02X} sent", cmd);
    }
    Ok(())
}

/// Set display window directly
unsafe fn set_window_direct(
    io: *mut esp_lcd_panel_io_t,
    x1: u16, y1: u16, x2: u16, y2: u16
) -> Result<()> {
    // CASET
    let caset: [u8; 4] = [
        (x1 >> 8) as u8, (x1 & 0xFF) as u8,
        (x2 >> 8) as u8, (x2 & 0xFF) as u8,
    ];
    send_command(io, 0x2A, Some(&caset))?;
    
    // RASET - with Y offset for T-Display-S3
    let y1_off = y1 + 35;
    let y2_off = y2 + 35;
    let raset: [u8; 4] = [
        (y1_off >> 8) as u8, (y1_off & 0xFF) as u8,
        (y2_off >> 8) as u8, (y2_off & 0xFF) as u8,
    ];
    send_command(io, 0x2B, Some(&raset))?;
    
    Ok(())
}

/// Draw pattern directly
unsafe fn draw_direct_pattern(io: *mut esp_lcd_panel_io_t) -> Result<()> {
    // Small test pattern in center
    set_window_direct(io, 100, 50, 220, 120)?;
    
    // RAMWR
    esp_lcd_panel_io_tx_param(io, 0x2C, ptr::null(), 0);
    
    // Send colorful pattern
    let width = 121;
    let height = 71;
    let mut pattern = Vec::new();
    
    for y in 0..height {
        for x in 0..width {
            let color = if (x + y) % 10 < 5 {
                [0xFF, 0xFF] // White
            } else {
                [0xF8, 0x00] // Red
            };
            pattern.extend_from_slice(&color);
        }
    }
    
    esp_lcd_panel_io_tx_color(
        io,
        -1,
        pattern.as_ptr() as *const _,
        pattern.len(),
    );
    
    Ok(())
}

/// Compare initialization sequences
pub fn compare_init_sequences() {
    info!("=== INIT SEQUENCE COMPARISON ===");
    
    info!("GPIO Working Sequence:");
    info!("1. Power on LCD_PWR (GPIO15)");
    info!("2. Power on Backlight (GPIO38)");
    info!("3. Reset pulse 10ms");
    info!("4. SWRESET + 150ms delay");
    info!("5. SLPOUT + 120ms delay");
    info!("6. MADCTL (0x60)");
    info!("7. COLMOD (0x55)");
    info!("8. INVON");
    info!("9. NORON");
    info!("10. DISPON");
    info!("11. Clear display");
    
    info!("\nESP LCD Sequence:");
    info!("1. Power pins configured");
    info!("2. I80 bus created");
    info!("3. Panel IO created");
    info!("4. ST7789 panel created (includes reset)");
    info!("5. panel_init (internal ST7789 init)");
    info!("6. Set gap, swap_xy, mirror, invert");
    info!("7. Additional INVON, NORON");
    info!("8. MADCTL (0x60)");
    info!("9. Window setup");
    info!("10. Display on");
    
    info!("\nKey Differences:");
    info!("- ESP LCD may have different reset timing");
    info!("- ESP LCD init may send additional commands");
    info!("- GPIO version has explicit delays");
    info!("- ESP LCD relies on driver's internal timing");
}