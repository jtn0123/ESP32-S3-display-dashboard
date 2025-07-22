/// Pixel test patterns for verifying display coordinate mapping and color accuracy
use super::colors;
use anyhow::Result;
use log::info;

pub trait DisplayDriver {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()>;
    fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

pub struct TestPatterns;

impl TestPatterns {
    /// Draw corner pixels to verify coordinate system
    pub fn corner_pixels<D: DisplayDriver>(display: &mut D) -> Result<()> {
        info!("[TEST] Drawing corner pixels...");
        let w = display.width();
        let h = display.height();
        
        // Clear screen first
        display.fill_rect(0, 0, w, h, colors::BLACK)?;
        
        // Top-left (RED)
        display.set_pixel(0, 0, colors::RED)?;
        display.set_pixel(1, 0, colors::RED)?;
        display.set_pixel(0, 1, colors::RED)?;
        
        // Top-right (GREEN)
        display.set_pixel(w - 1, 0, colors::GREEN)?;
        display.set_pixel(w - 2, 0, colors::GREEN)?;
        display.set_pixel(w - 1, 1, colors::GREEN)?;
        
        // Bottom-left (BLUE)
        display.set_pixel(0, h - 1, colors::BLUE)?;
        display.set_pixel(1, h - 1, colors::BLUE)?;
        display.set_pixel(0, h - 2, colors::BLUE)?;
        
        // Bottom-right (WHITE)
        display.set_pixel(w - 1, h - 1, colors::WHITE)?;
        display.set_pixel(w - 2, h - 1, colors::WHITE)?;
        display.set_pixel(w - 1, h - 2, colors::WHITE)?;
        
        display.flush()?;
        info!("[TEST] Corner pixels: RED(top-left), GREEN(top-right), BLUE(bottom-left), WHITE(bottom-right)");
        Ok(())
    }
    
    /// Draw a grid pattern to check pixel alignment
    pub fn grid_pattern<D: DisplayDriver>(display: &mut D, spacing: u16) -> Result<()> {
        info!("[TEST] Drawing grid pattern with {}px spacing...", spacing);
        let w = display.width();
        let h = display.height();
        
        // Clear screen
        display.fill_rect(0, 0, w, h, colors::BLACK)?;
        
        // Vertical lines
        for x in (0..w).step_by(spacing as usize) {
            for y in 0..h {
                display.set_pixel(x, y, colors::GRAY)?;
            }
        }
        
        // Horizontal lines
        for y in (0..h).step_by(spacing as usize) {
            for x in 0..w {
                display.set_pixel(x, y, colors::GRAY)?;
            }
        }
        
        // Mark origin with red cross
        for i in 0..10 {
            display.set_pixel(i, 0, colors::RED)?;
            display.set_pixel(0, i, colors::RED)?;
        }
        
        display.flush()?;
        info!("[TEST] Grid pattern complete - origin marked in RED");
        Ok(())
    }
    
    /// Draw color bars to verify RGB order
    pub fn color_bars<D: DisplayDriver>(display: &mut D) -> Result<()> {
        info!("[TEST] Drawing color bars...");
        let w = display.width();
        let h = display.height();
        let bar_width = w / 8;
        
        let colors_array = [
            colors::WHITE,
            colors::YELLOW,
            colors::CYAN,
            colors::GREEN,
            colors::MAGENTA,
            colors::RED,
            colors::BLUE,
            colors::BLACK,
        ];
        
        for (i, &color) in colors_array.iter().enumerate() {
            let x = (i as u16) * bar_width;
            let width = if i == 7 { w - x } else { bar_width }; // Last bar takes remaining space
            display.fill_rect(x, 0, width, h, color)?;
        }
        
        display.flush()?;
        info!("[TEST] Color bars: WHITE, YELLOW, CYAN, GREEN, MAGENTA, RED, BLUE, BLACK");
        Ok(())
    }
    
    /// Draw a border around the display edges
    pub fn border_test<D: DisplayDriver>(display: &mut D, thickness: u16) -> Result<()> {
        info!("[TEST] Drawing border with {}px thickness...", thickness);
        let w = display.width();
        let h = display.height();
        
        // Clear screen
        display.fill_rect(0, 0, w, h, colors::BLACK)?;
        
        // Top border (RED)
        display.fill_rect(0, 0, w, thickness, colors::RED)?;
        
        // Bottom border (GREEN)  
        display.fill_rect(0, h - thickness, w, thickness, colors::GREEN)?;
        
        // Left border (BLUE)
        display.fill_rect(0, 0, thickness, h, colors::BLUE)?;
        
        // Right border (YELLOW)
        display.fill_rect(w - thickness, 0, thickness, h, colors::YELLOW)?;
        
        display.flush()?;
        info!("[TEST] Border test: RED(top), GREEN(bottom), BLUE(left), YELLOW(right)");
        Ok(())
    }
    
