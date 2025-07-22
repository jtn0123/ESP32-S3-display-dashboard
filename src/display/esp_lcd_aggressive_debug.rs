use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::*;
use esp_idf_hal::delay::Ets;
use log::info;
use esp_idf_hal::peripheral::Peripheral;

pub fn aggressive_esp_lcd_debug(
    backlight: Gpio38,
    lcd_power: Gpio15,
) -> Result<()> {
    info!("=== AGGRESSIVE ESP LCD DEBUG ===");
    
    // First, test basic GPIO control of backlight and power
    info!("Testing direct GPIO control of backlight and power...");
    
    // Configure as outputs
    let mut backlight = PinDriver::output(backlight)?;
    let mut lcd_power = PinDriver::output(lcd_power)?;
    
    // Test 1: Flash backlight to verify control
    info!("Test 1: Flashing backlight 5 times...");
    for i in 0..5 {
        info!("  Flash {}: ON", i + 1);
        backlight.set_high()?;
        Ets::delay_ms(500);
        
        info!("  Flash {}: OFF", i + 1);
        backlight.set_low()?;
        Ets::delay_ms(500);
    }
    
    // Turn backlight on
    backlight.set_high()?;
    info!("Backlight should now be ON");
    
    // Test 2: Toggle LCD power
    info!("Test 2: Testing LCD power control...");
    info!("  LCD power OFF");
    lcd_power.set_low()?;
    Ets::delay_ms(1000);
    
    info!("  LCD power ON");
    lcd_power.set_high()?;
    Ets::delay_ms(1000);
    
    info!("Backlight and LCD power are now ON");
    info!("If you see the backlight flashing, GPIO control works");
    info!("If display is still black with backlight on, issue is elsewhere");
    
    // Test 3: PWM test skipped (would require consuming the backlight pin)
    info!("Test 3: PWM control test skipped");
    info!("(PWM test would require consuming the backlight pin)");
    
    // Keep backlight on
    backlight.set_high()?;
    
    // Test 4: Verify all GPIO states
    info!("Test 4: Reading all GPIO states...");
    unsafe {
        // Read GPIO states
        let gpio_pins = [5, 6, 7, 8, 9, 15, 38, 39, 40, 41, 42, 45, 46, 47, 48];
        for &pin in &gpio_pins {
            let level = gpio_get_level(pin);
            info!("  GPIO{}: {}", pin, if level == 0 { "LOW" } else { "HIGH" });
        }
    }
    
    info!("=== AGGRESSIVE DEBUG COMPLETE ===");
    info!("Visual indicators tested:");
    info!("1. Backlight should have flashed 5 times");
    info!("2. Backlight should have faded up");
    info!("3. Backlight should now be at 100%");
    info!("4. If no visual changes, check hardware connections");
    
    Ok(())
}

// Test raw ST7789 commands with delays
pub unsafe fn test_st7789_with_delays(
    io: *mut esp_lcd_panel_io_t,
    panel: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== ST7789 COMMAND TEST WITH DELAYS ===");
    
    // Helper to send command with delay
    let send_cmd = |cmd: u8, params: Option<&[u8]>, delay_ms: u32| {
        info!("Sending command 0x{:02X} with {}ms delay", cmd, delay_ms);
        
        if let Some(data) = params {
            esp_lcd_panel_io_tx_param(io, cmd as i32, data.as_ptr() as *const _, data.len());
        } else {
            esp_lcd_panel_io_tx_param(io, cmd as i32, std::ptr::null(), 0);
        }
        
        if delay_ms > 0 {
            Ets::delay_ms(delay_ms);
        }
    };
    
    // Software reset with long delay
    send_cmd(0x01, None, 150); // SWRESET - needs 120ms minimum
    
    // Sleep out with delay
    send_cmd(0x11, None, 120); // SLPOUT - needs 120ms
    
    // Normal display mode
    send_cmd(0x13, None, 10); // NORON
    
    // Display inversion on (required for ST7789)
    send_cmd(0x21, None, 10); // INVON
    
    // Memory data access control - try different values
    info!("Testing different MADCTL values...");
    
    // Test 1: Default portrait
    send_cmd(0x36, Some(&[0x00]), 10);
    test_draw_pattern(panel, "Portrait (0x00)")?;
    Ets::delay_ms(1000);
    
    // Test 2: Landscape (MX + MV)
    send_cmd(0x36, Some(&[0x60]), 10);
    test_draw_pattern(panel, "Landscape (0x60)")?;
    Ets::delay_ms(1000);
    
    // Test 3: Landscape with RGB
    send_cmd(0x36, Some(&[0x70]), 10);
    test_draw_pattern(panel, "Landscape RGB (0x70)")?;
    Ets::delay_ms(1000);
    
    // Test 4: Try BGR mode
    send_cmd(0x36, Some(&[0x68]), 10);
    test_draw_pattern(panel, "Landscape BGR (0x68)")?;
    Ets::delay_ms(1000);
    
    // Interface pixel format
    send_cmd(0x3A, Some(&[0x55]), 10); // 16-bit RGB565
    
    // Display on
    send_cmd(0x29, None, 100); // DISPON
    
    info!("=== ST7789 COMMAND TEST COMPLETE ===");
    
    Ok(())
}

fn test_draw_pattern(panel: esp_lcd_panel_handle_t, label: &str) -> Result<()> {
    info!("Drawing test pattern: {}", label);
    
    // Create a simple pattern buffer
    let mut buffer = vec![0u16; 320 * 170];
    
    // Fill with different colors in quadrants
    for y in 0..170 {
        for x in 0..320 {
            let idx = y * 320 + x;
            buffer[idx] = if x < 160 && y < 85 {
                0xF800 // Red
            } else if x >= 160 && y < 85 {
                0x07E0 // Green
            } else if x < 160 && y >= 85 {
                0x001F // Blue
            } else {
                0xFFFF // White
            };
        }
    }
    
    // Draw to display
    unsafe {
        esp_lcd_panel_draw_bitmap(
            panel,
            0, 0,
            320, 170,
            buffer.as_ptr() as *const _,
        );
    }
    
    Ok(())
}

// Test byte order in pixel data
pub fn test_pixel_byte_order(panel: esp_lcd_panel_handle_t) -> Result<()> {
    info!("=== PIXEL BYTE ORDER TEST ===");
    
    // Test different byte orders for RGB565
    let test_patterns = [
        ("RGB565 Little Endian", false),
        ("RGB565 Big Endian", true),
    ];
    
    for (name, swap_bytes) in &test_patterns {
        info!("Testing: {}", name);
        
        let mut buffer = vec![0u16; 100];
        
        // Fill with pure red (RGB565: R=31, G=0, B=0)
        for i in 0..100 {
            let red_565 = 0xF800u16;
            buffer[i] = if *swap_bytes {
                red_565.swap_bytes()
            } else {
                red_565
            };
        }
        
        // Draw a 10x10 red square
        unsafe {
            esp_lcd_panel_draw_bitmap(
                panel,
                10, 10,
                20, 20,
                buffer.as_ptr() as *const _,
            );
        }
        
        info!("  Drew 10x10 square at (10,10)");
        info!("  Should be RED if byte order is correct");
        
        Ets::delay_ms(2000);
    }
    
    info!("=== BYTE ORDER TEST COMPLETE ===");
    
    Ok(())
}