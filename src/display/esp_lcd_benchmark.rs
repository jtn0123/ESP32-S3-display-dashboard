/// Performance benchmark for ESP LCD implementation
use super::lcd_cam_display_manager::LcdDisplayManager;
use super::lcd_cam_esp_hal::LcdCamDisplay;
use super::esp_lcd_config::{OptimizedLcdConfig, LcdClockSpeed};
use super::colors;
use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::delay::Ets;
use log::info;
use std::time::{Instant, Duration};

pub struct LcdBenchmark {
    display: LcdDisplayManager,
}

impl LcdBenchmark {
    pub fn new() -> Result<Self> {
        let peripherals = Peripherals::take().unwrap();
        
        let display = LcdDisplayManager::new(
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
        
        Ok(Self { display })
    }
    
    /// Benchmark full screen clear operations
    pub fn benchmark_clear(&mut self) -> Result<f32> {
        info!("[LCD_BENCH] Starting clear benchmark...");
        
        let iterations = 100;
        let start = Instant::now();
        
        for i in 0..iterations {
            // Alternate between black and white
            let color = if i % 2 == 0 { colors::BLACK } else { colors::WHITE };
            self.display.clear(color)?;
            self.display.flush()?;
        }
        
        let elapsed = start.elapsed();
        let fps = iterations as f32 / elapsed.as_secs_f32();
        
        info!("[LCD_BENCH] Clear benchmark: {:.1} FPS", fps);
        Ok(fps)
    }
    
    /// Benchmark rectangle drawing
    pub fn benchmark_rectangles(&mut self) -> Result<f32> {
        info!("[LCD_BENCH] Starting rectangle benchmark...");
        
        let iterations = 100;
        let start = Instant::now();
        
        for i in 0..iterations {
            self.display.clear(colors::BLACK)?;
            
            // Draw 10 rectangles per frame
            for j in 0..10 {
                let x = (j * 30) as u16;
                let y = (j * 15) as u16;
                let color = match j % 3 {
                    0 => colors::RED,
                    1 => colors::GREEN,
                    _ => colors::BLUE,
                };
                self.display.fill_rect(x, y, 25, 25, color)?;
            }
            
            self.display.flush()?;
        }
        
        let elapsed = start.elapsed();
        let fps = iterations as f32 / elapsed.as_secs_f32();
        
        info!("[LCD_BENCH] Rectangle benchmark: {:.1} FPS", fps);
        Ok(fps)
    }
    
    /// Benchmark text rendering
    pub fn benchmark_text(&mut self) -> Result<f32> {
        info!("[LCD_BENCH] Starting text benchmark...");
        
        let iterations = 100;
        let start = Instant::now();
        
        for i in 0..iterations {
            self.display.clear(colors::BLACK)?;
            
            // Draw multiple lines of text
            for line in 0..10 {
                let y = (line * 15 + 10) as u16;
                let text = format!("Line {} Frame {}", line, i);
                self.display.draw_text(10, y, &text, colors::WHITE, None, 1)?;
            }
            
            self.display.flush()?;
        }
        
        let elapsed = start.elapsed();
        let fps = iterations as f32 / elapsed.as_secs_f32();
        
        info!("[LCD_BENCH] Text benchmark: {:.1} FPS", fps);
        Ok(fps)
    }
    
    /// Benchmark with different pixel clock speeds
    pub fn benchmark_clock_speeds(&mut self) -> Result<()> {
        info!("[LCD_BENCH] Starting clock speed benchmarks...");
        
        // Note: Clock speed changes would require reinitializing the display
        // For now, just run at current speed
        let fps = self.benchmark_clear()?;
        
        info!("[LCD_BENCH] Current clock speed results: {:.1} FPS", fps);
        info!("[LCD_BENCH] To test different speeds, modify pclk_hz in lcd_cam_esp_hal.rs");
        
        Ok(())
    }
    
    /// Run all benchmarks
    pub fn run_all_benchmarks(&mut self) -> Result<()> {
        info!("[LCD_BENCH] Running comprehensive ESP LCD benchmarks...");
        info!("[LCD_BENCH] Display: 320x170, 8-bit parallel, DMA enabled");
        
        // Warm up
        info!("[LCD_BENCH] Warming up...");
        for _ in 0..10 {
            self.display.clear(colors::BLACK)?;
            self.display.flush()?;
        }
        
        // Run benchmarks
        let clear_fps = self.benchmark_clear()?;
        Ets::delay_ms(1000);
        
        let rect_fps = self.benchmark_rectangles()?;
        Ets::delay_ms(1000);
        
        let text_fps = self.benchmark_text()?;
        Ets::delay_ms(1000);
        
        // Summary
        info!("[LCD_BENCH] ===== BENCHMARK SUMMARY =====");
        info!("[LCD_BENCH] Clear FPS: {:.1}", clear_fps);
        info!("[LCD_BENCH] Rectangle FPS: {:.1}", rect_fps);
        info!("[LCD_BENCH] Text FPS: {:.1}", text_fps);
        info!("[LCD_BENCH] Average FPS: {:.1}", (clear_fps + rect_fps + text_fps) / 3.0);
        
        // Compare to GPIO baseline
        info!("[LCD_BENCH] ===== COMPARISON =====");
        info!("[LCD_BENCH] GPIO baseline: ~10 FPS");
        info!("[LCD_BENCH] Improvement: {:.1}x", clear_fps / 10.0);
        
        Ok(())
    }
}