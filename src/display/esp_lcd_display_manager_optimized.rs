/// ESP_LCD Display Manager - Optimized Progressive Implementation
/// Builds on simplified version with progressive enhancements
///
/// Strategy:
/// 1. Start with proven line-by-line approach
/// 2. Progressively increase buffer sizes (1 → 5 → 10 → 20 lines)
/// 3. Add performance monitoring to find optimal configuration
/// 4. Implement proper DMA descriptor chaining

// Note: DMA descriptor types are not exposed by esp-idf-sys
// The esp_lcd API handles DMA internally

use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use log::{info, debug, warn, error};
use std::time::Instant;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::colors::*;
use super::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use super::dirty_rect_manager::DirtyRectManager;

// Display configuration
const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 170;
const DISPLAY_Y_OFFSET: u16 = 35;

// Progressive buffer configuration
const MIN_LINES_PER_TRANSFER: usize = 1;
const MAX_LINES_PER_TRANSFER: usize = 20;
const INITIAL_LINES_PER_TRANSFER: usize = 1; // Start with just 1 line to test

// Performance configuration
const INITIAL_PIXEL_CLOCK_HZ: u32 = 10 * 1000 * 1000; // 10MHz - start slower

// Static buffer for DMA transfers (aligned, in internal RAM)
#[repr(align(64))]
struct AlignedBuffer {
    data: [u16; DISPLAY_WIDTH as usize * MAX_LINES_PER_TRANSFER],
}

// Force buffer into internal RAM
#[link_section = ".dram2_uninit"]
static mut DMA_BUFFER: AlignedBuffer = AlignedBuffer {
    data: [0; DISPLAY_WIDTH as usize * MAX_LINES_PER_TRANSFER],
};

pub struct EspLcdDisplayManager {
    bus_handle: esp_lcd_i80_bus_handle_t,
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: esp_lcd_panel_io_handle_t,
    width: u16,
    height: u16,
    
    // Frame buffer (can be in PSRAM)
    frame_buffer: Vec<u16>,
    
    // Progressive configuration
    lines_per_transfer: AtomicU32,
    
    // Performance metrics
    total_transfer_time: Arc<Mutex<u128>>,
    transfer_count: Arc<AtomicU32>,
    
    // Synchronization
    flush_mutex: Arc<Mutex<()>>,
    
