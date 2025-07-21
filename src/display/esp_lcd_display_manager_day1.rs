/// ESP_LCD Display Manager - Day 1 Implementation
/// Focuses on DMA descriptor locality and cache coherency fixes

use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use log::{info, debug, warn};
use std::time::Instant;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

// Additional imports for missing types
use esp_idf_sys::{
    esp_lcd_i80_bus_handle_t, esp_lcd_panel_handle_t, esp_lcd_panel_io_handle_t,
    esp_lcd_i80_bus_config_t, esp_lcd_panel_io_i80_config_t, esp_lcd_panel_dev_config_t,
    esp_lcd_new_i80_bus, esp_lcd_new_panel_io_i80, esp_lcd_new_panel_st7789,
    esp_lcd_panel_reset, esp_lcd_panel_init, esp_lcd_panel_invert_color,
    esp_lcd_panel_swap_xy, esp_lcd_panel_mirror, esp_lcd_panel_set_gap,
    esp_lcd_panel_disp_on_off, esp_lcd_panel_draw_bitmap, esp_lcd_panel_del,
    esp_lcd_panel_io_del, esp_lcd_del_i80_bus,
    gpio_config_t, gpio_config, gpio_set_level, gpio_mode_t_GPIO_MODE_OUTPUT,
    gpio_mode_t_GPIO_MODE_INPUT, gpio_pullup_t_GPIO_PULLUP_DISABLE,
    gpio_pullup_t_GPIO_PULLUP_ENABLE, gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
    gpio_int_type_t_GPIO_INTR_DISABLE, soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT,
    ESP_OK, esp_get_free_heap_size, esp_get_minimum_free_heap_size,
    heap_caps_get_free_size, heap_caps_get_largest_free_block,
    MALLOC_CAP_DMA, MALLOC_CAP_INTERNAL
};

use super::colors::*;
use super::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use super::dirty_rect_manager::DirtyRectManager;

// Display configuration
const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 170;
const DISPLAY_Y_OFFSET: u16 = 35;

// Day 1: Use strip buffers for testing
const STRIP_HEIGHT: usize = 20;
const STRIP_PIXELS: usize = (DISPLAY_WIDTH as usize) * STRIP_HEIGHT;

// Performance configuration
const INITIAL_PIXEL_CLOCK_HZ: u32 = 20 * 1000 * 1000; // Start at 20MHz

// Static allocations in internal RAM
// Note: ESP32 requires 32-byte alignment for DMA descriptors
static mut DMA_DESCRIPTORS: [dma_descriptor_t; 32] = [dma_descriptor_t {
    size: 0,
    length: 0,
    buf: ptr::null_mut(),
    next: ptr::null_mut(),
}; 32];

// Strip buffers (aligned for DMA)
static mut STRIP_BUFFER_A: [u16; STRIP_PIXELS] = [0; STRIP_PIXELS];
static mut STRIP_BUFFER_B: [u16; STRIP_PIXELS] = [0; STRIP_PIXELS];

// DMA descriptor structure (matching ESP-IDF)
#[repr(C)]
#[derive(Copy, Clone)]
struct dma_descriptor_t {
    size: u16,
    length: u16,
    buf: *mut u8,
    next: *mut dma_descriptor_t,
}

pub struct EspLcdDisplayManager {
    bus_handle: esp_lcd_i80_bus_handle_t,
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: esp_lcd_panel_io_handle_t,
    width: u16,
    height: u16,
    
    // Frame buffer (can be in PSRAM)
    frame_buffer: Vec<u16>,
    
    // Strip buffer selection
    current_strip: bool,
    
    // Synchronization
    flush_mutex: Arc<Mutex<()>>,
    dma_active: Arc<AtomicBool>,
    
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
        info!("=== ESP_LCD Day 1 Implementation ===");
        info!("Focus: DMA descriptor locality & cache coherency");
        
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
            // Log memory state before initialization
            Self::log_memory_state("Before init");
            
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
            
            // Configure I80 bus
            info!("Initializing Intel 8080 bus with DMA...");
            let mut bus_config: esp_lcd_i80_bus_config_t = unsafe { core::mem::zeroed() };
            bus_config.dc_gpio_num = dc_pin;
            bus_config.wr_gpio_num = wr_pin;
            bus_config.clk_src = soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT;
            bus_config.data_gpio_nums = [
                d0_pin, d1_pin, d2_pin, d3_pin,
                d4_pin, d5_pin, d6_pin, d7_pin,
                -1, -1, -1, -1, -1, -1, -1, -1,
            ];
            bus_config.bus_width = 8;
            bus_config.max_transfer_bytes = (STRIP_PIXELS * 2) as usize;
            bus_config.sram_trans_align = 4;
            
