pub mod colors;
pub mod font5x7;
pub mod lcd_bus;

// Color type not used - colors are defined as u16 constants

use anyhow::Result;
use self::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use self::lcd_bus::LcdBus;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, Output};
use esp_idf_hal::delay::FreeRtos;
use std::time::Instant;

// Display boundaries from Arduino verified testing
const DISPLAY_X_START: u16 = 10;   // Left boundary 
const DISPLAY_Y_START: u16 = 36;   // Top boundary
const DISPLAY_WIDTH: u16 = 300;    // Maximum visible width
const DISPLAY_HEIGHT: u16 = 168;   // Maximum visible height

// Controller dimensions removed - not used

// Dirty rectangle tracking removed - not implemented

// ST7789 Commands
const CMD_NOP: u8 = 0x00;
const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
const CMD_INVON: u8 = 0x21;
const CMD_DISPON: u8 = 0x29;
const CMD_CASET: u8 = 0x2A;
const CMD_RASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_MADCTL: u8 = 0x36;
const CMD_COLMOD: u8 = 0x3A;
const CMD_PORCTRL: u8 = 0xB2;
const CMD_GCTRL: u8 = 0xB7;
const CMD_VCOMS: u8 = 0xBB;
const CMD_LCMCTRL: u8 = 0xC0;
const CMD_VDVVRHEN: u8 = 0xC2;
const CMD_VRHS: u8 = 0xC3;
const CMD_VDVS: u8 = 0xC4;
const CMD_FRCTRL2: u8 = 0xC6;
const CMD_PWRCTRL1: u8 = 0xD0;

pub struct DisplayManager {
    lcd_bus: LcdBus,
    backlight_pin: Option<PinDriver<'static, AnyIOPin, Output>>, // Keep backlight alive
    lcd_power_pin: Option<PinDriver<'static, AnyIOPin, Output>>, // Keep LCD power alive
    rd_pin: Option<PinDriver<'static, AnyIOPin, Output>>, // Keep RD pin high
    width: u16,
    height: u16,
    last_activity: Instant,
}

impl DisplayManager {
    pub fn new(
        d0: impl Into<AnyIOPin> + 'static,
        d1: impl Into<AnyIOPin> + 'static,
        d2: impl Into<AnyIOPin> + 'static,
        d3: impl Into<AnyIOPin> + 'static,
        d4: impl Into<AnyIOPin> + 'static,
        d5: impl Into<AnyIOPin> + 'static,
        d6: impl Into<AnyIOPin> + 'static,
        d7: impl Into<AnyIOPin> + 'static,
        wr: impl Into<AnyIOPin> + 'static,
        dc: impl Into<AnyIOPin> + 'static,
        cs: impl Into<AnyIOPin> + 'static,
        rst: impl Into<AnyIOPin> + 'static,
        backlight: impl Into<AnyIOPin> + 'static,
        lcd_power: impl Into<AnyIOPin> + 'static,
        rd: impl Into<AnyIOPin> + 'static,
    ) -> Result<Self> {
        // For now, use simple GPIO for backlight
        use esp_idf_hal::gpio::PinDriver;
        let mut backlight_pin = PinDriver::output(backlight.into())?;
        backlight_pin.set_high()?;
        log::info!("Backlight enabled (GPIO high)");
        
        let mut display = Self {
            lcd_bus: LcdBus::new(d0, d1, d2, d3, d4, d5, d6, d7, wr, dc, cs, rst)?,
            backlight_pin: None, // Will be set after struct creation
            lcd_power_pin: None, // Will be set after struct creation
            rd_pin: None, // Will be set after struct creation
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            last_activity: Instant::now(),
        };

        // Set up LCD power pin (GPIO 15) - CRITICAL: Must keep alive!
        let mut lcd_power_pin = PinDriver::output(lcd_power.into())?;
        lcd_power_pin.set_high()?;
        log::info!("LCD power enabled and will be kept alive");
        
        // Set up RD pin (GPIO 9) - Must be kept high
        let mut rd_pin = PinDriver::output(rd.into())?;
        rd_pin.set_high()?;
        log::info!("RD pin set high and will be kept alive");
        
        // Store all pins to keep them alive for the lifetime of the display
        display.backlight_pin = Some(backlight_pin);
        display.lcd_power_pin = Some(lcd_power_pin);
        display.rd_pin = Some(rd_pin);
        
        display.init()?;
        Ok(display)
    }

