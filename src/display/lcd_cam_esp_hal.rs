/// LCD_CAM implementation using ESP-IDF LCD driver
/// This provides hardware-accelerated display output via the LCD_CAM peripheral
use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::{Pin, Gpio39, Gpio40, Gpio41, Gpio42, Gpio45, Gpio46, Gpio47, Gpio48};
use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Gpio8};
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use core::slice;
use log::info;
use super::esp_lcd_config::{OptimizedLcdConfig, LcdClockSpeed};

// Display configuration for T-Display-S3
const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 170;
const DISPLAY_X_OFFSET: u16 = 0;  // No offset needed with proper initialization
const DISPLAY_Y_OFFSET: u16 = 35; // ST7789 offset for 170-pixel height

// Display offsets are handled by esp_lcd_panel_set_gap

pub struct LcdCamDisplay {
    bus_handle: esp_lcd_i80_bus_handle_t,
    panel_handle: esp_lcd_panel_handle_t,
    width: u16,
    height: u16,
    frame_buffer: Vec<u16>,
    double_buffer: Option<Vec<u16>>,
    active_buffer: u8,
    config: OptimizedLcdConfig,
}

impl LcdCamDisplay {
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
    ) -> Result<Self> {
        Self::with_config(d0, d1, d2, d3, d4, d5, d6, d7, wr, dc, cs, rst, OptimizedLcdConfig::default())
    }
    
    pub fn with_config(
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
        config: OptimizedLcdConfig,
    ) -> Result<Self> {
        unsafe {
            info!("Initializing LCD_CAM with ESP-IDF driver...");
            
            // Configure I80 bus
            let bus_config = esp_lcd_i80_bus_config_t {
                dc_gpio_num: dc.pin() as i32,
                wr_gpio_num: wr.pin() as i32,
                clk_src: soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT,
                data_gpio_nums: [
                    d0.pin() as i32,
                    d1.pin() as i32,
                    d2.pin() as i32,
                    d3.pin() as i32,
                    d4.pin() as i32,
                    d5.pin() as i32,
                    d6.pin() as i32,
                    d7.pin() as i32,
                    -1, -1, -1, -1, -1, -1, -1, -1, // Only using 8-bit mode
                ],
                bus_width: 8,
                max_transfer_bytes: config.transfer_size.bytes(DISPLAY_WIDTH as usize),
                __bindgen_anon_1: esp_lcd_i80_bus_config_t__bindgen_ty_1 {
                    psram_trans_align: 64,
                },
                sram_trans_align: 4,
            };
            
            let mut bus_handle: esp_lcd_i80_bus_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_i80_bus(&bus_config, &mut bus_handle);
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to create I80 bus: {:?}", ret));
            }
            info!("I80 bus created successfully");
            
            // Configure panel for I80 interface
            let io_config = esp_lcd_panel_io_i80_config_t {
                cs_gpio_num: cs.pin() as i32,
                pclk_hz: config.clock_speed.as_hz(),
                trans_queue_depth: config.queue_depth,
                dc_levels: esp_lcd_panel_io_i80_config_t__bindgen_ty_1 {
                    _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_1::new_bitfield_1(
                        0, // dc_idle_level
                        0, // dc_cmd_level  
                        0, // dc_dummy_level
                        1, // dc_data_level
                    ),
                    ..Default::default()
                },
                flags: esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
                    _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
                        0, // cs_active_high
                        0, // reverse_color_bits
                        0, // swap_color_bytes
                        0, // pclk_active_neg
                        0, // pclk_idle_low
                    ),
                    ..Default::default()
                },
                on_color_trans_done: None,
                user_ctx: ptr::null_mut(),
                lcd_cmd_bits: 8,
                lcd_param_bits: 8,
            };
            
            let mut io_handle: esp_lcd_panel_io_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_panel_io_i80(bus_handle, &io_config, &mut io_handle);
            if ret != ESP_OK {
                esp_lcd_del_i80_bus(bus_handle);
                return Err(anyhow::anyhow!("Failed to create panel IO: {:?}", ret));
            }
            info!("Panel IO created successfully");
            
            // Create ST7789 panel driver
            let panel_config = esp_lcd_panel_dev_config_t {
                reset_gpio_num: rst.pin() as i32,
                __bindgen_anon_1: esp_lcd_panel_dev_config_t__bindgen_ty_1 {
                    rgb_ele_order: lcd_rgb_element_order_t_LCD_RGB_ELEMENT_ORDER_RGB,
                },
                data_endian: lcd_rgb_data_endian_t_LCD_RGB_DATA_ENDIAN_BIG,
                bits_per_pixel: 16,
                flags: esp_lcd_panel_dev_config_t__bindgen_ty_2 {
                    _bitfield_1: esp_lcd_panel_dev_config_t__bindgen_ty_2::new_bitfield_1(
                        0, // reset_active_high
                    ),
                    ..Default::default()
                },
                vendor_config: ptr::null_mut(),
            };
            
            let mut panel_handle: esp_lcd_panel_handle_t = ptr::null_mut();
            let ret = esp_lcd_new_panel_st7789(io_handle, &panel_config, &mut panel_handle);
            if ret != ESP_OK {
                esp_lcd_panel_io_del(io_handle);
                esp_lcd_del_i80_bus(bus_handle);
                return Err(anyhow::anyhow!("Failed to create ST7789 panel: {:?}", ret));
            }
            info!("ST7789 panel created successfully");
            
            // Initialize the panel
            esp_lcd_panel_reset(panel_handle);
            esp_lcd_panel_init(panel_handle);
            
            // Configure for our specific display
            Self::configure_display(panel_handle)?;
            
            // Turn on display
            esp_lcd_panel_disp_on_off(panel_handle, true);
            
            // Allocate frame buffer
            let frame_buffer = vec![0u16; DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize];
            
            // Allocate double buffer if enabled
            let double_buffer = if config.double_buffer.enabled {
                info!("Allocating double buffer: {} bytes", config.double_buffer.buffer_size);
                Some(vec![0u16; config.double_buffer.buffer_size / 2])
            } else {
                None
            };
            
            info!("LCD initialized with {} clock, {} lines transfer", 
                  config.clock_speed.name(), config.transfer_size.lines());
            
            Ok(Self {
                bus_handle,
                panel_handle,
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                frame_buffer,
                double_buffer,
                active_buffer: 0,
                config,
            })
        }
    }
    
    fn configure_display(panel: esp_lcd_panel_handle_t) -> Result<()> {
        unsafe {
            // The panel driver handles most initialization, but we need to set specific modes
            
            // Set to landscape mode
            esp_lcd_panel_swap_xy(panel, false);
            esp_lcd_panel_mirror(panel, true, false);
            
            // Set gaps for our specific display
            esp_lcd_panel_set_gap(panel, DISPLAY_X_OFFSET as i32, DISPLAY_Y_OFFSET as i32);
            
            info!("Display configuration complete");
            Ok(())
        }
    }
    
    pub fn clear(&mut self, color: u16) -> Result<()> {
        // Fill frame buffer with color
        self.frame_buffer.fill(color);
        
        // Send to display
        self.flush()
    }
    
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }
        
        let idx = (y as usize * self.width as usize) + x as usize;
        self.frame_buffer[idx] = color;
        Ok(())
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        
        for py in y..y_end {
            for px in x..x_end {
                let idx = (py as usize * self.width as usize) + px as usize;
                self.frame_buffer[idx] = color;
            }
        }
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        unsafe {
            // Convert u16 buffer to u8 for transmission
            let byte_slice = slice::from_raw_parts(
                self.frame_buffer.as_ptr() as *const u8,
                self.frame_buffer.len() * 2
            );
            
            // Use hardware-accelerated draw
            let ret = esp_lcd_panel_draw_bitmap(
                self.panel_handle,
                0,                      // x_start
                0,                      // y_start  
                self.width as i32,      // x_end
                self.height as i32,     // y_end
                byte_slice.as_ptr() as *const _
            );
            
            if ret != ESP_OK {
                return Err(anyhow::anyhow!("Failed to draw bitmap: {:?}", ret));
            }
        }
        Ok(())
    }
    
    pub fn flush_region(&mut self, _x: u16, _y: u16, _w: u16, _h: u16) -> Result<()> {
        // For now, flush entire screen - can optimize later
        self.flush()
    }
    
    pub fn width(&self) -> u16 {
        self.width
    }
    
    pub fn height(&self) -> u16 {
        self.height
    }
}

