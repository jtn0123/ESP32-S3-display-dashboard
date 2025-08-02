pub mod colors;
pub mod font5x7;
pub mod lcd_bus;
pub mod dirty_rect_manager; // Enhanced dirty rectangle management

// Color type not used - colors are defined as u16 constants

use anyhow::Result;
use self::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use self::lcd_bus::LcdBus;
// use self::perf_metrics::DisplayMetrics;
use self::dirty_rect_manager::DirtyRectManager;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, Output};
use esp_idf_hal::delay::FreeRtos;
use std::time::Instant;


// Display boundaries - Discovered values from Arduino testing
const DISPLAY_X_START: u16 = 10;   // Left boundary offset
const DISPLAY_Y_START: u16 = 36;   // Top boundary offset
const DISPLAY_WIDTH: u16 = 300;    // Actual visible width
const DISPLAY_HEIGHT: u16 = 168;   // Actual visible height

// Controller dimensions - ST7789 expects these
const CONTROLLER_WIDTH: u16 = 480;
const CONTROLLER_HEIGHT: u16 = 320;

// Controller dimensions removed - not used

// Dirty rectangle tracking for optimized rendering
#[derive(Debug, Clone, Copy)]
pub struct DirtyRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl DirtyRect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn merge(&mut self, other: &DirtyRect) {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        
        self.x = x1;
        self.y = y1;
        self.width = x2 - x1;
        self.height = y2 - y1;
    }
    
}

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

#[cfg(not(feature = "esp_lcd_driver"))]
pub struct DisplayManager {
    lcd_bus: LcdBus,
    backlight_pin: Option<PinDriver<'static, AnyIOPin, Output>>, // Keep backlight alive
    lcd_power_pin: Option<PinDriver<'static, AnyIOPin, Output>>, // Keep LCD power alive
    _rd_pin: Option<PinDriver<'static, AnyIOPin, Output>>, // Keep RD pin high
    width: u16,
    height: u16,
    last_activity: Instant,
    dirty_rect_manager: DirtyRectManager,
    // metrics: DisplayMetrics, // Performance tracking
}

