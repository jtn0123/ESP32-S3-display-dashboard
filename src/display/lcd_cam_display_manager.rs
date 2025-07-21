/// Hardware-accelerated DisplayManager using LCD_CAM
use super::lcd_cam_esp_hal::LcdCamDisplay;
use super::colors;
use super::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use anyhow::Result;
use esp_idf_hal::gpio::{Gpio39, Gpio40, Gpio41, Gpio42, Gpio45, Gpio46, Gpio47, Gpio48};
use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Gpio8, Gpio15, Gpio38, Gpio9};
use esp_idf_hal::gpio::{PinDriver, Output};
use std::time::Instant;

pub struct LcdDisplayManager {
    display: LcdCamDisplay,
    backlight_pin: PinDriver<'static, Gpio38, Output>,
    lcd_power_pin: PinDriver<'static, Gpio15, Output>,
    _rd_pin: PinDriver<'static, Gpio9, Output>,
    pub width: u16,
    pub height: u16,
    last_activity: Instant,
}

impl LcdDisplayManager {
    pub fn new(
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
    ) -> Result<Self> {
        log::info!("Initializing hardware-accelerated LCD display...");
        
        // Set up power pins first
        let mut lcd_power_pin = PinDriver::output(lcd_power)?;
        lcd_power_pin.set_high()?;
        log::info!("LCD power enabled");
        
        // Set up RD pin (must be high for write mode)
        let mut rd_pin = PinDriver::output(rd)?;
        rd_pin.set_high()?;
        
        // Set up backlight
        let mut backlight_pin = PinDriver::output(backlight)?;
        backlight_pin.set_high()?;
        log::info!("Backlight enabled");
        
        // Initialize LCD_CAM display
        let display = LcdCamDisplay::new(d0, d1, d2, d3, d4, d5, d6, d7, wr, dc, cs, rst)?;
        
        let width = display.width();
        let height = display.height();
        
        let mut manager = Self {
            display,
            backlight_pin,
            lcd_power_pin,
            _rd_pin: rd_pin,
            width,
            height,
            last_activity: Instant::now(),
        };
        
        // Clear to black
        manager.clear(colors::BLACK)?;
        
        log::info!("LCD display initialized with hardware acceleration!");
        Ok(manager)
    }
    
