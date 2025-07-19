use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use super::DisplayManager;
use super::colors::{BLACK, WHITE, PRIMARY_BLUE, PRIMARY_GREEN, PRIMARY_RED, YELLOW, PRIMARY_PURPLE, rgb565};

/// Simple toggle test to validate display timing
/// This should show alternating red/cyan screens every 16ms
/// If this works, basic display communication is good
pub fn toggle_color_test(display: &mut DisplayManager) -> Result<()> {
    log::warn!("Starting toggle color test - display should alternate red/cyan");
    log::warn!("Press reset to exit test");
    
    let mut frame_count = 0u32;
    let start_time = unsafe { esp_idf_sys::esp_timer_get_time() };
    
    loop {
        // Red frame
        display.clear(PRIMARY_RED)?;
        // Remove delay to test raw performance
        // FreeRtos::delay_ms(16);
        
        // Cyan frame  
        display.clear(rgb565(0, 255, 255))?; // Cyan
        // Remove delay to test raw performance
        // FreeRtos::delay_ms(16);
        
        frame_count += 2;
        
        // Log FPS every 60 frames
        if frame_count % 60 == 0 {
            let elapsed_us = (unsafe { esp_idf_sys::esp_timer_get_time() } - start_time) as u64;
            let fps = (frame_count as u64 * 1_000_000) / elapsed_us;
            log::info!("Toggle test: {} frames, {} FPS", frame_count, fps);
            
            // Reset watchdog
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    }
}

/// Test display write performance with pattern
pub fn benchmark_display_write(display: &mut DisplayManager) -> Result<()> {
    log::info!("Starting display write benchmark...");
    
    const ITERATIONS: u32 = 100;
    let mut total_time_us = 0u64;
    
    // Test 1: Full screen clear
    for i in 0..ITERATIONS {
        let color = if i % 2 == 0 { BLACK } else { WHITE };
        let start = unsafe { esp_idf_sys::esp_timer_get_time() };
        
        display.clear(color)?;
        
        let elapsed = (unsafe { esp_idf_sys::esp_timer_get_time() } - start) as u64;
        total_time_us += elapsed;
        
        if i % 10 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    }
    
    let avg_clear_us = total_time_us / ITERATIONS as u64;
    let clear_fps = 1_000_000 / avg_clear_us;
    log::info!("Clear performance: {} us/frame ({} FPS)", avg_clear_us, clear_fps);
    
    // Test 2: Small rectangle updates
    total_time_us = 0;
    for i in 0..ITERATIONS {
        let start = unsafe { esp_idf_sys::esp_timer_get_time() };
        
        // Draw 10 small rectangles
        for j in 0..10 {
            let x = (j * 30) as u16;
            let y = (i % 10 * 16) as u16;
            display.fill_rect(x, y, 25, 12, PRIMARY_BLUE)?;
        }
        
        let elapsed = (unsafe { esp_idf_sys::esp_timer_get_time() } - start) as u64;
        total_time_us += elapsed;
    }
    
    let avg_rect_us = total_time_us / ITERATIONS as u64;
    log::info!("10 rects performance: {} us/frame", avg_rect_us);
    
    // Test 3: Pixel-by-pixel (worst case)
    let start = unsafe { esp_idf_sys::esp_timer_get_time() };
    for y in 0..50 {
        for x in 0..100 {
            display.draw_pixel(x, y, PRIMARY_GREEN)?;
        }
        if y % 10 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    }
    let pixel_time = (unsafe { esp_idf_sys::esp_timer_get_time() } - start) as u64;
    let pixels_per_sec = (5000 * 1_000_000) / pixel_time;
    log::info!("Pixel performance: {} pixels/sec", pixels_per_sec);
    
    Ok(())
}

/// Verify display boundaries are correct
pub fn boundary_test(display: &mut DisplayManager) -> Result<()> {
    log::info!("Starting boundary test - should see colored border");
    
    // Clear to black
    display.clear(BLACK)?;
    
    // Draw red border at exact display boundaries
    let width = display.width();
    let height = display.height();
    
    // Top edge (red)
    for x in 0..width {
        display.draw_pixel(x, 0, PRIMARY_RED)?;
    }
    
    // Bottom edge (green)
    for x in 0..width {
        display.draw_pixel(x, height - 1, PRIMARY_GREEN)?;
    }
    
    // Left edge (blue)
    for y in 0..height {
        display.draw_pixel(0, y, PRIMARY_BLUE)?;
    }
    
    // Right edge (yellow)  
    for y in 0..height {
        display.draw_pixel(width - 1, y, YELLOW)?;
    }
    
    // Draw corner markers (white)
    for i in 0..10 {
        // Top-left
        display.draw_pixel(i, 0, WHITE)?;
        display.draw_pixel(0, i, WHITE)?;
        
        // Top-right
        display.draw_pixel(width - 1 - i, 0, WHITE)?;
        display.draw_pixel(width - 1, i, WHITE)?;
        
        // Bottom-left
        display.draw_pixel(i, height - 1, WHITE)?;
        display.draw_pixel(0, height - 1 - i, WHITE)?;
        
        // Bottom-right
        display.draw_pixel(width - 1 - i, height - 1, WHITE)?;
        display.draw_pixel(width - 1, height - 1 - i, WHITE)?;
    }
    
    log::info!("Boundary test complete - check display edges");
    Ok(())
}

/// Memory pattern test to detect addressing issues
pub fn memory_pattern_test(display: &mut DisplayManager) -> Result<()> {
    log::info!("Starting memory pattern test");
    
    // Pattern 1: Vertical stripes
    for x in 0..display.width() {
        let color = if x % 10 < 5 { WHITE } else { BLACK };
        for y in 0..display.height() {
            display.draw_pixel(x, y, color)?;
        }
        if x % 50 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    }
    FreeRtos::delay_ms(1000);
    
    // Pattern 2: Horizontal stripes
    for y in 0..display.height() {
        let color = if y % 10 < 5 { PRIMARY_RED } else { PRIMARY_BLUE };
        for x in 0..display.width() {
            display.draw_pixel(x, y, color)?;
        }
    }
    FreeRtos::delay_ms(1000);
    
    // Pattern 3: Checkerboard
    for y in 0..display.height() {
        for x in 0..display.width() {
            let color = if (x + y) % 2 == 0 { PRIMARY_GREEN } else { PRIMARY_PURPLE };
            display.draw_pixel(x, y, color)?;
        }
        if y % 20 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    }
    
    log::info!("Memory pattern test complete");
    Ok(())
}