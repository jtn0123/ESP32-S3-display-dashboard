/// ESP_LCD Display Manager - Day 1 Simplified
/// Ultra-minimal approach to isolate DMA issues

use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use log::{info, debug, warn, error};
use std::time::Instant;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use super::colors::*;
use super::font5x7::{FONT_WIDTH, FONT_HEIGHT, get_char_data};
use super::dirty_rect_manager::DirtyRectManager;

// Display configuration
const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 170;
const DISPLAY_Y_OFFSET: u16 = 35;

// Performance configuration
const INITIAL_PIXEL_CLOCK_HZ: u32 = 10 * 1000 * 1000; // Start slower at 10MHz

pub struct EspLcdDisplayManager {
    bus_handle: esp_lcd_i80_bus_handle_t,
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: esp_lcd_panel_io_handle_t,
    width: u16,
    height: u16,
    
    // Single line buffer for minimal testing
    line_buffer: Vec<u16>,
    
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
        info!("=== ESP_LCD Day 1 Simplified ===");
        info!("Testing minimal DMA approach");
        
        // Convert pins (same as before)
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
            
            // Configure I80 bus - smaller transfer size
            info!("Initializing Intel 8080 bus with minimal DMA...");
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
            bus_config.max_transfer_bytes = DISPLAY_WIDTH as usize * 2; // Just one line
            bus_config.sram_trans_align = 4;
            
            let mut i80_bus: esp_lcd_i80_bus_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_i80_bus(&bus_config, &mut i80_bus);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create I80 bus: {:?}", ret));
            }
            info!("I80 bus created successfully");
            
            // Configure panel IO with lower clock
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
            
            // Allocate single line buffer
            info!("Allocating line buffer...");
            let line_buffer = vec![BLACK; DISPLAY_WIDTH as usize];
            
            Ok(Self {
                bus_handle: i80_bus,
                panel_handle,
                io_handle,
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                line_buffer,
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
    
    /// Test with a single color
    pub fn test_single_color(&mut self, color: u16) -> Result<()> {
        info!("Testing single color fill: 0x{:04X}", color);
        
        // Fill line buffer with color
        self.line_buffer.fill(color);
        
        // Try to draw just the first line
        info!("Drawing first line...");
        unsafe {
            let ret = esp_lcd_panel_draw_bitmap(
                self.panel_handle,
                0, 0,
                self.width as i32, 1, // Just one line!
                self.line_buffer.as_ptr() as *const _,
            );
            
            if ret != ESP_OK {
                error!("Failed to draw single line: {:?}", ret);
                return Err(anyhow::anyhow!("Single line draw failed"));
            }
        }
        
        info!("Single line drawn successfully!");
        
        // If that worked, try the whole screen line by line
        info!("Drawing full screen line by line...");
        for y in 0..self.height {
            unsafe {
                let ret = esp_lcd_panel_draw_bitmap(
                    self.panel_handle,
                    0, y as i32,
                    self.width as i32, (y + 1) as i32,
                    self.line_buffer.as_ptr() as *const _,
                );
                
                if ret != ESP_OK {
                    error!("Failed to draw line {}: {:?}", y, ret);
                    return Err(anyhow::anyhow!("Line draw failed at y={}", y));
                }
            }
            
            // Small delay to see progress
            if y % 10 == 0 {
                debug!("Drew {} lines", y);
            }
        }
        
        info!("Full screen drawn successfully!");
        Ok(())
    }
    
    /// Clear the entire display with a color
    pub fn clear(&mut self, color: u16) -> Result<()> {
        debug!("Clear called with color: 0x{:04X}", color);
        self.test_single_color(color)
    }
    
    // Stub implementations for other methods
    pub fn draw_pixel(&mut self, _x: u16, _y: u16, _color: u16) -> Result<()> { Ok(()) }
    pub fn fill_rect(&mut self, _x: u16, _y: u16, _w: u16, _h: u16, _color: u16) -> Result<()> { Ok(()) }
    pub fn flush(&mut self) -> Result<()> { Ok(()) }
    pub fn draw_line(&mut self, _x0: u16, _y0: u16, _x1: u16, _y1: u16, _color: u16) -> Result<()> { Ok(()) }
    pub fn draw_rect(&mut self, _x: u16, _y: u16, _w: u16, _h: u16, _color: u16) -> Result<()> { Ok(()) }
    pub fn draw_char(&mut self, _x: u16, _y: u16, _c: char, _color: u16, _bg_color: Option<u16>, _scale: u8) -> Result<()> { Ok(()) }
    pub fn draw_text(&mut self, _x: u16, _y: u16, _text: &str, _color: u16, _bg_color: Option<u16>, _scale: u8) -> Result<()> { Ok(()) }
    pub fn draw_text_centered(&mut self, _y: u16, _text: &str, _color: u16, _bg_color: Option<u16>, _scale: u8) -> Result<()> { Ok(()) }
    pub fn fill_circle(&mut self, _cx: u16, _cy: u16, _radius: u16, _color: u16) -> Result<()> { Ok(()) }
    pub fn draw_circle(&mut self, _cx: u16, _cy: u16, _radius: u16, _color: u16) -> Result<()> { Ok(()) }
    pub fn draw_progress_bar(&mut self, _x: u16, _y: u16, _w: u16, _h: u16, _progress: u8, _fill_color: u16, _bg_color: u16, _border_color: u16) -> Result<()> { Ok(()) }
    pub fn ensure_display_on(&mut self) -> Result<()> { Ok(()) }
    pub fn width(&self) -> u16 { self.width }
    pub fn height(&self) -> u16 { self.height }
    pub fn reset_activity_timer(&mut self) {}
    pub fn update_auto_dim(&mut self) -> Result<()> { Ok(()) }
    pub fn enable_frame_buffer(&mut self, _enable: bool) -> Result<()> { Ok(()) }
    pub fn is_frame_buffer_enabled(&self) -> bool { false }
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