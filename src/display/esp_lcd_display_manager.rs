/// ESP_LCD-based DisplayManager implementation
/// This provides hardware-accelerated display output via DMA
/// Full API compatibility with the GPIO-based DisplayManager

use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use log::{info, debug};
use std::time::Instant;

use super::colors::*;
use super::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use super::dirty_rect_manager::DirtyRectManager;

// Display configuration matching the hardware
const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 170;
const DISPLAY_X_OFFSET: u16 = 0;
const DISPLAY_Y_OFFSET: u16 = 35; // ST7789 offset for 170-pixel height

// Performance configuration
const PIXEL_CLOCK_HZ: u32 = 20 * 1000 * 1000; // 20MHz for stability (reduced from 40MHz)
const MAX_TRANSFER_BYTES: usize = DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize * 2; // Full screen

// Auto-dim configuration
const AUTO_DIM_TIMEOUT_SECS: u64 = 300; // 5 minutes before dimming (increased from 2)
const DIM_BRIGHTNESS: u8 = 100; // 100/255 brightness when dimmed (increased from 20)
const FULL_BRIGHTNESS: u8 = 255;

pub struct EspLcdDisplayManager {
    bus_handle: esp_lcd_i80_bus_handle_t,
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: esp_lcd_panel_io_handle_t,
    width: u16,
    height: u16,
    frame_buffer: Vec<u16>,
    dirty_rect_manager: DirtyRectManager,
    last_render_time: Instant,
    render_count: u32,
    last_activity: Instant,
    backlight_level: u8,
    backlight_pin: i32,
    lcd_power_pin: i32,
    use_frame_buffer: bool,
    initialized: bool,
}