    pub fn clear(&mut self, color: u16) -> Result<()> {
        self.display.clear(color)
    }
    
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        self.display.draw_pixel(x, y, color)
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        self.display.fill_rect(x, y, w, h, color)
    }
    
    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> Result<()> {
        // Bresenham's line algorithm
        let dx = (x1 as i32 - x0 as i32).abs();
        let dy = (y1 as i32 - y0 as i32).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0 as i32;
        let mut y = y0 as i32;

        loop {
            self.draw_pixel(x as u16, y as u16, color)?;

            if x == x1 as i32 && y == y1 as i32 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }

        Ok(())
    }
    
    pub fn draw_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        self.draw_line(x, y, x + w - 1, y, color)?;
        self.draw_line(x, y + h - 1, x + w - 1, y + h - 1, color)?;
        self.draw_line(x, y, x, y + h - 1, color)?;
        self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, color)?;
        Ok(())
    }
    
    pub fn draw_char(&mut self, x: u16, y: u16, c: char, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let char_data = get_char_data(c);
        
        // Calculate character dimensions
        let char_width = FONT_WIDTH * scale;
        let char_height = FONT_HEIGHT * scale;
        
        // If we have a background color, fill the entire character area first
        if let Some(bg) = bg_color {
            self.fill_rect(x, y, char_width as u16, char_height as u16, bg)?;
        }
        
        // Draw character pixels
        for row in 0..FONT_HEIGHT {
            for col in 0..FONT_WIDTH {
                let pixel_on = (char_data[col as usize] >> row) & 1 == 1;
                
                if pixel_on {
                    let px = x + (col * scale) as u16;
                    let py = y + (row * scale) as u16;
                    self.fill_rect(px, py, scale as u16, scale as u16, color)?;
                }
            }
        }
        
        Ok(())
    }
    
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let mut cursor_x = x;
        let char_width = (FONT_WIDTH * scale + 1) as u16;
        
        for c in text.chars() {
            if cursor_x + char_width > self.width {
                break;
            }
            
            self.draw_char(cursor_x, y, c, color, bg_color, scale)?;
            cursor_x += char_width;
        }
        
        Ok(())
    }
    
    pub fn draw_text_centered(&mut self, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let char_width = (FONT_WIDTH * scale + 1) as u16;
        let text_width = text.len() as u16 * char_width;
        let x = (self.width - text_width) / 2;
        
        self.draw_text(x, y, text, color, bg_color, scale)
    }
    
    pub fn draw_circle(&mut self, cx: u16, cy: u16, r: u16, color: u16) -> Result<()> {
        let mut x = r as i32;
        let mut y = 0i32;
        let mut err = 0i32;

        while x >= y {
            self.draw_pixel((cx as i32 + x) as u16, (cy as i32 + y) as u16, color)?;
            self.draw_pixel((cx as i32 + y) as u16, (cy as i32 + x) as u16, color)?;
            self.draw_pixel((cx as i32 - y) as u16, (cy as i32 + x) as u16, color)?;
            self.draw_pixel((cx as i32 - x) as u16, (cy as i32 + y) as u16, color)?;
            self.draw_pixel((cx as i32 - x) as u16, (cy as i32 - y) as u16, color)?;
            self.draw_pixel((cx as i32 - y) as u16, (cy as i32 - x) as u16, color)?;
            self.draw_pixel((cx as i32 + y) as u16, (cy as i32 - x) as u16, color)?;
            self.draw_pixel((cx as i32 + x) as u16, (cy as i32 - y) as u16, color)?;

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }

        Ok(())
    }
    
    pub fn fill_circle(&mut self, cx: u16, cy: u16, r: u16, color: u16) -> Result<()> {
        for dy in 0..=r as i32 {
            let dx = ((r as i32 * r as i32 - dy * dy) as f32).sqrt() as i32;
            
            if dx > 0 {
                let x_start = (cx as i32 - dx).max(0) as u16;
                let x_end = (cx as i32 + dx).min(self.width as i32 - 1) as u16;
                let width = x_end - x_start + 1;
                
                if cy as i32 - dy >= 0 {
                    self.fill_rect(x_start, (cy as i32 - dy) as u16, width, 1, color)?;
                }
                
                if dy > 0 && cy as i32 + dy < self.height as i32 {
                    self.fill_rect(x_start, (cy as i32 + dy) as u16, width, 1, color)?;
                }
            }
        }
        Ok(())
    }
    
    pub fn draw_progress_bar(&mut self, x: u16, y: u16, w: u16, h: u16, progress: u8, fg_color: u16, bg_color: u16, border_color: u16) -> Result<()> {
        // Draw border
        self.draw_rect(x, y, w, h, border_color)?;
        
        // Fill background
        self.fill_rect(x + 1, y + 1, w - 2, h - 2, bg_color)?;
        
        // Fill progress
        let progress_width = ((w - 2) as u32 * progress as u32 / 100) as u16;
        if progress_width > 0 {
            self.fill_rect(x + 1, y + 1, progress_width, h - 2, fg_color)?;
        }
        
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        self.display.flush()
    }
    
    pub fn flush_region(&mut self, x: u16, y: u16, w: u16, h: u16) -> Result<()> {
        self.display.flush_region(x, y, w, h)
    }
    
    pub fn reset_activity_timer(&mut self) {
        self.last_activity = Instant::now();
    }
    
    pub fn ensure_display_on(&mut self) -> Result<()> {
        self.backlight_pin.set_high()?;
        Ok(())
    }
    
    pub fn width(&self) -> u16 {
        self.width
    }
    
    pub fn height(&self) -> u16 {
        self.height
    }
    
    pub fn benchmark(&mut self) -> Result<()> {
        super::lcd_cam_esp_hal::benchmark_lcd_cam(&mut self.display)
    }
}