    /// Draw diagonal lines to test coordinate mapping
    pub fn diagonal_lines<D: DisplayDriver>(display: &mut D) -> Result<()> {
        info!("[TEST] Drawing diagonal lines...");
        let w = display.width();
        let h = display.height();
        
        // Clear screen
        display.fill_rect(0, 0, w, h, colors::BLACK)?;
        
        // Top-left to bottom-right (RED)
        let steps = w.min(h);
        for i in 0..steps {
            let x = (i as u32 * w as u32 / steps as u32) as u16;
            let y = (i as u32 * h as u32 / steps as u32) as u16;
            display.set_pixel(x, y, colors::RED)?;
            if x > 0 { display.set_pixel(x - 1, y, colors::RED)?; }
            if x < w - 1 { display.set_pixel(x + 1, y, colors::RED)?; }
        }
        
        // Top-right to bottom-left (GREEN)
        for i in 0..steps {
            let x = w - 1 - (i as u32 * w as u32 / steps as u32) as u16;
            let y = (i as u32 * h as u32 / steps as u32) as u16;
            display.set_pixel(x, y, colors::GREEN)?;
            if x > 0 { display.set_pixel(x - 1, y, colors::GREEN)?; }
            if x < w - 1 { display.set_pixel(x + 1, y, colors::GREEN)?; }
        }
        
        display.flush()?;
        info!("[TEST] Diagonal lines: RED(TL->BR), GREEN(TR->BL)");
        Ok(())
    }
    
    /// Fill screen with incrementing pattern to detect skipped pixels
    pub fn pixel_counter<D: DisplayDriver>(display: &mut D) -> Result<()> {
        info!("[TEST] Drawing pixel counter pattern...");
        let w = display.width();
        let h = display.height();
        
        // Create a pattern that changes every pixel
        for y in 0..h {
            for x in 0..w {
                // Create a color based on position
                let r = ((x * 31 / w) as u16) << 11;
                let g = ((y * 63 / h) as u16) << 5;
                let b = ((x + y) * 31 / (w + h)) as u16;
                let color = r | g | b;
                display.set_pixel(x, y, color)?;
            }
        }
        
        display.flush()?;
        info!("[TEST] Pixel counter pattern - should show smooth gradients");
        Ok(())
    }
    
    /// Test specific problem areas for T-Display-S3
    pub fn t_display_s3_test<D: DisplayDriver>(display: &mut D) -> Result<()> {
        info!("[TEST] T-Display-S3 specific test...");
        let w = display.width();
        let h = display.height();
        
        // Clear screen
        display.fill_rect(0, 0, w, h, colors::BLACK)?;
        
        // Draw text indicating display size
        // Top area - should be visible
        display.fill_rect(0, 0, w, 20, colors::BLUE)?;
        
        // Draw markers at key positions
        // Y=35 offset marker (where visible area starts in some configs)
        if h > 35 {
            display.fill_rect(0, 35, w, 2, colors::RED)?;
        }
        
        // Center cross
        let cx = w / 2;
        let cy = h / 2;
        display.fill_rect(cx - 20, cy, 40, 1, colors::WHITE)?;
        display.fill_rect(cx, cy - 20, 1, 40, colors::WHITE)?;
        
        // Size indicators
        info!("[TEST] Display size: {}x{}", w, h);
        info!("[TEST] Center at: {},{}", cx, cy);
        
        display.flush()?;
        Ok(())
    }
}

/// Run all test patterns in sequence
pub fn run_all_tests<D: DisplayDriver>(display: &mut D, delay_ms: u32) -> Result<()> {
    use esp_idf_hal::delay::Ets;
    
    info!("[TEST] Starting display test sequence...");
    
    // Test 1: Corner pixels
    TestPatterns::corner_pixels(display)?;
    Ets::delay_ms(delay_ms);
    
    // Test 2: Grid pattern
    TestPatterns::grid_pattern(display, 20)?;
    Ets::delay_ms(delay_ms);
    
    // Test 3: Color bars
    TestPatterns::color_bars(display)?;
    Ets::delay_ms(delay_ms);
    
    // Test 4: Border test
    TestPatterns::border_test(display, 5)?;
    Ets::delay_ms(delay_ms);
    
    // Test 5: Diagonal lines
    TestPatterns::diagonal_lines(display)?;
    Ets::delay_ms(delay_ms);
    
    // Test 6: Pixel counter
    TestPatterns::pixel_counter(display)?;
    Ets::delay_ms(delay_ms);
    
    // Test 7: T-Display-S3 specific
    TestPatterns::t_display_s3_test(display)?;
    Ets::delay_ms(delay_ms);
    
    info!("[TEST] Display test sequence complete!");
    Ok(())
}