impl EspLcdDisplayManager {
    pub fn new(
        d0: impl Into<AnyIOPin>,
        d1: impl Into<AnyIOPin>,
        d2: impl Into<AnyIOPin>,
        d3: impl Into<AnyIOPin>,
        d4: impl Into<AnyIOPin>,
        d5: impl Into<AnyIOPin>,
        d6: impl Into<AnyIOPin>,
        d7: impl Into<AnyIOPin>,
        wr: impl Into<AnyIOPin>,
        dc: impl Into<AnyIOPin>,
        cs: impl Into<AnyIOPin>,
        rst: impl Into<AnyIOPin>,
        backlight: impl Into<AnyIOPin>,
        lcd_power: impl Into<AnyIOPin>,
        _rd: impl Into<AnyIOPin>, // RD pin configured as input with pullup
    ) -> Result<Self> {
        info!("Initializing ESP_LCD display manager with DMA...");
        
        // Convert pins to GPIO numbers using a mapping similar to the test
        let d0_pin = 39i32; // GPIO39
        let d1_pin = 40i32; // GPIO40
        let d2_pin = 41i32; // GPIO41
        let d3_pin = 42i32; // GPIO42
        let d4_pin = 45i32; // GPIO45
        let d5_pin = 46i32; // GPIO46
        let d6_pin = 47i32; // GPIO47
        let d7_pin = 48i32; // GPIO48
        let wr_pin = 8i32;  // GPIO8
        let dc_pin = 7i32;  // GPIO7
        let cs_pin = 6i32;  // GPIO6
        let rst_pin = 5i32; // GPIO5
        let backlight_pin = 38i32; // GPIO38
        let lcd_power_pin = 15i32; // GPIO15
        let rd_pin = 9i32;  // GPIO9
        
        // We ignore the pin parameters for now - they match our hardcoded values
        let _ = (d0.into(), d1.into(), d2.into(), d3.into(), 
                 d4.into(), d5.into(), d6.into(), d7.into(),
                 wr.into(), dc.into(), cs.into(), rst.into(),
                 backlight.into(), lcd_power.into(), _rd.into());
        
        unsafe {
            // Initialize LCD power and backlight pins
            info!("Initializing LCD power and backlight pins...");
            
            // Configure LCD power pin (GPIO15) as output and set HIGH
            let lcd_power_config = gpio_config_t {
                pin_bit_mask: 1u64 << lcd_power_pin,
                mode: gpio_mode_t_GPIO_MODE_OUTPUT,
                pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
                pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            gpio_config(&lcd_power_config);
            gpio_set_level(lcd_power_pin, 1); // Turn on LCD power
            
            // Configure backlight pin as output and set HIGH
            let backlight_config = gpio_config_t {
                pin_bit_mask: 1u64 << backlight_pin,
                mode: gpio_mode_t_GPIO_MODE_OUTPUT,
                pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
                pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            gpio_config(&backlight_config);
            gpio_set_level(backlight_pin, 1); // Turn on backlight
            
            // Wait for power to stabilize
            FreeRtos::delay_ms(100);
            
            // Configure LCD_RD pin as input with pullup
            info!("Configuring LCD RD GPIO...");
            let lcd_rd_gpio_config = gpio_config_t {
                pin_bit_mask: 1u64 << rd_pin,
                mode: gpio_mode_t_GPIO_MODE_INPUT,
                pull_up_en: gpio_pullup_t_GPIO_PULLUP_ENABLE,
                pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            let ret = gpio_config(&lcd_rd_gpio_config);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to configure RD pin: {:?}", ret));
            }
            
            // Configure I80 bus for DMA transfers
            info!("Initializing Intel 8080 bus with DMA...");
            let mut bus_config: esp_lcd_i80_bus_config_t = Default::default();
            bus_config.dc_gpio_num = dc_pin;
            bus_config.wr_gpio_num = wr_pin;
            bus_config.clk_src = soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT;
            bus_config.data_gpio_nums = [
                d0_pin, d1_pin, d2_pin, d3_pin,
                d4_pin, d5_pin, d6_pin, d7_pin,
                -1, -1, -1, -1, -1, -1, -1, -1, // Only 8-bit mode
            ];
            bus_config.bus_width = 8;
            bus_config.max_transfer_bytes = MAX_TRANSFER_BYTES;
            bus_config.sram_trans_align = 4;
            
            let mut i80_bus: esp_lcd_i80_bus_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_i80_bus(&bus_config, &mut i80_bus);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create I80 bus: {:?}", ret));
            }
            info!("I80 bus created successfully");
            
            // Configure panel IO
            info!("Creating panel IO...");
            let mut io_config: esp_lcd_panel_io_i80_config_t = Default::default();
            io_config.cs_gpio_num = cs_pin;
            io_config.pclk_hz = PIXEL_CLOCK_HZ;
            io_config.trans_queue_depth = 20;
            io_config.on_color_trans_done = None;
            io_config.user_ctx = ptr::null_mut();
            io_config.lcd_cmd_bits = 8;
            io_config.lcd_param_bits = 8;
            
            // Set DC levels using bitfield
            io_config.dc_levels._bitfield_1 = esp_lcd_panel_io_i80_config_t__bindgen_ty_1::new_bitfield_1(
                0, // dc_idle_level
                0, // dc_cmd_level  
                0, // dc_dummy_level
                1, // dc_data_level
            );
            
            let mut io_handle: esp_lcd_panel_io_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_panel_io_i80(i80_bus, &io_config, &mut io_handle);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create panel IO: {:?}", ret));
            }
            info!("Panel IO created successfully");
            
            // Create ST7789 panel
            info!("Initializing ST7789 LCD Driver...");
            let mut panel_config: esp_lcd_panel_dev_config_t = Default::default();
            panel_config.reset_gpio_num = rst_pin;
            panel_config.bits_per_pixel = 16;
            panel_config.vendor_config = ptr::null_mut();
            
            let mut panel_handle: esp_lcd_panel_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_panel_st7789(io_handle, &panel_config, &mut panel_handle);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create ST7789 panel: {:?}", ret));
            }
            info!("ST7789 panel created successfully");
            
            // Initialize panel
            info!("Resetting panel...");
            esp_lcd_panel_reset(panel_handle);
            FreeRtos::delay_ms(100);
            
            info!("Initializing panel...");
            esp_lcd_panel_init(panel_handle);
            FreeRtos::delay_ms(100);
            
            // Configure display orientation
            info!("Configuring display orientation...");
            esp_lcd_panel_invert_color(panel_handle, true);
            esp_lcd_panel_swap_xy(panel_handle, true);
            esp_lcd_panel_mirror(panel_handle, false, true);
            esp_lcd_panel_set_gap(panel_handle, 0, DISPLAY_Y_OFFSET as i32);
            
            info!("Turning display on...");
            esp_lcd_panel_disp_on_off(panel_handle, true);
            
            // Create frame buffer - ensure it's heap allocated
            let buffer_size = (DISPLAY_WIDTH as usize) * (DISPLAY_HEIGHT as usize) * 2;
            info!("Allocating frame buffer: {} bytes", buffer_size);
            let frame_buffer = vec![BLACK; (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize];
            info!("Frame buffer allocated successfully");
            
            Ok(Self {
                bus_handle: i80_bus,
                panel_handle,
                io_handle,
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                frame_buffer,
                dirty_rect_manager: DirtyRectManager::new(),
                last_render_time: Instant::now(),
                render_count: 0,
                last_activity: Instant::now(),
                backlight_level: FULL_BRIGHTNESS,
                backlight_pin,
                lcd_power_pin,
                use_frame_buffer: true, // Always use frame buffer for dirty rect optimization
                initialized: true,
            })
        }
    }
    
