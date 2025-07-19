/// Test the LCD_CAM HAL implementation
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_sys::*;
use super::lcd_cam_hal::LcdCamHal;

pub fn test_lcd_cam_hal() -> Result<()> {
    log::warn!("Starting LCD_CAM HAL test...");
    
    // Initialize LCD power and backlight first
    unsafe {
        // LCD power pin (GPIO 15)
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        log::info!("LCD power enabled on GPIO 15");
        
        // Backlight pin (GPIO 38)
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
        log::info!("Backlight enabled on GPIO 38");
    }
    
    // Wait for power to stabilize
    FreeRtos::delay_ms(100);
    
    // Test 1: Initialize LCD_CAM
    log::info!("Test 1: Initializing LCD_CAM...");
    unsafe {
        match LcdCamHal::init() {
            Ok(()) => log::info!("LCD_CAM initialized successfully"),
            Err(e) => {
                log::error!("LCD_CAM init failed: {}", e);
                return Err(anyhow::anyhow!("LCD_CAM init failed: {}", e));
            }
        }
    }
    
    // Test 2: Configure for i8080 mode
    log::info!("Test 2: Configuring i8080 mode...");
    unsafe {
        match LcdCamHal::configure_i8080_8bit(10_000_000) {
            Ok(()) => log::info!("i8080 mode configured successfully"),
            Err(e) => {
                log::error!("i8080 config failed: {}", e);
                return Err(anyhow::anyhow!("i8080 config failed: {}", e));
            }
        }
    }
    
    // Test 3: Send a test command
    log::info!("Test 3: Sending test command...");
    unsafe {
        match LcdCamHal::send_command(0x00) {
            Ok(()) => log::info!("Test command sent successfully"),
            Err(e) => {
                log::error!("Command send failed: {}", e);
                return Err(anyhow::anyhow!("Command send failed: {}", e));
            }
        }
    }
    
    // Test 4: Check idle status
    log::info!("Test 4: Checking idle status...");
    unsafe {
        let idle = LcdCamHal::is_idle();
        log::info!("LCD_CAM idle status: {}", idle);
    }
    
    log::info!("LCD_CAM HAL test completed successfully!");
    log::info!("All tests passed. LCD_CAM peripheral is accessible.");
    
    // Keep running to show success
    loop {
        log::info!("LCD_CAM HAL test running...");
        FreeRtos::delay_ms(1000);
        unsafe { esp_task_wdt_reset(); }
    }
}