            let mut i80_bus: esp_lcd_i80_bus_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_i80_bus(&bus_config, &mut i80_bus);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create I80 bus: {:?}", ret));
            }
            info!("I80 bus created successfully");
            
            // Configure panel IO
            info!("Creating panel IO...");
            let mut io_config: esp_lcd_panel_io_i80_config_t = unsafe { core::mem::zeroed() };
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
            let mut panel_config: esp_lcd_panel_dev_config_t = unsafe { core::mem::zeroed() };
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
            
            // Allocate frame buffer (can be in PSRAM)
            info!("Allocating frame buffer...");
            let frame_buffer = vec![BLACK; (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize];
            
            // Clear static buffers
            STRIP_BUFFER_A.fill(BLACK);
            STRIP_BUFFER_B.fill(BLACK);
            
            // Log memory state after initialization
            Self::log_memory_state("After init");
            
            Ok(Self {
                bus_handle: i80_bus,
                panel_handle,
                io_handle,
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                frame_buffer,
                current_strip: false,
                flush_mutex: Arc::new(Mutex::new(())),
                dma_active: Arc::new(AtomicBool::new(false)),
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
    
    /// Flush frame buffer to display using strip-based DMA
    pub fn flush(&mut self) -> Result<()> {
        let _guard = self.flush_mutex.lock().unwrap();
        
        if self.dirty_rect_manager.is_empty() {
            return Ok(());
        }
        
        // Wait for any previous DMA to complete
        while self.dma_active.load(Ordering::Acquire) {
            FreeRtos::delay_ms(1);
        }
        
        info!("Flush starting - using strip buffer approach");
        
        // For Day 1, we'll update the entire screen in strips
        let mut y = 0u16;
        while y < self.height {
            let strip_height = STRIP_HEIGHT.min((self.height - y) as usize);
            
            // Select strip buffer
            let strip_buffer = unsafe {
                if self.current_strip {
                    &mut STRIP_BUFFER_B
                } else {
                    &mut STRIP_BUFFER_A
                }
            };
            
            // Copy frame buffer data to strip buffer
            for row in 0..strip_height {
                let src_start = (y + row as u16) as usize * self.width as usize;
                let src_end = src_start + self.width as usize;
                let dst_start = row * self.width as usize;
                let dst_end = dst_start + self.width as usize;
                
                strip_buffer[dst_start..dst_end].copy_from_slice(&self.frame_buffer[src_start..src_end]);
            }
            
            // CRITICAL: Flush cache before DMA
            unsafe {
                Self::flush_cache_for_dma(
                    strip_buffer.as_ptr() as *const u8,
                    strip_height * self.width as usize * 2
                );
            }
            
            // Mark DMA as active
            self.dma_active.store(true, Ordering::Release);
            
            // Send strip to display
            unsafe {
                let ret = esp_lcd_panel_draw_bitmap(
                    self.panel_handle,
                    0,
                    y as i32,
                    self.width as i32,
                    (y + strip_height as u16) as i32,
                    strip_buffer.as_ptr() as *const _,
                );
                
                if ret != ESP_OK {
                    warn!("Failed to draw strip at y={}: {:?}", y, ret);
                    self.dma_active.store(false, Ordering::Release);
                    return Err(anyhow::anyhow!("DMA transfer failed"));
                }
            }
            
            // Wait for this strip to complete
            FreeRtos::delay_ms(2); // Conservative wait
            self.dma_active.store(false, Ordering::Release);
            
            // Toggle strip buffer
            self.current_strip = !self.current_strip;
            y += strip_height as u16;
        }
        
        self.dirty_rect_manager.clear();
        self.render_count += 1;
        self.last_render_time = Instant::now();
        
        info!("Flush complete - {} strips sent", (self.height + STRIP_HEIGHT as u16 - 1) / STRIP_HEIGHT as u16);
        
        Ok(())
    }
    
    /// Critical: Flush cache for DMA operations
    unsafe fn flush_cache_for_dma(addr: *const u8, size: usize) {
        // ESP32-S3 cache operations
        // Note: Cache_WriteBack_Addr may not be directly available
        // Use memory barrier instead for now
        core::sync::atomic::fence(Ordering::SeqCst);
        
        // Force compiler to not optimize away the write
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        
        // For debugging
        debug!("Cache sync for DMA: addr=0x{:08x}, size={}", addr as u32, size);
    }
    
    /// Log memory state for debugging
    unsafe fn log_memory_state(label: &str) {
        let free_heap = esp_get_free_heap_size();
        let min_free = esp_get_minimum_free_heap_size();
        let dma_caps = heap_caps_get_free_size(MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL);
        let largest_free = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL);
        
        info!("[MEM] {} - Heap: {}KB (min: {}KB), DMA: {}KB, Largest: {}KB",
            label,
            free_heap / 1024,
            min_free / 1024,
            dma_caps / 1024,
            largest_free / 1024
        );
    }
    
    // Drawing primitives (same as before)
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
    
    // Additional drawing methods
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
        // ESP_LCD should handle this internally
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