impl Drop for LcdCamDisplay {
    fn drop(&mut self) {
        unsafe {
            esp_lcd_panel_del(self.panel_handle);
            esp_lcd_del_i80_bus(self.bus_handle);
        }
    }
}

// Performance test function
pub fn benchmark_lcd_cam(display: &mut LcdCamDisplay) -> Result<()> {
    use std::time::Instant;
    
    info!("Starting LCD_CAM performance benchmark...");
    
    // Test 1: Clear screen performance
    let start = Instant::now();
    for i in 0..60 {
        let color = if i % 2 == 0 { 0xF800 } else { 0x07E0 }; // Red/Green
        display.clear(color)?;
    }
    let elapsed = start.elapsed();
    let fps = 60.0 / elapsed.as_secs_f32();
    info!("Clear screen: {:.1} FPS", fps);
    
    // Test 2: Full screen update with pattern
    let start = Instant::now();
    for frame in 0..60 {
        // Create gradient pattern
        for y in 0..display.height() {
            for x in 0..display.width() {
                let r = ((x * 31) / display.width()) as u16;
                let g = ((y * 63) / display.height()) as u16;
                let b = ((frame * 31) / 60) as u16;
                let color = (r << 11) | (g << 5) | b;
                display.draw_pixel(x, y, color)?;
            }
        }
        display.flush()?;
    }
    let elapsed = start.elapsed();
    let fps = 60.0 / elapsed.as_secs_f32();
    info!("Pattern draw: {:.1} FPS", fps);
    
    // Test 3: Rectangle fill performance
    let start = Instant::now();
    for i in 0..1000 {
        let x = (i * 17) % (display.width() - 50);
        let y = (i * 23) % (display.height() - 50);
        let color = (i as u16 * 1337) & 0xFFFF;
        display.fill_rect(x, y, 50, 50, color)?;
        if i % 20 == 19 {
            display.flush()?;
        }
    }
    let elapsed = start.elapsed();
    let rects_per_sec = 1000.0 / elapsed.as_secs_f32();
    info!("Rectangle fill: {:.0} rects/sec", rects_per_sec);
    
    info!("LCD_CAM benchmark complete!");
    Ok(())
}