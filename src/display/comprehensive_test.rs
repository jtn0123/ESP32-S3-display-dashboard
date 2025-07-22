/// Comprehensive display test to isolate issues
use anyhow::Result;
use esp_idf_sys::*;
use log::{info, error};
use esp_idf_hal::delay::Ets;
use super::colors;
use super::esp_lcd_chunk_wrapper::safe_draw_bitmap;

pub fn run_comprehensive_test(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    info!("=== COMPREHENSIVE DISPLAY TEST ===");
    
    unsafe {
        // Test 1: Basic functionality
        info!("Test 1: Panel handle validity");
        if panel_handle.is_null() {
            error!("Panel handle is NULL!");
            return Err(anyhow::anyhow!("Invalid panel handle"));
        } else {
            info!("✓ Panel handle is valid: {:?}", panel_handle);
        }
        
        // Test 2: Try different coordinate systems
        info!("Test 2: Testing different coordinate systems");
        
        // Test 2a: Small rectangle at origin
        info!("  2a: Drawing 10x10 red square at (0,0)");
        let red_pixels: Vec<u16> = vec![colors::RED; 100];
        safe_draw_bitmap(
            panel_handle,
            0, 0,
            10, 10,
            red_pixels.as_ptr() as *const _
        )?;
        info!("  ✓ Red square drawn");
        Ets::delay_ms(500);
        esp_task_wdt_reset();
        
        // Test 2b: Rectangle with offset
        info!("  2b: Drawing 20x20 green square at (50,50)");
        let green_pixels: Vec<u16> = vec![colors::GREEN; 400];
        safe_draw_bitmap(
            panel_handle,
            50, 50,
            70, 70,
            green_pixels.as_ptr() as *const _
        )?;
        info!("  ✓ Green square drawn");
        Ets::delay_ms(500);
        esp_task_wdt_reset();
        
        // Test 2c: Full width line
        info!("  2c: Drawing full width blue line at y=100");
        let blue_pixels: Vec<u16> = vec![colors::BLUE; 320];
        let ret = esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 100,
            320, 101,
            blue_pixels.as_ptr() as *const _
        );
        info!("  Result: {} (0x{:X})", ret, ret);
        Ets::delay_ms(500);
        
        // Test 3: Different pixel formats
        info!("Test 3: Testing pixel format variations");
        
        // Test 3a: White pixels (all bits set)
        info!("  3a: Drawing white rectangle");
        let white_pixels: Vec<u16> = vec![0xFFFF; 100];
        let ret = esp_lcd_panel_draw_bitmap(
            panel_handle,
            100, 10,
            110, 20,
            white_pixels.as_ptr() as *const _
        );
        info!("  Result: {} (0x{:X})", ret, ret);
        
        // Test 3b: Pattern
        info!("  3b: Drawing checkerboard pattern");
        let mut pattern: Vec<u16> = Vec::with_capacity(400);
        for y in 0..20 {
            for x in 0..20 {
                if (x + y) % 2 == 0 {
                    pattern.push(colors::WHITE);
                } else {
                    pattern.push(colors::BLACK);
                }
            }
        }
        let ret = esp_lcd_panel_draw_bitmap(
            panel_handle,
            150, 50,
            170, 70,
            pattern.as_ptr() as *const _
        );
        info!("  Result: {} (0x{:X})", ret, ret);
        
        // Test 4: Boundary tests
        info!("Test 4: Testing display boundaries");
        
        // Test 4a: Corner pixels
        info!("  4a: Drawing pixels at corners");
        let corner_pixel: [u16; 1] = [colors::YELLOW];
        
        // Top-left
        esp_lcd_panel_draw_bitmap(panel_handle, 0, 0, 1, 1, corner_pixel.as_ptr() as *const _);
        // Top-right
        esp_lcd_panel_draw_bitmap(panel_handle, 319, 0, 320, 1, corner_pixel.as_ptr() as *const _);
        // Bottom-left
        esp_lcd_panel_draw_bitmap(panel_handle, 0, 169, 1, 170, corner_pixel.as_ptr() as *const _);
        // Bottom-right
        esp_lcd_panel_draw_bitmap(panel_handle, 319, 169, 320, 170, corner_pixel.as_ptr() as *const _);
        info!("  ✓ Corner pixels drawn");
        
        // Test 5: Performance test
        info!("Test 5: Performance test - rapid updates");
        let start = std::time::Instant::now();
        for i in 0..10 {
            let color = if i % 2 == 0 { colors::RED } else { colors::GREEN };
            let pixels: Vec<u16> = vec![color; 3200]; // 100x32 rectangle
            let ret = esp_lcd_panel_draw_bitmap(
                panel_handle,
                0, 50,
                100, 82,
                pixels.as_ptr() as *const _
            );
            if ret != ESP_OK {
                error!("  Draw failed at iteration {}: {}", i, ret);
            }
        }
        let elapsed = start.elapsed();
        info!("  ✓ 10 draws completed in {:?}", elapsed);
        
        // Test 6: Memory alignment test
        info!("Test 6: Testing memory alignment");
        
        // Test with odd-sized buffers
        let odd_pixels: Vec<u16> = vec![colors::CYAN; 17];
        let ret = esp_lcd_panel_draw_bitmap(
            panel_handle,
            10, 150,
            27, 151,
            odd_pixels.as_ptr() as *const _
        );
        info!("  Odd buffer result: {} (0x{:X})", ret, ret);
        
        info!("=== COMPREHENSIVE TEST COMPLETE ===");
        info!("Check display for:");
        info!("  - Red square at top-left");
        info!("  - Green square at (50,50)");
        info!("  - Blue line across screen");
        info!("  - White rectangle");
        info!("  - Checkerboard pattern");
        info!("  - Yellow corner pixels");
        info!("  - Flashing red/green rectangle");
        info!("  - Cyan line near bottom");
    }
    
    Ok(())
}