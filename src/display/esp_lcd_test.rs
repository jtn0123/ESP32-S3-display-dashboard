/// Test module for ESP LCD implementation
use super::lcd_cam_display_manager::LcdDisplayManager;
use super::colors;
use super::esp_lcd_aggressive_debug;
use super::esp_lcd_config::OptimizedLcdConfig;
use anyhow::Result;
use esp_idf_hal::delay::Ets;
use log::info;
use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Gpio8, Gpio9, Gpio15, Gpio38, Gpio39, Gpio40, Gpio41, Gpio42, Gpio45, Gpio46, Gpio47, Gpio48};

pub fn test_esp_lcd_black_screen(
    d0: Gpio39,
    d1: Gpio40,
    d2: Gpio41,
    d3: Gpio42,
    d4: Gpio45,
    d5: Gpio46,
    d6: Gpio47,
    d7: Gpio48,
    wr: Gpio8,
    dc: Gpio7,
    cs: Gpio6,
    rst: Gpio5,
    backlight: Gpio38,
    lcd_power: Gpio15,
    rd: Gpio9,
) -> Result<()> {
    info!("[ESP_LCD_TEST] ========================================");
    info!("[ESP_LCD_TEST] ESP LCD DMA Hardware Test v5.37-dma");
    info!("[ESP_LCD_TEST] ========================================");
    info!("[ESP_LCD_TEST] Starting test sequence...");
    
    // First run aggressive debug to test backlight
    info!("[ESP_LCD_TEST] Running aggressive hardware debug first...");
    info!("[ESP_LCD_TEST] NOTE: This will use the backlight and LCD power pins");
    info!("[ESP_LCD_TEST] The main test will need to be modified to work without them");
    
    // For now, let's skip the aggressive test and focus on other debugging
    // esp_lcd_aggressive_debug::aggressive_esp_lcd_debug(backlight, lcd_power)?;
    
    // Create display manager with ESP LCD using slower speed for debug
    info!("[ESP_LCD_TEST] Using debug_slow configuration (5 MHz)");
    let mut display = LcdDisplayManager::with_config(
        d0, d1, d2, d3, d4, d5, d6, d7,
        wr, dc, cs, rst,
        backlight, lcd_power, rd,
        OptimizedLcdConfig::debug_slow(),
    )?;
    
    info!("[ESP_LCD_TEST] Display initialized successfully!");
    
    // Test 1: Fill screen black
    info!("[ESP_LCD_TEST] Test 1: Filling screen black...");
    info!("[ESP_LCD_TEST] Display dimensions: {}x{}", display.width(), display.height());
    display.clear(colors::BLACK)?;
    display.flush()?;
    info!("[ESP_LCD_TEST] Black fill complete - display should turn dark");
    Ets::delay_ms(1000);
    
    // Test 2: Fill screen with different colors
    info!("[ESP_LCD_TEST] Test 2: Color cycle test...");
    
    // Red
    info!("[ESP_LCD_TEST] - Red");
    display.clear(colors::RED)?;
    display.flush()?;
    Ets::delay_ms(500);
    
    // Green
    info!("[ESP_LCD_TEST] - Green");
    display.clear(colors::GREEN)?;
    display.flush()?;
    Ets::delay_ms(500);
    
    // Blue
    info!("[ESP_LCD_TEST] - Blue");
    display.clear(colors::BLUE)?;
    display.flush()?;
    Ets::delay_ms(500);
    
    // White
    info!("[ESP_LCD_TEST] - White");
    display.clear(colors::WHITE)?;
    display.flush()?;
    Ets::delay_ms(500);
    
    // Test 3: Draw rectangles
    info!("[ESP_LCD_TEST] Test 3: Drawing rectangles...");
    display.clear(colors::BLACK)?;
    
    // Draw test pattern - adjusted for 320x170 landscape display
    display.fill_rect(10, 10, 50, 50, colors::RED)?;
    display.fill_rect(70, 10, 50, 50, colors::GREEN)?;
    display.fill_rect(130, 10, 50, 50, colors::BLUE)?;
    display.fill_rect(190, 10, 50, 50, colors::YELLOW)?;
    display.fill_rect(250, 10, 50, 50, colors::WHITE)?;  // Added 5th rectangle
    
    display.flush()?;
    Ets::delay_ms(2000);
    
    // Test 4: Text rendering
    info!("[ESP_LCD_TEST] Test 4: Text rendering...");
    display.clear(colors::BLACK)?;
    display.draw_text(10, 10, "ESP LCD Working!", colors::WHITE, None, 1)?;
    display.draw_text(10, 30, "DMA Enabled", colors::GREEN, None, 1)?;
    display.draw_text(10, 50, &format!("v{}", crate::version::DISPLAY_VERSION), colors::YELLOW, None, 1)?;
    display.flush()?;
    
    info!("[ESP_LCD_TEST] All tests completed successfully!");
    info!("[ESP_LCD_TEST] Expected serial output:");
    info!("[ESP_LCD_TEST] - 'I (xxx) lcd_panel: new I80 bus(iomux), clk=17MHz ...'");
    info!("[ESP_LCD_TEST] - Display should show colors and text");
    
    // Run performance benchmarks if test passed
    info!("[ESP_LCD_TEST] Starting performance benchmarks...");
    Ets::delay_ms(2000);
    
    // Can't run full benchmark here since we already took peripherals
    // Just measure current performance
    let start = std::time::Instant::now();
    for _ in 0..100 {
        display.clear(colors::BLACK)?;
        display.flush()?;
    }
    let elapsed = start.elapsed();
    let fps = 100.0 / elapsed.as_secs_f32();
    
    info!("[ESP_LCD_TEST] Quick benchmark: {:.1} FPS (target: >25 FPS)", fps);
    
    Ok(())
}