    fn init(&mut self) -> Result<()> {
        log::info!("Initializing ST7789 display (LilyGO T-Display-S3)...");

        // Send several NOPs to clear any garbage
        for _ in 0..10 {
            self.lcd_bus.write_command(CMD_NOP)?;
            FreeRtos::delay_ms(1);
        }
        
        // Hardware reset
        self.lcd_bus.reset()?;
        
        // More NOPs after reset
        for _ in 0..5 {
            self.lcd_bus.write_command(CMD_NOP)?;
        }

        // Software reset
        self.lcd_bus.write_command(CMD_SWRESET)?;
        FreeRtos::delay_ms(150);  // Increased delay

        // Sleep out
        self.lcd_bus.write_command(CMD_SLPOUT)?;
        FreeRtos::delay_ms(120);

        // Memory access control - matching Arduino implementation
        self.lcd_bus.write_command(CMD_MADCTL)?;
        self.lcd_bus.write_data(0x60)?; // Same as Arduino - landscape mode

        // Pixel format - 16-bit RGB565
        self.lcd_bus.write_command(CMD_COLMOD)?;
        self.lcd_bus.write_data(0x55)?; // 16-bit color - matching Arduino

        // Porch control
        self.lcd_bus.write_command(CMD_PORCTRL)?;
        self.lcd_bus.write_data_bytes(&[0x0C, 0x0C, 0x00, 0x33, 0x33])?;

        // Gate control
        self.lcd_bus.write_command(CMD_GCTRL)?;
        self.lcd_bus.write_data(0x35)?;

        // VCOM setting
        self.lcd_bus.write_command(CMD_VCOMS)?;
        self.lcd_bus.write_data(0x19)?;

        // LCM control
        self.lcd_bus.write_command(CMD_LCMCTRL)?;
        self.lcd_bus.write_data(0x2C)?;

        // VDV and VRH enable
        self.lcd_bus.write_command(CMD_VDVVRHEN)?;
        self.lcd_bus.write_data(0x01)?;

        // VRH set
        self.lcd_bus.write_command(CMD_VRHS)?;
        self.lcd_bus.write_data(0x12)?;

        // VDV set
        self.lcd_bus.write_command(CMD_VDVS)?;
        self.lcd_bus.write_data(0x20)?;

        // Frame rate control - 60Hz
        self.lcd_bus.write_command(CMD_FRCTRL2)?;
        self.lcd_bus.write_data(0x0F)?;

        // Power control
        self.lcd_bus.write_command(CMD_PWRCTRL1)?;
        self.lcd_bus.write_data_bytes(&[0xA4, 0xA1])?;

        // Inversion ON (required for this panel)
        self.lcd_bus.write_command(CMD_INVON)?;

        // Display ON
        self.lcd_bus.write_command(CMD_DISPON)?;
        FreeRtos::delay_ms(20);

        // Clear full controller memory to remove factory patterns
        log::info!("Clearing controller memory...");
        self.clear_controller_memory()?;
        
        // Additional delay after clear
        FreeRtos::delay_ms(50);
        
        // Clear visible area to black instead of white test
        log::info!("Clearing visible area to black...");
        self.clear(colors::BLACK)?;

        log::info!("Display initialized successfully");
        Ok(())
    }


    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<()> {
        // Apply display boundaries offsets
        let x0_offset = x0 + DISPLAY_X_START;
        let x1_offset = x1 + DISPLAY_X_START;
        let y0_offset = y0 + DISPLAY_Y_START;
        let y1_offset = y1 + DISPLAY_Y_START;
        
        // Column address set
        self.lcd_bus.write_command(CMD_CASET)?;
        self.lcd_bus.write_data_16(x0_offset)?;
        self.lcd_bus.write_data_16(x1_offset)?;

        // Row address set
        self.lcd_bus.write_command(CMD_RASET)?;
        self.lcd_bus.write_data_16(y0_offset)?;
        self.lcd_bus.write_data_16(y1_offset)?;

        // Note: RAMWR command removed from here - must be sent before each pixel write
        Ok(())
    }

    pub fn clear(&mut self, color: u16) -> Result<()> {
        self.set_window(0, 0, self.width - 1, self.height - 1)?;
        
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        
        self.lcd_bus.begin_write()?;
        
        let total_pixels = self.width as u32 * self.height as u32;
        let color_bytes = [(color >> 8) as u8, (color & 0xFF) as u8];
        for _ in 0..total_pixels {
            self.lcd_bus.write_raw(&color_bytes)?;
        }
        
        self.lcd_bus.end_write()?;
        Ok(())
    }
    
