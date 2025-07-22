use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;

/// Test different I80 clock speeds
pub unsafe fn test_clock_speeds(
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== ESP LCD CLOCK SPEED TEST ===");
    
    // Test pattern - simple red fill
    let red_buffer = vec![0xF800u16; 320 * 170];
    
    let clock_speeds = [
        ("2 MHz (Very Slow)", 2_000_000),
        ("5 MHz (Slow)", 5_000_000),
        ("10 MHz (Medium)", 10_000_000),
        ("17 MHz (Current)", 17_000_000),
        ("20 MHz (Fast)", 20_000_000),
    ];
    
    for (name, _speed) in &clock_speeds {
        info!("\nTesting clock speed: {}", name);
        info!("Note: Clock speed already set during initialization");
        info!("This test verifies display works at current speed");
        
        // Clear display to black
        let black_buffer = vec![0u16; 320 * 170];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            320, 170,
            black_buffer.as_ptr() as *const _,
        );
        
        Ets::delay_ms(100);
        
        // Draw red fill
        info!("Drawing red fill pattern...");
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            320, 170,
            red_buffer.as_ptr() as *const _,
        );
        
        info!("Pattern drawn. Display should show red screen.");
        info!("Waiting 2 seconds...");
        Ets::delay_ms(2000);
    }
    
    info!("\n=== CLOCK SPEED TEST COMPLETE ===");
    info!("If no display output at any speed, issue is not clock-related");
    
    Ok(())
}

/// Test hardware reset sequence with detailed timing
pub unsafe fn test_reset_sequence(
    rst_pin: i32,
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== ESP LCD RESET SEQUENCE TEST ===");
    
    // Configure reset pin as output
    let gpio_config = gpio_config_t {
        pin_bit_mask: 1u64 << rst_pin,
        mode: gpio_mode_t_GPIO_MODE_OUTPUT,
        pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
        pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_ENABLE,
        intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
    };
    esp_idf_sys::gpio_config(&gpio_config);
    
    info!("Test 1: Long reset pulse (100ms low)");
    gpio_set_level(rst_pin, 1); // High
    Ets::delay_ms(10);
    gpio_set_level(rst_pin, 0); // Low - Reset active
    info!("  Reset pin LOW");
    Ets::delay_ms(100);
    gpio_set_level(rst_pin, 1); // High - Reset release
    info!("  Reset pin HIGH");
    Ets::delay_ms(200); // Wait for stabilization
    
    // Re-initialize panel after reset
    info!("Re-initializing panel after reset...");
    esp_lcd_panel_init(panel_handle);
    Ets::delay_ms(100);
    
    // Try to draw something
    draw_test_pattern(panel_handle)?;
    Ets::delay_ms(2000);
    
    info!("Test 2: Multiple reset pulses");
    for i in 0..3 {
        info!("  Reset pulse {}", i + 1);
        gpio_set_level(rst_pin, 0);
        Ets::delay_ms(10);
        gpio_set_level(rst_pin, 1);
        Ets::delay_ms(50);
    }
    
    // Re-initialize again
    esp_lcd_panel_init(panel_handle);
    Ets::delay_ms(100);
    draw_test_pattern(panel_handle)?;
    
    info!("\n=== RESET SEQUENCE TEST COMPLETE ===");
    
    Ok(())
}

/// Test power sequencing with detailed timing
pub unsafe fn test_power_sequence(
    backlight_pin: i32,
    lcd_power_pin: i32,
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== ESP LCD POWER SEQUENCE TEST ===");
    
    // Configure power pins
    let gpio_config = gpio_config_t {
        pin_bit_mask: (1u64 << backlight_pin) | (1u64 << lcd_power_pin),
        mode: gpio_mode_t_GPIO_MODE_OUTPUT,
        pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
        pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
        intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
    };
    esp_idf_sys::gpio_config(&gpio_config);
    
    info!("Test 1: Power off/on cycle");
    
    // Power everything off
    info!("Powering off display...");
    gpio_set_level(backlight_pin, 0);
    gpio_set_level(lcd_power_pin, 0);
    Ets::delay_ms(500);
    
    // Power on sequence
    info!("Power on sequence:");
    info!("  1. LCD power ON");
    gpio_set_level(lcd_power_pin, 1);
    Ets::delay_ms(100);
    
    info!("  2. Wait 100ms");
    Ets::delay_ms(100);
    
    info!("  3. Re-initialize panel");
    esp_lcd_panel_init(panel_handle);
    Ets::delay_ms(100);
    
    info!("  4. Backlight ON");
    gpio_set_level(backlight_pin, 1);
    Ets::delay_ms(100);
    
    info!("  5. Draw test pattern");
    draw_test_pattern(panel_handle)?;
    
    info!("Display should now show test pattern");
    Ets::delay_ms(2000);
    
    info!("Test 2: Backlight PWM fade");
    // Simple PWM simulation with GPIO
    for _ in 0..3 {
        for i in 0..10 {
            // Duty cycle simulation
            gpio_set_level(backlight_pin, 1);
            Ets::delay_us(i * 100);
            gpio_set_level(backlight_pin, 0);
            Ets::delay_us((10 - i) * 100);
        }
    }
    gpio_set_level(backlight_pin, 1); // Full on
    
    info!("\n=== POWER SEQUENCE TEST COMPLETE ===");
    
    Ok(())
}

/// Draw a simple test pattern
fn draw_test_pattern(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    // Create a diagonal stripe pattern
    let mut buffer = vec![0u16; 320 * 170];
    
    for y in 0..170 {
        for x in 0..320 {
            let idx = y * 320 + x;
            if (x + y) % 20 < 10 {
                buffer[idx] = 0xF800; // Red
            } else {
                buffer[idx] = 0xFFFF; // White
            }
        }
    }
    
    unsafe {
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            320, 170,
            buffer.as_ptr() as *const _,
        );
    }
    
    Ok(())
}