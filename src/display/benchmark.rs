/// Display performance benchmark
/// Compares GPIO bit-bang vs ESP_LCD DMA implementations

use anyhow::Result;
use std::time::Instant;
use log::info;

#[cfg(not(feature = "esp_lcd_driver"))]
use super::DisplayManager;

#[cfg(feature = "esp_lcd_driver")]
use super::esp_lcd_display_manager::EspLcdDisplayManager as DisplayManager;

use super::colors::*;

pub struct BenchmarkResults {
    pub clear_screen_ms: f32,
    pub draw_1000_pixels_ms: f32,
    pub draw_100_rects_ms: f32,
    pub draw_text_ms: f32,
    pub full_frame_update_ms: f32,
    pub partial_update_ms: f32,
    pub max_theoretical_fps: f32,
}

impl BenchmarkResults {
    pub fn print_summary(&self, driver_name: &str) {
        info!("=== {} Performance Benchmark ===", driver_name);
        info!("Clear screen: {:.2}ms", self.clear_screen_ms);
        info!("Draw 1000 pixels: {:.2}ms", self.draw_1000_pixels_ms);
        info!("Draw 100 rectangles: {:.2}ms", self.draw_100_rects_ms);
        info!("Draw text (100 chars): {:.2}ms", self.draw_text_ms);
        info!("Full frame update: {:.2}ms", self.full_frame_update_ms);
        info!("Partial update (10%): {:.2}ms", self.partial_update_ms);
        info!("Max theoretical FPS: {:.1}", self.max_theoretical_fps);
        info!("=====================================");
    }
}

pub fn run_display_benchmark(display: &mut DisplayManager) -> Result<BenchmarkResults> {
    info!("Starting display performance benchmark...");
    
    // Test 1: Clear screen
    let start = Instant::now();
    display.clear(BLACK)?;
    display.flush()?;
    let clear_screen_ms = start.elapsed().as_secs_f32() * 1000.0;
    
    // Test 2: Draw 1000 random pixels
    let start = Instant::now();
    for i in 0..1000 {
        let x = (i * 37) % 320;
        let y = (i * 23) % 170;
        let color = if i % 2 == 0 { WHITE } else { PRIMARY_RED };
        display.draw_pixel(x as u16, y as u16, color)?;
    }
    display.flush()?;
    let draw_1000_pixels_ms = start.elapsed().as_secs_f32() * 1000.0;
    
    // Test 3: Draw 100 rectangles
    display.clear(BLACK)?;
    display.flush()?;
    let start = Instant::now();
    for i in 0..100 {
        let x = (i * 13) % 280;
        let y = (i * 11) % 130;
        let color = match i % 3 {
            0 => PRIMARY_RED,
            1 => WHITE,
            _ => TEXT_SECONDARY,
        };
        display.draw_rect(x as u16, y as u16, 30, 20, color)?;
    }
    display.flush()?;
    let draw_100_rects_ms = start.elapsed().as_secs_f32() * 1000.0;
    
    // Test 4: Draw text
    display.clear(BLACK)?;
    display.flush()?;
    let start = Instant::now();
    let test_text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    for i in 0..3 {
        display.draw_text(10, (20 + i * 20) as u16, test_text, WHITE, Some(BLACK), 1)?;
    }
    display.flush()?;
    let draw_text_ms = start.elapsed().as_secs_f32() * 1000.0;
    
    // Test 5: Full frame update (worst case)
    let start = Instant::now();
    display.clear(PRIMARY_RED)?;
    display.flush()?;
    display.clear(WHITE)?;
    display.flush()?;
    let full_frame_update_ms = (start.elapsed().as_secs_f32() * 1000.0) / 2.0;
    
    // Test 6: Partial update (typical UI change)
    display.clear(BLACK)?;
    display.flush()?;
    let start = Instant::now();
    // Update only 10% of screen
    display.draw_rect(100, 50, 120, 68, WHITE)?;
    display.flush()?;
    let partial_update_ms = start.elapsed().as_secs_f32() * 1000.0;
    
    // Calculate theoretical max FPS
    let max_theoretical_fps = 1000.0 / full_frame_update_ms;
    
    Ok(BenchmarkResults {
        clear_screen_ms,
        draw_1000_pixels_ms,
        draw_100_rects_ms,
        draw_text_ms,
        full_frame_update_ms,
        partial_update_ms,
        max_theoretical_fps,
    })
}

/// Compare performance between implementations
pub fn compare_implementations() -> Result<()> {
    #[cfg(feature = "esp_lcd_driver")]
    let driver_name = "ESP_LCD DMA Driver";
    
    #[cfg(not(feature = "esp_lcd_driver"))]
    let driver_name = "GPIO Bit-Bang Driver";
    
    info!("Running benchmark for: {}", driver_name);
    
    // Note: In real usage, we'd create a display instance here
    // For now, this is a template for when the hardware test succeeds
    
    Ok(())
}