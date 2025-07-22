/// LCD_CAM implementation using ESP-IDF LCD driver
/// This provides hardware-accelerated display output via the LCD_CAM peripheral
use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::{Pin, Gpio39, Gpio40, Gpio41, Gpio42, Gpio45, Gpio46, Gpio47, Gpio48};
use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Gpio8};
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, Output};
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use core::slice;
use log::{info, error};
use super::esp_lcd_config::{OptimizedLcdConfig, LcdClockSpeed};
use super::debug_trace::{traced_lcd_panel_io_tx_param, traced_lcd_panel_io_tx_color};
use super::error_diagnostics::{log_display_error, log_i80_config, check_display_health, log_memory_diagnostics};

// Display configuration for T-Display-S3
// The visible area is 170x320, but positioned at row 35 in the ST7789's GRAM
const PANEL_WIDTH: u16 = 170;   // Visible width (portrait mode)
const PANEL_HEIGHT: u16 = 320;  // Visible height (portrait mode)
const X_GAP: i32 = 0;           // Column offset in GRAM
const Y_GAP: i32 = 35;          // Row offset in GRAM (critical for T-Display-S3!)

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
            info!("=== ESP LCD Initialization Debug Start ===");
            info!("Display dimensions: {}x{} (portrait)", PANEL_WIDTH, PANEL_HEIGHT);
            info!("Display offsets: X_GAP={}, Y_GAP={}", X_GAP, Y_GAP);
            
            // Add timing measurement
            let start_time = std::time::Instant::now();
            
            info!("Pin configuration:");
            info!("  Data: D0-D7 = GPIO 39,40,41,42,45,46,47,48");
            info!("  Control: WR={}, DC={}, CS={}, RST={}", wr.pin(), dc.pin(), cs.pin(), rst.pin());
            
            // Verify pin states
            info!("Verifying GPIO initial states...");
            super::gpio_debug::verify_gpio_states();
            
            // Configure I80 bus
            let mut bus_config = esp_lcd_i80_bus_config_t {
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
                // Fix: Use max of width/height to support both orientations
                max_transfer_bytes: config.transfer_size.bytes(PANEL_WIDTH.max(PANEL_HEIGHT) as usize),
                __bindgen_anon_1: esp_lcd_i80_bus_config_t__bindgen_ty_1 {
                    psram_trans_align: 64,
                },
                sram_trans_align: 4,
            };
            
            // Configure panel for I80 interface
            let mut io_config = esp_lcd_panel_io_i80_config_t {
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
                        0, // swap_color_bytes - Let ST7789 handle byte swapping
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
            
            // Apply 6-block pattern fix BEFORE creating the bus
            info!("Detected 6-block pattern issue - applying targeted fix...");
            super::esp_lcd_6block_fix::apply_6block_fix(&mut bus_config, &mut io_config)?;
            
            // Also apply anti-flicker fixes
            info!("Applying anti-flicker configuration...");
            super::esp_lcd_flicker_fix::apply_flicker_fix(&mut bus_config, &mut io_config)?;
            
            // Log configuration and memory state
            log_i80_config(&bus_config, &io_config);
            log_memory_diagnostics();
            
            // Now create the bus with fixed configuration
            let mut bus_handle: esp_lcd_i80_bus_handle_t = ptr::null_mut();
            info!("Creating I80 bus with fixed configuration...");
            info!("  Max transfer bytes: {}", bus_config.max_transfer_bytes);
            info!("  Bus width: 8-bit");
            info!("  PSRAM align: {}, SRAM align: {}", 
                  bus_config.__bindgen_anon_1.psram_trans_align,
                  bus_config.sram_trans_align);
            
            let ret = esp_lcd_new_i80_bus(&bus_config, &mut bus_handle);
            if ret != ESP_OK {
                log_display_error("esp_lcd_new_i80_bus", "Failed to create I80 bus", ret);
                error!("Hint: ESP_ERR_INVALID_ARG=-258, ESP_ERR_NO_MEM=-257");
                return Err(anyhow::anyhow!("Failed to create I80 bus: error code {}", ret));
            }
            info!("✓ I80 bus created successfully with 6-block fix");
            info!("  Time elapsed: {:?}", start_time.elapsed());
            
            // Check GPIO39 (D0) for stuck HIGH condition
            info!("Checking GPIO39 (D0) for stuck HIGH condition...");
            if let Err(e) = super::gpio39_diagnostic::test_gpio39_stuck_high() {
                error!("GPIO39 diagnostic failed: {}", e);
            }
            
            // Check all data pins
            if let Err(e) = super::gpio39_diagnostic::check_all_data_pins() {
                error!("Data pin check failed: {}", e);
            }
            
            let mut io_handle: esp_lcd_panel_io_handle_t = ptr::null_mut();
            info!("Creating panel IO with CS={}, {}Hz clock (fixed)", cs.pin(), io_config.pclk_hz);
            let ret = esp_lcd_new_panel_io_i80(bus_handle, &io_config, &mut io_handle);
            if ret != ESP_OK {
                log_display_error("esp_lcd_new_panel_io_i80", "Failed to create panel IO", ret);
                esp_lcd_del_i80_bus(bus_handle);
                return Err(anyhow::anyhow!("Failed to create panel IO: {:?}", ret));
            }
            info!("✓ Panel IO created successfully");
            info!("  Time elapsed: {:?}", start_time.elapsed());
            
            // Create ST7789 panel driver
            let panel_config = esp_lcd_panel_dev_config_t {
                reset_gpio_num: rst.pin() as i32,
                __bindgen_anon_1: esp_lcd_panel_dev_config_t__bindgen_ty_1 {
                    color_space: lcd_rgb_element_order_t_LCD_RGB_ELEMENT_ORDER_RGB, // T-Display-S3 uses RGB order
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
            info!("Creating ST7789 panel driver...");
            info!("  Reset pin: GPIO{}", rst.pin());
            info!("  Color space: RGB");
            info!("  Bits per pixel: 16");
            let ret = esp_lcd_new_panel_st7789(io_handle, &panel_config, &mut panel_handle);
            if ret != ESP_OK {
                log_display_error("esp_lcd_new_panel_st7789", "Failed to create ST7789 panel", ret);
                esp_lcd_panel_io_del(io_handle);
                esp_lcd_del_i80_bus(bus_handle);
                return Err(anyhow::anyhow!("Failed to create ST7789 panel: {:?}", ret));
            }
            info!("✓ ST7789 panel created successfully");
            info!("  Time elapsed: {:?}", start_time.elapsed());
            
            // Initialize the panel
            info!("Resetting panel...");
            let ret = esp_lcd_panel_reset(panel_handle);
            if ret != ESP_OK {
                error!("Panel reset failed: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Panel reset complete");
            }
            
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            
            info!("Initializing panel...");
            let ret = esp_lcd_panel_init(panel_handle);
            if ret != ESP_OK {
                error!("Panel init failed: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Panel init complete");
            }
            info!("  Time elapsed: {:?}", start_time.elapsed());
            
            // Add delay to ensure init is complete
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            
            // Configure the panel for T-Display-S3 specifics
            // CRITICAL: These must be called AFTER init but BEFORE any drawing
            info!("Configuring panel for T-Display-S3...");
            
            // Add delay after init
            info!("Adding 200ms delay after panel init for stabilization...");
            esp_idf_hal::delay::FreeRtos::delay_ms(200);
            
            let ret = esp_lcd_panel_set_gap(panel_handle, X_GAP, Y_GAP);
            if ret != ESP_OK {
                error!("Failed to set gap: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Set display gap: X={}, Y={} (maps visible window)", X_GAP, Y_GAP);
            }
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            
            let ret = esp_lcd_panel_swap_xy(panel_handle, true);
            if ret != ESP_OK {
                error!("Failed to swap XY: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Swapped X/Y for landscape orientation");
            }
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            
            let ret = esp_lcd_panel_mirror(panel_handle, false, true);
            if ret != ESP_OK {
                error!("Failed to mirror: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Mirrored Y axis for correct orientation");
            }
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            
            let ret = esp_lcd_panel_invert_color(panel_handle, true);
            if ret != ESP_OK {
                error!("Failed to invert colors: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Inverted colors (required for ST7789)");
            }
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            
            // Add explicit initialization commands as fallback
            // Some ST7789 batches need these extra commands
            info!("Sending additional init commands with delays...");
            traced_lcd_panel_io_tx_param(io_handle, 0x21, ptr::null(), 0); // INVON
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            traced_lcd_panel_io_tx_param(io_handle, 0x13, ptr::null(), 0); // NORON (Normal display on)
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            
            // Explicitly set MADCTL for landscape with byte swapping enabled
            // MX=1, MV=1, BGR=1, ML=1 (byte swap) = 0x60 | 0x08 = 0x68
            let madctl_data: [u8; 1] = [0x68]; // Landscape + BGR + byte-swap
            traced_lcd_panel_io_tx_param(io_handle, 0x36, madctl_data.as_ptr() as *const _, 1);
            
            // Clear the display memory by setting a window and filling with black
            // CASET - Column address set (0-169 after rotation)
            let caset_data: [u8; 4] = [0, 0, 0, 169];
            traced_lcd_panel_io_tx_param(io_handle, 0x2A, caset_data.as_ptr() as *const _, 4);
            
            // RASET - Row address set (35 to 204 = 170 pixels with 35 offset)
            // Fixed calculation: 35 + 169 = 204 (not 354!)
            let raset_data: [u8; 4] = [0, 35, 0, 204]; // 35 to 204 (0x23 to 0xCC)
            traced_lcd_panel_io_tx_param(io_handle, 0x2B, raset_data.as_ptr() as *const _, 4);
            
            // Turn on display with delay before and after
            info!("Waiting 100ms before turning display on...");
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            
            let ret = esp_lcd_panel_disp_on_off(panel_handle, true);
            if ret != ESP_OK {
                error!("Failed to turn on display: error {} (0x{:X})", ret, ret);
            } else {
                info!("✓ Display turned on");
            }
            
            info!("Waiting 100ms after display on...");
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            
            // Print command trace summary
            super::debug_trace::print_command_summary();
            
            #[cfg(feature = "display-tests")]
            {
                info!("=== Running Display Tests (disable with --no-default-features) ===");
                
                // Extend watchdog timeout for tests
                esp_task_wdt_deinit();
                esp_task_wdt_init(15000, false); // 15 second timeout for tests
                esp_task_wdt_add(ptr::null_mut());
                
                // Test the 6-block fix
                info!("=== Testing 6-Block Fix ===");
                
                // Analyze the pattern first
                super::esp_lcd_6block_fix::analyze_6block_pattern(panel_handle, io_handle)?;
                esp_idf_hal::delay::FreeRtos::delay_ms(2000);
                esp_task_wdt_reset();
                
                // Test different clock speeds
                super::esp_lcd_6block_fix::test_clock_speeds(panel_handle)?;
                esp_idf_hal::delay::FreeRtos::delay_ms(2000);
                esp_task_wdt_reset();
                
                info!("=== 6-Block Fix Test Complete ===");
                info!("If you still see blocky output:");
                info!("- The pattern analysis will show which pixels are visible");
                info!("- Try even slower clock speeds");
                info!("- Check the alignment test results");
            }
            
            #[cfg(not(feature = "display-tests"))]
            {
                info!("Display tests disabled. Enable with: --features display-tests");
            }
            
            #[cfg(feature = "display-tests")]
            {
                // Try a simple pixel test
                info!("=== Starting pixel test ===");
                info!("Drawing test pattern directly...");
                
                // Try to draw a simple red rectangle
                let test_color: [u16; 100] = [0xF800; 100]; // Red pixels
                if let Err(e) = super::esp_lcd_chunk_wrapper::safe_draw_bitmap(
                    panel_handle,
                    0, 0,     // Start position
                    10, 10,   // End position
                    test_color.as_ptr() as *const _
                ) {
                    error!("Failed to draw test bitmap: {}", e);
                } else {
                    info!("✓ Test bitmap drawn at (0,0) to (10,10)");
                }
                esp_task_wdt_reset();
                
                // Run comprehensive test
                info!("Running comprehensive display test...");
                if let Err(e) = super::comprehensive_test::run_comprehensive_test(panel_handle) {
                    error!("Comprehensive test failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Run debug test to try raw commands
                info!("Running debug test with raw commands...");
                if let Err(e) = super::esp_lcd_debug_test::debug_esp_lcd_raw_commands(io_handle) {
                    error!("Debug test failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Run MADCTL configuration test
                info!("Running MADCTL configuration test...");
                if let Err(e) = super::esp_lcd_madctl_test::test_madctl_configurations(panel_handle, io_handle) {
                    error!("MADCTL test failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Run clock speed test
                info!("Running clock speed test...");
                if let Err(e) = super::esp_lcd_clock_test::test_clock_speeds(panel_handle) {
                    error!("Clock speed test failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Run direct I80 test
                info!("Running direct I80 test...");
                if let Err(e) = super::esp_lcd_direct_test::test_direct_i80(io_handle) {
                    error!("Direct I80 test failed: {}", e);
                }
                
                // Compare init sequences
                super::esp_lcd_direct_test::compare_init_sequences();
                
                // Run D0 test pattern for vertical striping diagnosis
                info!("Running D0 test pattern for vertical striping diagnosis...");
                if let Err(e) = super::d0_test_pattern::draw_d0_test_pattern(panel_handle) {
                    error!("D0 test pattern failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Analyze D0 symptoms
                if let Err(e) = super::d0_test_pattern::analyze_d0_symptoms(panel_handle) {
                    error!("D0 symptom analysis failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Run reset sequence test
                info!("Running reset sequence test...");
                if let Err(e) = super::esp_lcd_clock_test::test_reset_sequence(rst.pin() as i32, panel_handle) {
                    error!("Reset sequence test failed: {}", e);
                }
                esp_task_wdt_reset();
                
                // Restore normal watchdog timeout
                esp_task_wdt_deinit();
                esp_task_wdt_init(5000, false); // Back to 5 second timeout
                esp_task_wdt_add(ptr::null_mut());
            }
            
            info!("=== ESP LCD Initialization Complete ===");
            info!("Total initialization time: {:?}", start_time.elapsed());
            
            // Allocate frame buffer for the actual visible area
            // Note: After swap_xy, width and height are swapped for landscape
            let display_width = PANEL_HEIGHT;  // 320 in landscape
            let display_height = PANEL_WIDTH;  // 170 in landscape
            let frame_buffer = vec![0u16; display_width as usize * display_height as usize];
            
            // Allocate double buffer if enabled
            let double_buffer = if config.double_buffer.enabled {
                info!("Allocating double buffer: {} bytes", config.double_buffer.buffer_size);
                Some(vec![0u16; config.double_buffer.buffer_size / 2])
            } else {
                None
            };
            
            info!("LCD initialized: {}x{} landscape, {} MHz clock, {} lines transfer", 
                  display_width, display_height,
                  config.clock_speed.as_hz() / 1_000_000,
                  config.transfer_size.lines());
            
            let mut display = Self {
                bus_handle,
                panel_handle,
                width: display_width,
                height: display_height,
                frame_buffer,
                double_buffer,
                active_buffer: 0,
                config,
            };
            
            // Draw a simple test pattern to verify display is working
            info!("Drawing initial test pattern to verify display...");
            
            // Fill with white to test
            display.frame_buffer.fill(0xFFFF);
            display.flush()?;
            info!("Test: Display should be WHITE");
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
            
            // Then fill with red
            display.frame_buffer.fill(0xF800);
            display.flush()?;
            info!("Test: Display should be RED");
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
            
            // Finally clear to black
            display.frame_buffer.fill(0x0000);
            display.flush()?;
            info!("Test: Display should be BLACK");
            
            info!("Test pattern complete - display should have flashed white->red->black");
            
            // Perform health check
            if let Err(e) = check_display_health(panel_handle) {
                error!("Display health check failed: {}", e);
                error!("Display may not be responding correctly!");
            } else {
                info!("Display health check passed - communication verified");
            }
            
            Ok(display)
        }
    }
    
    // Remove configure_display - we handle everything in new() now
    
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
            // Log flush operation
            log::debug!("ESP LCD flush: frame buffer size = {} pixels, {} bytes", 
                self.frame_buffer.len(), self.frame_buffer.len() * 2);
            
            // Check if frame buffer has any non-black content
            let has_content = self.frame_buffer.iter().any(|&pixel| pixel != 0x0000);
            log::debug!("Frame buffer has content: {}", has_content);
            
            // Convert u16 buffer to u8 for transmission
            let byte_slice = slice::from_raw_parts(
                self.frame_buffer.as_ptr() as *const u8,
                self.frame_buffer.len() * 2
            );
            
            // Use hardware-accelerated draw with safe chunking
            // IMPORTANT: Coordinates are in landscape mode after swap_xy
            // Drawing from (0,0) to (width,height) which is (0,0) to (320,170)
            log::debug!("Calling safe_draw_bitmap with dimensions: {}x{}", self.width, self.height);
            
            match super::esp_lcd_chunk_wrapper::safe_draw_bitmap(
                self.panel_handle,
                0,                      // x_start  
                0,                      // y_start
                self.width as i32,      // x_end (320)
                self.height as i32,     // y_end (170)
                byte_slice.as_ptr() as *const _
            ) {
                Ok(()) => {
                    log::debug!("safe_draw_bitmap completed successfully");
                }
                Err(e) => {
                    error!("safe_draw_bitmap failed: {:?}", e);
                    // Try to analyze the error
                    super::error_diagnostics::analyze_crash_pattern();
                    return Err(e);
                }
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