    dirty_rect_manager: DirtyRectManager,
    last_render_time: Instant,
    render_count: u32,
    backlight_pin: i32,
    lcd_power_pin: i32,
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
        _rd: impl Into<AnyIOPin>,
    ) -> Result<Self> {
        info!("=== ESP_LCD Optimized Progressive Implementation ===");
        info!("Starting with {} lines per transfer", INITIAL_LINES_PER_TRANSFER);
        
        // Convert pins
        let d0_pin = 39i32;
        let d1_pin = 40i32;
        let d2_pin = 41i32;
        let d3_pin = 42i32;
        let d4_pin = 45i32;
        let d5_pin = 46i32;
        let d6_pin = 47i32;
        let d7_pin = 48i32;
        let wr_pin = 8i32;
        let dc_pin = 7i32;
        let cs_pin = 6i32;
        let rst_pin = 5i32;
        let backlight_pin = 38i32;
        let lcd_power_pin = 15i32;
        let rd_pin = 9i32;
        
        let _ = (d0.into(), d1.into(), d2.into(), d3.into(), 
                 d4.into(), d5.into(), d6.into(), d7.into(),
                 wr.into(), dc.into(), cs.into(), rst.into(),
                 backlight.into(), lcd_power.into(), _rd.into());
        
        unsafe {
            // Initialize power pins
            info!("Initializing LCD power and backlight pins...");
            let lcd_power_config = gpio_config_t {
                pin_bit_mask: 1u64 << lcd_power_pin,
                mode: gpio_mode_t_GPIO_MODE_OUTPUT,
                pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
                pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            gpio_config(&lcd_power_config);
            gpio_set_level(lcd_power_pin, 1);
            
            let backlight_config = gpio_config_t {
                pin_bit_mask: 1u64 << backlight_pin,
                mode: gpio_mode_t_GPIO_MODE_OUTPUT,
                pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
                pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            gpio_config(&backlight_config);
            gpio_set_level(backlight_pin, 1);
            
            FreeRtos::delay_ms(100);
            
            // Configure RD pin as input with pull-up
            let lcd_rd_gpio_config = gpio_config_t {
                pin_bit_mask: 1u64 << rd_pin,
                mode: gpio_mode_t_GPIO_MODE_INPUT,
                pull_up_en: gpio_pullup_t_GPIO_PULLUP_ENABLE,
                pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
                intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            };
            gpio_config(&lcd_rd_gpio_config);
            
            // Configure I80 bus with progressive buffer size
            info!("Initializing Intel 8080 bus with progressive DMA...");
            let mut bus_config: esp_lcd_i80_bus_config_t = core::mem::zeroed();
            bus_config.dc_gpio_num = dc_pin;
            bus_config.wr_gpio_num = wr_pin;
            bus_config.clk_src = soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT;
            bus_config.data_gpio_nums = [
                d0_pin, d1_pin, d2_pin, d3_pin,
                d4_pin, d5_pin, d6_pin, d7_pin,
                -1, -1, -1, -1, -1, -1, -1, -1,
            ];
            bus_config.bus_width = 8;
            bus_config.max_transfer_bytes = DISPLAY_WIDTH as usize * MAX_LINES_PER_TRANSFER * 2;
            bus_config.sram_trans_align = 64; // Better alignment for DMA
            
            let mut i80_bus: esp_lcd_i80_bus_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_i80_bus(&bus_config, &mut i80_bus);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create I80 bus: {:?}", ret));
            }
            info!("I80 bus created successfully");
            
            // Configure panel IO
            info!("Creating panel IO...");
            let mut io_config: esp_lcd_panel_io_i80_config_t = core::mem::zeroed();
            io_config.cs_gpio_num = cs_pin;
            io_config.pclk_hz = INITIAL_PIXEL_CLOCK_HZ;
            io_config.trans_queue_depth = 10;
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
            let mut panel_config: esp_lcd_panel_dev_config_t = core::mem::zeroed();
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
            
            // Configure display
            info!("Configuring display orientation...");
            esp_lcd_panel_invert_color(panel_handle, true);
            esp_lcd_panel_swap_xy(panel_handle, true);
            esp_lcd_panel_mirror(panel_handle, false, true);
            esp_lcd_panel_set_gap(panel_handle, 0, DISPLAY_Y_OFFSET as i32);
            
            info!("Turning display on...");
            esp_lcd_panel_disp_on_off(panel_handle, true);
            
            // Allocate frame buffer
            info!("Allocating frame buffer...");
            let frame_buffer = vec![BLACK; (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize];
            
            Ok(Self {
                bus_handle: i80_bus,
                panel_handle,
                io_handle,
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                frame_buffer,
                lines_per_transfer: AtomicU32::new(INITIAL_LINES_PER_TRANSFER as u32),
                total_transfer_time: Arc::new(Mutex::new(0)),
                transfer_count: Arc::new(AtomicU32::new(0)),
                flush_mutex: Arc::new(Mutex::new(())),
                dirty_rect_manager: DirtyRectManager::new(),
                last_render_time: Instant::now(),
                render_count: 0,
                backlight_pin,
                lcd_power_pin,
                initialized: true,
            })
        }
    }
    
    /// Clear the entire display with a color
    pub fn clear(&mut self, color: u16) -> Result<()> {
        debug!("Clear called with color: 0x{:04X}", color);
        self.frame_buffer.fill(color);
        self.dirty_rect_manager.add_rect(0, 0, self.width, self.height);
        Ok(())
    }
    
    /// Draw a single pixel
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }
        
        let idx = (y as usize * self.width as usize) + x as usize;
        self.frame_buffer[idx] = color;
        self.dirty_rect_manager.add_rect(x, y, 1, 1);
        
        Ok(())
    }
    
    /// Fill a rectangle with a color
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }
        
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        
        for row in y..y_end {
            let start_idx = (row as usize * self.width as usize) + x as usize;
            let end_idx = start_idx + (x_end - x) as usize;
            self.frame_buffer[start_idx..end_idx].fill(color);
        }
        
        self.dirty_rect_manager.add_rect(x, y, x_end - x, y_end - y);
        Ok(())
    }
    
    /// Flush frame buffer to display with progressive optimization
    pub fn flush(&mut self) -> Result<()> {
        let _guard = self.flush_mutex.lock().unwrap();
        
        if self.dirty_rect_manager.is_empty() {
            return Ok(());
        }
        
        let lines_per_transfer = self.lines_per_transfer.load(Ordering::Relaxed) as usize;
        let flush_start = Instant::now();
        
        debug!("Flush starting - {} lines per transfer", lines_per_transfer);
        
        // Update entire screen in strips
        let mut y = 0u16;
        let mut transfer_errors = 0u32;
        
        while y < self.height {
            let lines_to_transfer = lines_per_transfer.min((self.height - y) as usize);
            let transfer_start = Instant::now();
            
            // Copy frame buffer data to DMA buffer
            unsafe {
                let src_start = y as usize * self.width as usize;
                let copy_size = lines_to_transfer * self.width as usize;
                
                DMA_BUFFER.data[..copy_size].copy_from_slice(
                    &self.frame_buffer[src_start..src_start + copy_size]
                );
                
                // Memory barrier for cache coherency
                core::sync::atomic::fence(Ordering::SeqCst);
                core::sync::atomic::compiler_fence(Ordering::SeqCst);
            }
            
            // Send to display
            unsafe {
                debug!("Drawing bitmap: x=0, y={}, w={}, h={}, lines={}", 
                       y, self.width, y + lines_to_transfer as u16, lines_to_transfer);
                
                let ret = esp_lcd_panel_draw_bitmap(
                    self.panel_handle,
                    0,
                    y as i32,
                    self.width as i32,
                    (y + lines_to_transfer as u16) as i32,
                    DMA_BUFFER.data.as_ptr() as *const _,
                );
                
                if ret != ESP_OK {
                    warn!("Transfer failed at y={}: {:?}", y, ret);
                    transfer_errors += 1;
                    
                    // Fall back to smaller transfer size
                    if lines_per_transfer > MIN_LINES_PER_TRANSFER {
                        let new_size = (lines_per_transfer / 2).max(MIN_LINES_PER_TRANSFER);
                        self.lines_per_transfer.store(new_size as u32, Ordering::Relaxed);
                        warn!("Reducing transfer size to {} lines", new_size);
                    }
                }
            }
            
            let transfer_time = transfer_start.elapsed();
            *self.total_transfer_time.lock().unwrap() += transfer_time.as_micros();
            self.transfer_count.fetch_add(1, Ordering::Relaxed);
            
            y += lines_to_transfer as u16;
        }
        
        let flush_time = flush_start.elapsed();
        
        // Adaptive optimization - adjust transfer size based on performance
        if transfer_errors == 0 && self.render_count % 100 == 0 {
            let avg_transfer_time = {
                let total = *self.total_transfer_time.lock().unwrap();
                let count = self.transfer_count.load(Ordering::Relaxed);
                if count > 0 { total / count as u128 } else { 0 }
            };
            
            info!("Performance: {} lines/transfer, avg {}µs/transfer, total {}ms",
                lines_per_transfer, avg_transfer_time, flush_time.as_millis());
            
            // Try increasing transfer size if performance is good
            if avg_transfer_time < 5000 && lines_per_transfer < MAX_LINES_PER_TRANSFER {
                let new_size = (lines_per_transfer * 2).min(MAX_LINES_PER_TRANSFER);
                self.lines_per_transfer.store(new_size as u32, Ordering::Relaxed);
                info!("Increasing transfer size to {} lines", new_size);
                
                // Reset metrics
                *self.total_transfer_time.lock().unwrap() = 0;
                self.transfer_count.store(0, Ordering::Relaxed);
            }
        }
        
        self.dirty_rect_manager.clear();
        self.render_count += 1;
        self.last_render_time = Instant::now();
        
        Ok(())
    }
    
    // Drawing primitives
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
        self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, color)?;
        self.draw_line(x + w - 1, y + h - 1, x, y + h - 1, color)?;
        self.draw_line(x, y + h - 1, x, y, color)?;
        Ok(())
    }
    
    pub fn draw_char(&mut self, x: u16, y: u16, c: char, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        let char_data = get_char_data(c);
        let char_width = FONT_WIDTH * scale;
        let char_height = FONT_HEIGHT * scale;
        
        if let Some(bg) = bg_color {
            self.fill_rect(x, y, char_width as u16, char_height as u16, bg)?;
        }
        
        for row in 0..FONT_HEIGHT {
            for col in 0..FONT_WIDTH {
                if char_data[row as usize] & (1 << (FONT_WIDTH - 1 - col)) != 0 {
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
    
    pub fn fill_circle(&mut self, cx: u16, cy: u16, radius: u16, color: u16) -> Result<()> {
        let r = radius as i32;
        let cx = cx as i32;
        let cy = cy as i32;
        
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r {
                    let px = (cx + x) as u16;
                    let py = (cy + y) as u16;
                    if px < self.width && py < self.height {
                        self.draw_pixel(px, py, color)?;
                    }
                }
            }
        }
        Ok(())
    }
    
    pub fn draw_circle(&mut self, cx: u16, cy: u16, radius: u16, color: u16) -> Result<()> {
        let r = radius as i32;
        let cx = cx as i32;
        let cy = cy as i32;
        
        // Use Bresenham's circle algorithm
        let mut x = 0;
        let mut y = r;
        let mut d = 3 - 2 * r;
        
        while x <= y {
            self.draw_pixel((cx + x) as u16, (cy + y) as u16, color)?;
            self.draw_pixel((cx + y) as u16, (cy + x) as u16, color)?;
            self.draw_pixel((cx - x) as u16, (cy + y) as u16, color)?;
            self.draw_pixel((cx - y) as u16, (cy + x) as u16, color)?;
            self.draw_pixel((cx + x) as u16, (cy - y) as u16, color)?;
            self.draw_pixel((cx + y) as u16, (cy - x) as u16, color)?;
            self.draw_pixel((cx - x) as u16, (cy - y) as u16, color)?;
            self.draw_pixel((cx - y) as u16, (cy - x) as u16, color)?;
            
            if d < 0 {
                d += 4 * x + 6;
            } else {
                d += 4 * (x - y) + 10;
                y -= 1;
            }
            x += 1;
        }
        Ok(())
    }
    
    pub fn draw_progress_bar(&mut self, x: u16, y: u16, w: u16, h: u16, progress: u8, 
                            fill_color: u16, bg_color: u16, border_color: u16) -> Result<()> {
        // Draw border
        self.draw_rect(x, y, w, h, border_color)?;
        
        // Fill background
        if w > 2 && h > 2 {
            self.fill_rect(x + 1, y + 1, w - 2, h - 2, bg_color)?;
            
            // Fill progress
            let progress_width = ((w - 2) as u32 * progress as u32 / 100) as u16;
            if progress_width > 0 {
                self.fill_rect(x + 1, y + 1, progress_width, h - 2, fill_color)?;
            }
        }
        Ok(())
    }
    
    pub fn ensure_display_on(&mut self) -> Result<()> {
        Ok(())
    }
    
    // Required trait methods
    pub fn width(&self) -> u16 { self.width }
    pub fn height(&self) -> u16 { self.height }
    pub fn reset_activity_timer(&mut self) {}
    pub fn update_auto_dim(&mut self) -> Result<()> { Ok(()) }
    pub fn enable_frame_buffer(&mut self, _enable: bool) -> Result<()> { Ok(()) }
    pub fn is_frame_buffer_enabled(&self) -> bool { true }
}

impl Drop for EspLcdDisplayManager {
    fn drop(&mut self) {
        info!("Cleaning up ESP_LCD display manager...");
        
        // Log final performance metrics
        let total_transfers = self.transfer_count.load(Ordering::Relaxed);
        let total_time = *self.total_transfer_time.lock().unwrap();
        if total_transfers > 0 {
            info!("Final stats: {} transfers, avg {}µs/transfer",
                total_transfers, total_time / total_transfers as u128);
        }
        
        unsafe {
            let _ = esp_lcd_panel_disp_on_off(self.panel_handle, false);
            esp_lcd_panel_del(self.panel_handle);
            esp_lcd_panel_io_del(self.io_handle);
            esp_lcd_del_i80_bus(self.bus_handle);
            
            gpio_set_level(self.backlight_pin, 0);
            gpio_set_level(self.lcd_power_pin, 0);
        }
    }
}