#[cfg(not(feature = "esp_lcd_driver"))]
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
        
        // Set up LCD power pin (GPIO 15) FIRST - CRITICAL: Must keep alive!
        let mut lcd_power_pin = PinDriver::output(lcd_power.into())?;
        lcd_power_pin.set_high()?;
        log::info!("LCD power enabled and will be kept alive");
        
        // Set up RD pin (GPIO 9) - Must be kept high
        let mut rd_pin = PinDriver::output(rd.into())?;
        rd_pin.set_high()?;
        log::info!("RD pin set high and will be kept alive");
        
        // Set up backlight
        let mut backlight_pin = PinDriver::output(backlight.into())?;
        log::info!("About to set backlight HIGH during initialization");
        backlight_pin.set_high()?;
        log::info!("Backlight enabled (GPIO high) - display should be visible now");
        
        
        let mut display = Self {
            lcd_bus: LcdBus::new(d0, d1, d2, d3, d4, d5, d6, d7, wr, dc, cs, rst)?,
            backlight_pin: Some(backlight_pin),
            lcd_power_pin: Some(lcd_power_pin),
            _rd_pin: Some(rd_pin),
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            last_activity: Instant::now(),
            dirty_rect_manager: DirtyRectManager::new(),
            // metrics: DisplayMetrics::new(),
        };
        
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

        // Initialize the full controller memory
        // The ST7789 controller expects 480x320 pixels to be initialized
        log::info!("Performing comprehensive memory initialization...");
        self.comprehensive_memory_init()?;
        
        // Clear visible area to black
        log::info!("Clearing visible area to black...");
        self.clear(colors::BLACK)?;
        

        log::info!("Display initialized successfully");
        Ok(())
    }

    fn comprehensive_memory_init(&mut self) -> Result<()> {
        // Initialize the full controller memory (480x320)
        // This fixes display cutoff issues on T-Display-S3
        self.lcd_bus.write_command(CMD_CASET)?;
        self.lcd_bus.write_data_16(0)?;
        self.lcd_bus.write_data_16(CONTROLLER_WIDTH - 1)?;
        
        self.lcd_bus.write_command(CMD_RASET)?;
        self.lcd_bus.write_data_16(0)?;
        self.lcd_bus.write_data_16(CONTROLLER_HEIGHT - 1)?;
        
        self.lcd_bus.write_command(CMD_RAMWR)?;
        
        // Write black to entire controller memory - OPTIMIZED
        let total_pixels = CONTROLLER_WIDTH as u32 * CONTROLLER_HEIGHT as u32;
        let pixels_per_chunk = 8192u32; // Increased chunk size for faster writes
        
        // Pre-allocate buffer once
        let chunk_data = vec![0x00u8; (pixels_per_chunk * 2) as usize];
        let chunks = total_pixels / pixels_per_chunk;
        
        for chunk in 0..chunks {
            self.lcd_bus.write_data_bytes(&chunk_data)?;
            
            // Feed watchdog less frequently
            if chunk % 5 == 0 {
                unsafe { esp_idf_sys::esp_task_wdt_reset(); }
            }
        }
        
        // Handle remaining pixels
        let remaining = (total_pixels % pixels_per_chunk) as usize;
        if remaining > 0 {
            self.lcd_bus.write_data_bytes(&chunk_data[..remaining * 2])?;
        }
        
        log::info!("Memory initialization complete");
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
        // Direct clear - original implementation
        self.set_window(0, 0, self.width - 1, self.height - 1)?;
        
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        
        // Write pixels using optimized bulk write
        let total_pixels = self.width as u32 * self.height as u32;
        self.lcd_bus.write_pixels(color, total_pixels)?;
        
        // Mark entire screen as dirty
        self.dirty_rect_manager.add_rect(0, 0, self.width, self.height);
        
        Ok(())
    }
    
    // /// Clear the entire controller memory (480x320) - needed once after reset
    // fn clear_controller_memory(&mut self) -> Result<()> {
    //     // Set window to full controller RAM size (480x320)
    //     self.lcd_bus.write_command(CMD_CASET)?;
    //     self.lcd_bus.write_data_16(0)?;
    //     self.lcd_bus.write_data_16(479)?;  // 480 - 1
    //     
    //     self.lcd_bus.write_command(CMD_RASET)?;
    //     self.lcd_bus.write_data_16(0)?;
    //     self.lcd_bus.write_data_16(319)?;  // 320 - 1
    //     
    //     // CRITICAL: Must send RAMWR before pixel data
    //     self.lcd_bus.write_command(CMD_RAMWR)?;
    //     
    //     // Clear all 480x320 = 153,600 pixels
    //     let total_pixels = 480u32 * 320u32;
    //     
    //     // Write pixels directly
    //     for _ in 0..total_pixels {
    //         self.lcd_bus.write_data(0)?;  // Black high byte
    //         self.lcd_bus.write_data(0)?;  // Black low byte
    //     }
    //     Ok(())
    // }

    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }

        // Direct pixel write - original implementation
        self.set_window(x, y, x, y)?;
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        self.lcd_bus.write_data_16(color)?;
        
        // Track dirty region
        self.dirty_rect_manager.add_rect(x, y, 1, 1);
        
        Ok(())
    }

    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }

        let x1 = (x + w - 1).min(self.width - 1);
        let y1 = (y + h - 1).min(self.height - 1);
        let actual_width = x1 - x + 1;
        let actual_height = y1 - y + 1;

        // Direct fill - original implementation
        self.set_window(x, y, x1, y1)?;
        
        // CRITICAL: Must send RAMWR before pixel data
        self.lcd_bus.write_command(CMD_RAMWR)?;
        
        // Write pixels using optimized bulk write
        let total_pixels = actual_width as u32 * actual_height as u32;
        self.lcd_bus.write_pixels(color, total_pixels)?;
        
        // Track dirty region
        self.dirty_rect_manager.add_rect(x, y, actual_width, actual_height);
        
        Ok(())
    }

    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> Result<()> {
        // Calculate bounding box for the line
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        
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
        
        // Track the entire line's bounding box as dirty
        self.dirty_rect_manager.add_rect(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1);

        Ok(())
    }

    pub fn draw_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        self.draw_line(x, y, x + w - 1, y, color)?;
        self.draw_line(x, y + h - 1, x + w - 1, y + h - 1, color)?;
        self.draw_line(x, y, x, y + h - 1, color)?;
        self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, color)?;
        
        // The entire rectangle area is dirty
        self.dirty_rect_manager.add_rect(x, y, w, h);
        
        Ok(())
    }

    // set_brightness removed - PWM not implemented yet
    
    pub fn reset_activity_timer(&mut self) {
        self.last_activity = Instant::now();
    }
    
    pub fn update_auto_dim(&mut self, should_display_on: bool) -> Result<()> {
        // Control backlight based on power manager state
        if let Some(ref mut pin) = self.backlight_pin {
            if should_display_on {
                pin.set_high()?;
            } else {
                pin.set_low()?;  // Turn off backlight in sleep mode
            }
        }
        
        // Keep LCD power on always (turning it off requires re-initialization)
        if let Some(ref mut pin) = self.lcd_power_pin {
            pin.set_high()?;
        }
        
        Ok(())
    }
    
    // mark_dirty and clear_dirty_rects removed - dirty rect tracking not implemented

    
    pub fn ensure_display_on(&mut self) -> Result<()> {
        // Send SLPOUT and DISPON to ensure display doesn't sleep
        self.lcd_bus.write_command(CMD_SLPOUT)?;
        FreeRtos::delay_ms(5);
        self.lcd_bus.write_command(CMD_DISPON)?;
        log::debug!("Display wakeup commands sent");
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
        
        // Now draw the character pixels in batches
        // Group pixels by row for more efficient drawing
        for row in 0..FONT_HEIGHT {
            let mut col_start = None;
            let mut col_count = 0;
            
            for col in 0..FONT_WIDTH {
                let pixel_on = (char_data[col as usize] >> row) & 1 == 1;
                
                if pixel_on {
                    if col_start.is_none() {
                        col_start = Some(col);
                        col_count = 1;
                    } else {
                        col_count += 1;
                    }
                } else if let Some(start_col) = col_start {
                    // Draw the accumulated horizontal line
                    let rect_x = x + (start_col * scale) as u16;
                    let rect_y = y + (row * scale) as u16;
                    self.fill_rect(rect_x, rect_y, (col_count * scale) as u16, scale as u16, color)?;
                    
                    col_start = None;
                    col_count = 0;
                }
            }
            
            // Draw any remaining pixels in the row
            if let Some(start_col) = col_start {
                let rect_x = x + (start_col * scale) as u16;
                let rect_y = y + (row * scale) as u16;
                self.fill_rect(rect_x, rect_y, (col_count * scale) as u16, scale as u16, color)?;
            }
        }
        
        // Mark the entire character area as dirty
        let char_width = FONT_WIDTH * scale;
        let char_height = FONT_HEIGHT * scale;
        self.dirty_rect_manager.add_rect(x, y, char_width as u16, char_height as u16);
        
        Ok(())
    }

    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let mut cursor_x = x;
        let char_width = (FONT_WIDTH * scale + 1) as u16; // +1 for spacing
        let start_x = x;
        
        for c in text.chars() {
            if cursor_x + char_width > self.width {
                break; // Don't draw beyond screen
            }
            
            self.draw_char(cursor_x, y, c, color, bg_color, scale)?;
            cursor_x += char_width;
        }
        
        // Mark the entire text area as dirty
        if cursor_x > start_x {
            let text_width = cursor_x - start_x;
            let text_height = (FONT_HEIGHT * scale) as u16;
            self.dirty_rect_manager.add_rect(start_x, y, text_width, text_height);
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
        // Calculate bounding box
        let min_x = (cx as i32 - r as i32).max(0) as u16;
        let max_x = (cx as i32 + r as i32).min(self.width as i32 - 1) as u16;
        let min_y = (cy as i32 - r as i32).max(0) as u16;
        let max_y = (cy as i32 + r as i32).min(self.height as i32 - 1) as u16;
        
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
        
        // Mark the circle's bounding box as dirty
        self.dirty_rect_manager.add_rect(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1);

        Ok(())
    }

    pub fn fill_circle(&mut self, cx: u16, cy: u16, r: u16, color: u16) -> Result<()> {
        // Use horizontal line drawing for better performance
        for dy in 0..=r as i32 {
            // Calculate the horizontal line width at this y offset
            let dx = ((r as i32 * r as i32 - dy * dy) as f32).sqrt() as i32;
            
            if dx > 0 {
                // Draw horizontal lines instead of individual pixels
                let x_start = (cx as i32 - dx).max(0) as u16;
                let x_end = (cx as i32 + dx).min(self.width as i32 - 1) as u16;
                let width = x_end - x_start + 1;
                
                // Draw line above center
                if cy as i32 - dy >= 0 {
                    self.fill_rect(x_start, (cy as i32 - dy) as u16, width, 1, color)?;
                }
                
                // Draw line below center (avoid duplicate at dy=0)
                if dy > 0 && cy as i32 + dy < self.height as i32 {
                    self.fill_rect(x_start, (cy as i32 + dy) as u16, width, 1, color)?;
                }
            }
        }
        
        // Mark the circle's bounding box as dirty
        let min_x = (cx as i32 - r as i32).max(0) as u16;
        let max_x = (cx as i32 + r as i32).min(self.width as i32 - 1) as u16;
        let min_y = (cy as i32 - r as i32).max(0) as u16;
        let max_y = (cy as i32 + r as i32).min(self.height as i32 - 1) as u16;
        self.dirty_rect_manager.add_rect(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1);
        
        Ok(())
    }

    pub fn draw_progress_bar(&mut self, x: u16, y: u16, w: u16, h: u16, progress: u8, fg_color: u16, bg_color: u16, border_color: u16) -> Result<()> {
        // Cache last progress bar state to avoid redundant draws
        static mut LAST_PROGRESS_STATE: (u16, u16, u16, u16, u8) = (0, 0, 0, 0, 255);
        
        unsafe {
            if LAST_PROGRESS_STATE == (x, y, w, h, progress) {
                return Ok(()); // Skip if nothing changed
            }
            
            // Only redraw the parts that changed
            if LAST_PROGRESS_STATE.0 != x || LAST_PROGRESS_STATE.1 != y || 
               LAST_PROGRESS_STATE.2 != w || LAST_PROGRESS_STATE.3 != h {
                // Position or size changed, redraw everything
                self.draw_rect(x, y, w, h, border_color)?;
                self.fill_rect(x + 1, y + 1, w - 2, h - 2, bg_color)?;
            }
            
            // Calculate progress widths
            let old_progress_width = ((w - 2) as u32 * LAST_PROGRESS_STATE.4 as u32 / 100) as u16;
            let new_progress_width = ((w - 2) as u32 * progress as u32 / 100) as u16;
            
            if new_progress_width != old_progress_width {
                if new_progress_width > old_progress_width {
                    // Progress increased - just draw the new part
                    self.fill_rect(x + 1 + old_progress_width, y + 1, 
                                  new_progress_width - old_progress_width, h - 2, fg_color)?;
                } else {
                    // Progress decreased - clear the removed part
                    self.fill_rect(x + 1 + new_progress_width, y + 1, 
                                  old_progress_width - new_progress_width, h - 2, bg_color)?;
                }
            }
            
            LAST_PROGRESS_STATE = (x, y, w, h, progress);
        }
        
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        // Non-frame buffer path - just track dirty regions
        if !self.dirty_rect_manager.is_empty() {
            // Get statistics for debugging
            let (rect_count, merge_count, _update_count) = self.dirty_rect_manager.get_stats();
            if rect_count > 5 {
                log::debug!("Dirty rectangles: {} (merges: {})", rect_count, merge_count);
            }
            
            // For now, we'll still do a full refresh but track what changed
            // In the future, we can optimize to only update the dirty regions
            self.dirty_rect_manager.clear();
        }
        Ok(())
    }
    
    
    /// Draw a battery icon with charge level and optional charging indicator
    pub fn draw_battery_icon(&mut self, x: u16, y: u16, percentage: u8, is_charging: bool, scale: u8) -> Result<()> {
        let width = 24 * scale as u16;
        let height = 12 * scale as u16;
        let terminal_width = 2 * scale as u16;
        let terminal_height = 6 * scale as u16;
        
        // Determine battery color based on percentage
        let battery_color = if percentage > 50 {
            colors::PRIMARY_GREEN
        } else if percentage > 20 {
            colors::YELLOW
        } else {
            colors::PRIMARY_RED
        };
        
        // Draw battery outline
        self.draw_rect(x, y, width, height, colors::WHITE)?;
        
        // Draw battery terminal (positive end)
        self.fill_rect(x + width, y + (height - terminal_height) / 2, terminal_width, terminal_height, colors::WHITE)?;
        
        // Draw battery fill based on percentage
        let fill_width = ((width - 4) as u32 * percentage as u32 / 100) as u16;
        if fill_width > 0 {
            self.fill_rect(x + 2, y + 2, fill_width, height - 4, battery_color)?;
        }
        
        // Draw charging indicator if charging
        if is_charging {
            // Draw a simple lightning bolt in the center
            let cx = x + width / 2;
            let cy = y + height / 2;
            
            // Lightning bolt shape (simplified)
            self.draw_line(cx - 2, cy - 3, cx + 1, cy, colors::WHITE)?;
            self.draw_line(cx + 1, cy, cx - 1, cy + 3, colors::WHITE)?;
            self.draw_pixel(cx, cy - 1, colors::WHITE)?;
            self.draw_pixel(cx - 1, cy + 1, colors::WHITE)?;
        }
        
        Ok(())
    }

}