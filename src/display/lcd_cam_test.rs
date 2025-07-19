/// Test program for LCD_CAM display driver
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_sys::*;
use super::lcd_cam_display::LcdCamDisplay;
use super::colors::{PRIMARY_RED, rgb565};

/// Run LCD_CAM toggle color test
pub fn lcd_cam_toggle_test(
    d0: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d1: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d2: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d3: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d4: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d5: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d6: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    d7: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    wr: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    dc: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    cs: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
    rst: impl Into<esp_idf_hal::gpio::AnyIOPin> + 'static,
) -> Result<()> {
    log::warn!("Starting LCD_CAM toggle test...");
    
    // Initialize LCD power pin (GPIO 15) - CRITICAL!
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        log::info!("LCD power enabled on GPIO 15");
    }
    
    // Initialize backlight pin (GPIO 38)
    unsafe {
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
        log::info!("Backlight enabled on GPIO 38");
    }
    
    // Wait for power to stabilize
    FreeRtos::delay_ms(100);
    
    // Create LCD_CAM display
    let mut display = LcdCamDisplay::new(
        d0, d1, d2, d3, d4, d5, d6, d7,
        wr, dc, cs, rst
    )?;
    
    log::info!("LCD_CAM display created successfully");
    
    let start_time = unsafe { esp_idf_sys::esp_timer_get_time() };
    let mut frame_count = 0u32;
    
    loop {
        // Red frame
        display.clear(PRIMARY_RED)?;
        frame_count += 1;
        
        // Small delay to make color visible
        FreeRtos::delay_ms(16);
        
        // Cyan frame
        display.clear(rgb565(0, 255, 255))?; // Cyan
        frame_count += 1;
        
        FreeRtos::delay_ms(16);
        
        // Log performance every 60 frames
        if frame_count % 60 == 0 {
            let elapsed_us = (unsafe { esp_idf_sys::esp_timer_get_time() } - start_time) as u64;
            let fps = (frame_count as u64 * 1_000_000) / elapsed_us;
            
            let (_display_frames, dma_frames) = display.get_stats();
            log::info!("LCD_CAM: {} frames, {} FPS, DMA: {}", 
                      frame_count, fps, dma_frames);
            
            // Reset watchdog
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
        
        // Check for issues
        if frame_count > 10 && frame_count % 10 == 0 {
            let (display_frames, dma_frames) = display.get_stats();
            if display_frames != dma_frames {
                log::error!("Frame mismatch! Display: {}, DMA: {}", 
                          display_frames, dma_frames);
            }
        }
    }
}