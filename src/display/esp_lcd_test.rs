/// Test module for ESP LCD implementation
use super::lcd_cam_display_manager::LcdDisplayManager;
use super::colors;
use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::delay::Ets;
use log::info;

pub fn test_esp_lcd_black_screen() -> Result<()> {
    info!("[ESP_LCD_TEST] Starting black screen test...");
    
    let peripherals = Peripherals::take().unwrap();
    
    // Create display manager with ESP LCD
    let mut display = LcdDisplayManager::new(
        peripherals.pins.gpio39, // D0
        peripherals.pins.gpio40, // D1
        peripherals.pins.gpio41, // D2
        peripherals.pins.gpio42, // D3
        peripherals.pins.gpio45, // D4
        peripherals.pins.gpio46, // D5
        peripherals.pins.gpio47, // D6
        peripherals.pins.gpio48, // D7
        peripherals.pins.gpio8,  // WR
        peripherals.pins.gpio7,  // DC
        peripherals.pins.gpio6,  // CS
        peripherals.pins.gpio5,  // RST
        peripherals.pins.gpio38, // Backlight
        peripherals.pins.gpio15, // LCD Power
        peripherals.pins.gpio9,  // RD
    )?;
    
    info!("[ESP_LCD_TEST] Display initialized successfully!");
    
    // Test 1: Fill screen black
    info!("[ESP_LCD_TEST] Test 1: Filling screen black...");
    display.clear(colors::BLACK)?;
    display.flush()?;
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
    
    // Draw test pattern
    display.fill_rect(10, 10, 50, 50, colors::RED)?;
    display.fill_rect(70, 10, 50, 50, colors::GREEN)?;
    display.fill_rect(130, 10, 50, 50, colors::BLUE)?;
    display.fill_rect(190, 10, 50, 50, colors::YELLOW)?;
    
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
    
    Ok(())
}