    /// Clear the entire controller memory (480x320) - needed once after reset
    fn clear_controller_memory(&mut self) -> Result<()> {
        // Set window to full controller RAM size (480x320)
        self.lcd_bus.write_command(CMD_CASET)?;
        self.lcd_bus.write_data_16(0)?;
        self.lcd_bus.write_data_16(479)?;  // 480 - 1
        
        self.lcd_bus.write_command(CMD_RASET)?;
        self.lcd_bus.write_data_16(0)?;
        self.lcd_bus.write_data_16(319)?;  // 320 - 1
        
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        
        self.lcd_bus.begin_write()?;
        
        // Clear all 480x320 = 153,600 pixels
        let total_pixels = 480u32 * 320u32;
        let black = [0x00u8, 0x00u8]; // Black color
        
        // Write in chunks for better performance
        for _ in 0..total_pixels {
            self.lcd_bus.write_raw(&black)?;
        }
        
        self.lcd_bus.end_write()?;
        Ok(())
    }

    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }

        self.set_window(x, y, x, y)?;
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        self.lcd_bus.write_data_16(color)?;
        Ok(())
    }

    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }

        let x1 = (x + w - 1).min(self.width - 1);
        let y1 = (y + h - 1).min(self.height - 1);

        self.set_window(x, y, x1, y1)?;
        
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        
        self.lcd_bus.begin_write()?;
        
        let total_pixels = (x1 - x + 1) as u32 * (y1 - y + 1) as u32;
        let color_bytes = [(color >> 8) as u8, (color & 0xFF) as u8];
        for _ in 0..total_pixels {
            self.lcd_bus.write_raw(&color_bytes)?;
        }
        
        self.lcd_bus.end_write()?;
        Ok(())
    }

    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> Result<()> {
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

    // set_brightness removed - PWM not implemented yet
    
    pub fn reset_activity_timer(&mut self) {
        self.last_activity = Instant::now();
    }
    
    pub fn update_auto_dim(&mut self) -> Result<()> {
        // Auto-dim disabled - no PWM implementation yet
        // Just ensure backlight stays on
        if let Some(ref mut pin) = self.backlight_pin {
            pin.set_high()?;
        }
        Ok(())
    }
    
    // mark_dirty and clear_dirty_rects removed - dirty rect tracking not implemented

    pub fn flush(&mut self) -> Result<()> {
        // For direct GPIO control, no flush needed
        // But ensure display stays on
        self.ensure_display_on()?;
        Ok(())
    }
    
    pub fn ensure_display_on(&mut self) -> Result<()> {
        // Send SLPOUT and DISPON to ensure display doesn't sleep
        self.lcd_bus.write_command(CMD_SLPOUT)?;
        FreeRtos::delay_ms(5);
        self.lcd_bus.write_command(CMD_DISPON)?;
        Ok(())
    }

    pub fn draw_char(&mut self, x: u16, y: u16, c: char, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let char_data = get_char_data(c);
        
        for col in 0..FONT_WIDTH {
            for row in 0..FONT_HEIGHT {
                if (char_data[col as usize] >> row) & 1 == 1 {
                    // Draw pixel(s) for the character
                    for sx in 0..scale {
                        for sy in 0..scale {
                            self.draw_pixel(
                                x + (col * scale + sx) as u16,
                                y + (row * scale + sy) as u16,
                                color
                            )?;
                        }
                    }
                } else if let Some(bg) = bg_color {
                    // Draw background pixel(s)
                    for sx in 0..scale {
                        for sy in 0..scale {
                            self.draw_pixel(
                                x + (col * scale + sx) as u16,
                                y + (row * scale + sy) as u16,
                                bg
                            )?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let mut cursor_x = x;
        let char_width = (FONT_WIDTH * scale + 1) as u16; // +1 for spacing
        
        for c in text.chars() {
            if cursor_x + char_width as u16 > self.width {
                break; // Don't draw beyond screen
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
        for y in 0..=r {
            for x in 0..=r {
                if x * x + y * y <= r * r {
                    self.draw_pixel(cx + x, cy + y, color)?;
                    self.draw_pixel(cx - x, cy + y, color)?;
                    self.draw_pixel(cx + x, cy - y, color)?;
                    self.draw_pixel(cx - x, cy - y, color)?;
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

}