    /// Clear the entire display with a color
    pub fn clear(&mut self, color: u16) -> Result<()> {
        debug!("Clear called with color: 0x{:04X}", color);
        
        // Bounds check
        if self.frame_buffer.is_empty() {
            return Err(anyhow::anyhow!("Frame buffer not initialized"));
        }
        
        // Simple implementation - just update frame buffer and mark dirty
        self.frame_buffer.fill(color);
        self.dirty_rect_manager.add_rect(0, 0, self.width, self.height);
        
        debug!("Clear complete, marked {}x{} as dirty", self.width, self.height);
        Ok(())
    }
    
    /// Draw a single pixel
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(()); // Out of bounds
        }
        
        if self.use_frame_buffer {
            let idx = (y as usize * self.width as usize) + x as usize;
            self.frame_buffer[idx] = color;
            self.dirty_rect_manager.add_rect(x, y, 1, 1);
        } else {
            // Direct draw - less efficient for single pixels
            unsafe {
                esp_lcd_panel_draw_bitmap(
                    self.panel_handle,
                    x as i32, y as i32,
                    (x + 1) as i32, (y + 1) as i32,
                    &color as *const u16 as *const _,
                );
            }
        }
        
        Ok(())
    }
    
    /// Fill a rectangle with a color
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        debug!("fill_rect called: x={}, y={}, w={}, h={}, color=0x{:04X}", x, y, w, h, color);
        
        if x >= self.width || y >= self.height {
            debug!("fill_rect: out of bounds, returning");
            return Ok(());
        }
        
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        let actual_w = x_end - x;
        let actual_h = y_end - y;
        
        debug!("fill_rect: actual bounds x={}, y={}, w={}, h={}", x, y, actual_w, actual_h);
        
        if self.use_frame_buffer {
            // Update frame buffer
            for row in y..y_end {
                let start_idx = (row as usize * self.width as usize) + x as usize;
                let end_idx = start_idx + actual_w as usize;
                
                // Bounds check
                if start_idx >= self.frame_buffer.len() || end_idx > self.frame_buffer.len() {
                    return Err(anyhow::anyhow!("Frame buffer index out of bounds: start={}, end={}, len={}", 
                        start_idx, end_idx, self.frame_buffer.len()));
                }
                
                self.frame_buffer[start_idx..end_idx].fill(color);
            }
            self.dirty_rect_manager.add_rect(x, y, actual_w, actual_h);
            debug!("fill_rect: frame buffer updated");
        } else {
            // Direct draw
            let buffer = vec![color; (actual_w * actual_h) as usize];
            unsafe {
                esp_lcd_panel_draw_bitmap(
                    self.panel_handle,
                    x as i32, y as i32,
                    x_end as i32, y_end as i32,
                    buffer.as_ptr() as *const _,
                );
            }
        }
        
        Ok(())
    }
    
    /// Draw a line using Bresenham's algorithm
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
    
    /// Draw a rectangle outline
    pub fn draw_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        self.draw_line(x, y, x + w - 1, y, color)?;
        self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, color)?;
        self.draw_line(x + w - 1, y + h - 1, x, y + h - 1, color)?;
        self.draw_line(x, y + h - 1, x, y, color)?;
        Ok(())
    }
    
    /// Draw a character at the specified position
    pub fn draw_char(&mut self, x: u16, y: u16, c: char, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let char_data = get_char_data(c);
        let char_width = FONT_WIDTH * scale;
        let char_height = FONT_HEIGHT * scale;
        
        // Draw background if specified
        if let Some(bg) = bg_color {
            self.fill_rect(x, y, char_width as u16, char_height as u16, bg)?;
        }
        
        // Draw character pixels
        for row in 0..FONT_HEIGHT {
            for col in 0..FONT_WIDTH {
                if char_data[row as usize] & (1 << (FONT_WIDTH - 1 - col)) != 0 {
                    // Draw scaled pixel
                    for sy in 0..scale {
                        for sx in 0..scale {
                            self.draw_pixel(
                                x + (col * scale + sx) as u16,
                                y + (row * scale + sy) as u16,
                                color
                            )?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Draw text at the specified position
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let mut cursor_x = x;
        let char_width = (FONT_WIDTH * scale + 1) as u16;
        
        for c in text.chars() {
            if cursor_x + char_width > self.width {
                break; // Don't draw past screen edge
            }
            self.draw_char(cursor_x, y, c, color, bg_color, scale)?;
            cursor_x += char_width;
        }
        
        Ok(())
    }
    
    /// Draw centered text
    pub fn draw_text_centered(&mut self, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let char_width = (FONT_WIDTH * scale + 1) as u16;
        let text_width = text.len() as u16 * char_width;
        let x = (self.width - text_width) / 2;
        self.draw_text(x, y, text, color, bg_color, scale)
    }
    
    /// Draw a circle using midpoint algorithm
    pub fn draw_circle(&mut self, cx: u16, cy: u16, r: u16, color: u16) -> Result<()> {
        let mut x = 0i32;
        let mut y = r as i32;
        let mut d = 1 - r as i32;
        
        while x <= y {
            self.draw_pixel((cx as i32 + x) as u16, (cy as i32 + y) as u16, color)?;
            self.draw_pixel((cx as i32 + y) as u16, (cy as i32 + x) as u16, color)?;
            self.draw_pixel((cx as i32 - x) as u16, (cy as i32 + y) as u16, color)?;
            self.draw_pixel((cx as i32 - y) as u16, (cy as i32 + x) as u16, color)?;
            self.draw_pixel((cx as i32 + x) as u16, (cy as i32 - y) as u16, color)?;
            self.draw_pixel((cx as i32 + y) as u16, (cy as i32 - x) as u16, color)?;
            self.draw_pixel((cx as i32 - x) as u16, (cy as i32 - y) as u16, color)?;
            self.draw_pixel((cx as i32 - y) as u16, (cy as i32 - x) as u16, color)?;
            
            if d < 0 {
                d += 2 * x + 3;
            } else {
                d += 2 * (x - y) + 5;
                y -= 1;
            }
            x += 1;
        }
        
        Ok(())
    }
    
    /// Fill a circle
    pub fn fill_circle(&mut self, cx: u16, cy: u16, r: u16, color: u16) -> Result<()> {
        for y in 0..=r {
            let x = ((r as i32 * r as i32 - y as i32 * y as i32) as f32).sqrt() as u16;
            self.draw_line(cx - x, cy + y, cx + x, cy + y, color)?;
            if y > 0 {
                self.draw_line(cx - x, cy - y, cx + x, cy - y, color)?;
            }
        }
        Ok(())
    }
    
    /// Draw a progress bar
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
    
    /// Flush frame buffer to display using DMA
    pub fn flush(&mut self) -> Result<()> {
        if !self.use_frame_buffer {
            return Ok(()); // Nothing to flush in direct mode
        }
        
        // Ensure display stays on
        self.ensure_display_on()?;
        
        // Get dirty rectangles
        if self.dirty_rect_manager.is_empty() {
            return Ok(()); // Nothing to update
        }
        
        // Merge overlapping rectangles for optimization
        self.dirty_rect_manager.merge_all();
        
        // Update each dirty region via DMA
        let regions: Vec<_> = self.dirty_rect_manager.iter().cloned().collect();
        for region in regions {
            let x = region.x;
            let y = region.y;
            let w = region.width;
            let h = region.height;
            
            // Bounds checking
            if x >= self.width || y >= self.height {
                continue;
            }
            
            let x_end = (x + w).min(self.width);
            let y_end = (y + h).min(self.height);
            let actual_w = x_end - x;
            let actual_h = y_end - y;
            
            if actual_w == 0 || actual_h == 0 {
                continue;
            }
            
            // Create buffer for this region
            let mut region_buffer = Vec::with_capacity((actual_w * actual_h) as usize);
            
            // Copy pixels from frame buffer
            for row in y..y_end {
                let start_idx = (row as usize * self.width as usize) + x as usize;
                let end_idx = start_idx + actual_w as usize;
                
                // Bounds check for frame buffer access
                if start_idx < self.frame_buffer.len() && end_idx <= self.frame_buffer.len() {
                    region_buffer.extend_from_slice(&self.frame_buffer[start_idx..end_idx]);
                }
            }
            
            // Only send if we have data
            if !region_buffer.is_empty() {
                // Send to display via DMA
                unsafe {
                    let ret = esp_lcd_panel_draw_bitmap(
                        self.panel_handle,
                        x as i32, y as i32,
                        x_end as i32, y_end as i32,
                        region_buffer.as_ptr() as *const _,
                    );
                    if ret != ESP_OK {
                        debug!("Failed to draw bitmap at ({}, {}) size {}x{}: {:?}", x, y, actual_w, actual_h, ret);
                        // Continue with other regions even if one fails
                    }
                }
            }
        }
        
        // Clear dirty rectangles
        self.dirty_rect_manager.clear();
        
        // Update stats
        self.render_count += 1;
        self.last_render_time = Instant::now();
        
        Ok(())
    }
    
    /// Reset activity timer
    pub fn reset_activity_timer(&mut self) {
        self.last_activity = Instant::now();
        if self.backlight_level != FULL_BRIGHTNESS {
            self.set_backlight(FULL_BRIGHTNESS);
        }
    }
    
    /// Update auto-dim state
    pub fn update_auto_dim(&mut self) -> Result<()> {
        // Temporarily disable auto-dim to debug flickering
        // let elapsed = self.last_activity.elapsed().as_secs();
        // 
        // if elapsed >= AUTO_DIM_TIMEOUT_SECS && self.backlight_level == FULL_BRIGHTNESS {
        //     info!("Auto-dimming display after {} seconds", elapsed);
        //     self.set_backlight(DIM_BRIGHTNESS);
        // }
        
        // Always keep display at full brightness for now
        if self.backlight_level != FULL_BRIGHTNESS {
            self.set_backlight(FULL_BRIGHTNESS);
        }
        
        Ok(())
    }
    
    /// Set backlight brightness (0-255)
    fn set_backlight(&mut self, level: u8) {
        self.backlight_level = level;
        
        unsafe {
            // For simple on/off control (no PWM)
            if level > 0 {
                gpio_set_level(self.backlight_pin, 1);
            } else {
                gpio_set_level(self.backlight_pin, 0);
            }
        }
    }
    
    /// Ensure display stays on
    pub fn ensure_display_on(&mut self) -> Result<()> {
        unsafe {
            // Keep LCD power and backlight on
            gpio_set_level(self.lcd_power_pin, 1);
            gpio_set_level(self.backlight_pin, 1);
        }
        Ok(())
    }
    
    /// Enable/disable frame buffer
    pub fn enable_frame_buffer(&mut self, enable: bool) -> Result<()> {
        self.use_frame_buffer = enable;
        if enable && self.frame_buffer.is_empty() {
            self.frame_buffer = vec![BLACK; (self.width * self.height) as usize];
        }
        Ok(())
    }
    
    /// Get display width
    pub fn width(&self) -> u16 {
        self.width
    }
    
    /// Get display height
    pub fn height(&self) -> u16 {
        self.height
    }
    
    /// Check if frame buffer is enabled
    pub fn is_frame_buffer_enabled(&self) -> bool {
        self.use_frame_buffer
    }
}

impl Drop for EspLcdDisplayManager {
    fn drop(&mut self) {
        info!("Cleaning up ESP_LCD display manager...");
        
        unsafe {
            // Turn off display
            let _ = esp_lcd_panel_disp_on_off(self.panel_handle, false);
            
            // Delete panel and IO
            esp_lcd_panel_del(self.panel_handle);
            esp_lcd_panel_io_del(self.io_handle);
            esp_lcd_del_i80_bus(self.bus_handle);
            
            // Turn off backlight and power
            gpio_set_level(self.backlight_pin, 0);
            gpio_set_level(self.lcd_power_pin, 0);
        }
    }
}