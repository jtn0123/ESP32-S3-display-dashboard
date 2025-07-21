/// Minimal esp_lcd test based on working template configuration
/// This test uses the exact configuration from hiruna/esp-idf-t-display-s3

use anyhow::Result;
use esp_idf_sys::*;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, Output, Input, Pull};
use esp_idf_hal::delay::FreeRtos;
use core::ptr;
use log::info;

// Exact configuration from working template
const LCD_H_RES: u32 = 320;
const LCD_V_RES: u32 = 170;
const LCD_PIXEL_CLOCK_HZ: u32 = 17 * 1000 * 1000; // 17 MHz from template
const LCD_CMD_BITS: i32 = 8;
const LCD_PARAM_BITS: i32 = 8;
const LCD_I80_BUS_WIDTH: usize = 8;

pub fn test_esp_lcd_minimal() -> Result<()> {
    info!("Starting minimal esp_lcd test based on working template...");
    
    unsafe {
        // Step 0: Initialize LCD power and backlight pins
        info!("Initializing LCD power and backlight pins...");
        
        // Configure LCD power pin (GPIO15) as output and set HIGH
        let lcd_power_config = gpio_config_t {
            pin_bit_mask: 1u64 << 15,
            mode: gpio_mode_t_GPIO_MODE_OUTPUT,
            pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
            pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
        };
        gpio_config(&lcd_power_config);
        gpio_set_level(15, 1); // Turn on LCD power
        
        // Configure backlight pin (GPIO38) as output and set HIGH
        let backlight_config = gpio_config_t {
            pin_bit_mask: 1u64 << 38,
            mode: gpio_mode_t_GPIO_MODE_OUTPUT,
            pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
            pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
        };
        gpio_config(&backlight_config);
        gpio_set_level(38, 1); // Turn on backlight
        
        // Wait for power to stabilize
        FreeRtos::delay_ms(100);
        
        // Step 1: Configure LCD_RD pin (GPIO9) as input with pullup - from template
        info!("Configuring LCD RD GPIO...");
        let lcd_rd_gpio_config = gpio_config_t {
            pin_bit_mask: 1u64 << 9,
            mode: gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
        };
        let ret = gpio_config(&lcd_rd_gpio_config);
        if ret != ESP_OK {
            return Err(anyhow::anyhow!("Failed to configure RD pin: {:?}", ret));
        }
        
        // Step 2: Configure I80 bus exactly as template
        info!("Initializing Intel 8080 bus...");
        let mut bus_config: esp_lcd_i80_bus_config_t = Default::default();
        bus_config.dc_gpio_num = 7;  // LCD_DC
        bus_config.wr_gpio_num = 8;  // LCD_WR (PCLK)
        bus_config.clk_src = soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT;
        bus_config.data_gpio_nums = [
            39, 40, 41, 42, 45, 46, 47, 48, // D0-D7
            -1, -1, -1, -1, -1, -1, -1, -1, // Only 8-bit mode
        ];
        bus_config.bus_width = LCD_I80_BUS_WIDTH;
        bus_config.max_transfer_bytes = (LCD_H_RES * LCD_V_RES * 2) as usize; // Full screen: 2 bytes per pixel (RGB565)
        bus_config.sram_trans_align = 4;
        
        let mut i80_bus: esp_lcd_i80_bus_handle_t = ptr::null_mut();
        let ret = esp_lcd_new_i80_bus(&bus_config, &mut i80_bus);
        if ret != ESP_OK {
            return Err(anyhow::anyhow!("Failed to create I80 bus: {:?}", ret));
        }
        info!("I80 bus created successfully");
        
        // Step 3: Configure panel IO
        info!("Creating panel IO...");
        let mut io_config: esp_lcd_panel_io_i80_config_t = Default::default();
        io_config.cs_gpio_num = 6;  // LCD_CS
        io_config.pclk_hz = LCD_PIXEL_CLOCK_HZ;
        io_config.trans_queue_depth = 20; // From template
        io_config.on_color_trans_done = None;
        io_config.user_ctx = ptr::null_mut();
        io_config.lcd_cmd_bits = LCD_CMD_BITS;
        io_config.lcd_param_bits = LCD_PARAM_BITS;
        
        // Set DC levels using bitfield - dc_data_level = 1, others = 0
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
        
        // Step 4: Create ST7789 panel
        info!("Initializing ST7789 LCD Driver...");
        let mut panel_config: esp_lcd_panel_dev_config_t = Default::default();
        panel_config.reset_gpio_num = 5;  // LCD_RST
        panel_config.bits_per_pixel = 16;
        panel_config.vendor_config = ptr::null_mut();
        
        let mut panel_handle: esp_lcd_panel_handle_t = ptr::null_mut();
        let ret = esp_lcd_new_panel_st7789(io_handle, &panel_config, &mut panel_handle);
        if ret != ESP_OK {
            return Err(anyhow::anyhow!("Failed to create ST7789 panel: {:?}", ret));
        }
        info!("ST7789 panel created successfully");
        
        // Step 5: Initialize panel (following template sequence)
        info!("Resetting panel...");
        esp_lcd_panel_reset(panel_handle);
        FreeRtos::delay_ms(100);
        
        info!("Initializing panel...");
        esp_lcd_panel_init(panel_handle);
        FreeRtos::delay_ms(100);
        
        // Configure display orientation exactly as template
        info!("Configuring display orientation...");
        esp_lcd_panel_invert_color(panel_handle, true);
        esp_lcd_panel_swap_xy(panel_handle, true);
        esp_lcd_panel_mirror(panel_handle, false, true);
        esp_lcd_panel_set_gap(panel_handle, 0, 35); // Y gap for 170-pixel display
        
        info!("Turning display on...");
        esp_lcd_panel_disp_on_off(panel_handle, true);
        
        // Test: Fill screen with different colors to verify it's working
        info!("Testing display with color patterns...");
        
        // Test 1: Fill screen with RED
        info!("Drawing RED screen...");
        let red_data = vec![0xF800u16; (LCD_H_RES * LCD_V_RES) as usize]; // Red
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            LCD_H_RES as i32, LCD_V_RES as i32,
            red_data.as_ptr() as *const _,
        );
        FreeRtos::delay_ms(1000);
        esp_idf_sys::esp_task_wdt_reset();
        
        // Test 2: Fill screen with GREEN
        info!("Drawing GREEN screen...");
        let green_data = vec![0x07E0u16; (LCD_H_RES * LCD_V_RES) as usize]; // Green
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            LCD_H_RES as i32, LCD_V_RES as i32,
            green_data.as_ptr() as *const _,
        );
        FreeRtos::delay_ms(1000);
        esp_idf_sys::esp_task_wdt_reset();
        
        // Test 3: Fill screen with BLUE
        info!("Drawing BLUE screen...");
        let blue_data = vec![0x001Fu16; (LCD_H_RES * LCD_V_RES) as usize]; // Blue
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            LCD_H_RES as i32, LCD_V_RES as i32,
            blue_data.as_ptr() as *const _,
        );
        FreeRtos::delay_ms(1000);
        esp_idf_sys::esp_task_wdt_reset();
        
        // Test 4: Draw a test pattern with stripes
        info!("Drawing test pattern with stripes...");
        let mut pattern_data = vec![0x0000u16; (LCD_H_RES * LCD_V_RES) as usize];
        for y in 0..LCD_V_RES as usize {
            for x in 0..LCD_H_RES as usize {
                let idx = y * LCD_H_RES as usize + x;
                // Create vertical stripes
                pattern_data[idx] = match x % 30 {
                    0..10 => 0xF800,  // Red stripe
                    10..20 => 0x07E0, // Green stripe
                    _ => 0x001F,      // Blue stripe
                };
            }
        }
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0,
            LCD_H_RES as i32, LCD_V_RES as i32,
            pattern_data.as_ptr() as *const _,
        );
        FreeRtos::delay_ms(2000);
        esp_idf_sys::esp_task_wdt_reset();
        
        info!("ESP_LCD test pattern complete! You should have seen:");
        info!("1. RED screen for 1 second");
        info!("2. GREEN screen for 1 second");
        info!("3. BLUE screen for 1 second");
        info!("4. Vertical RGB stripes for 2 seconds");
        
        info!("Cleaning up esp_lcd resources...");
        
        // Cleanup
        esp_lcd_panel_del(panel_handle);
        esp_lcd_panel_io_del(io_handle);
        esp_lcd_del_i80_bus(i80_bus);
        
        info!("ESP_LCD test finished successfully!");
    }
    
    Ok(())
}