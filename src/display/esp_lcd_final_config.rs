/// Final working ESP LCD configuration for T-Display-S3
/// This configuration has been tested and verified to work correctly
use esp_idf_sys::*;

pub struct WorkingLcdConfig;

impl WorkingLcdConfig {
    /// Get the verified working I80 bus configuration
    pub fn get_bus_config(
        dc_pin: i32,
        wr_pin: i32,
        data_pins: [i32; 8],
    ) -> esp_lcd_i80_bus_config_t {
        esp_lcd_i80_bus_config_t {
            dc_gpio_num: dc_pin,
            wr_gpio_num: wr_pin,
            clk_src: soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT,
            data_gpio_nums: [
                data_pins[0], data_pins[1], data_pins[2], data_pins[3],
                data_pins[4], data_pins[5], data_pins[6], data_pins[7],
                -1, -1, -1, -1, -1, -1, -1, -1, // Only 8-bit mode
            ],
            bus_width: 8,
            max_transfer_bytes: 384, // 12-byte aligned, cache aligned
            __bindgen_anon_1: esp_lcd_i80_bus_config_t__bindgen_ty_1 {
                psram_trans_align: 64,
            },
            sram_trans_align: 12, // Critical for 6-pixel alignment fix
        }
    }
    
    /// Get the verified working panel IO configuration
    pub fn get_io_config(cs_pin: i32) -> esp_lcd_panel_io_i80_config_t {
        esp_lcd_panel_io_i80_config_t {
            cs_gpio_num: cs_pin,
            pclk_hz: 5_000_000, // 5 MHz - verified working speed
            trans_queue_depth: 1, // Synchronous transfers
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
                    1, // swap_color_bytes - Required for RGB565
                    0, // pclk_active_neg
                    0, // pclk_idle_low
                ),
                ..Default::default()
            },
            on_color_trans_done: None,
            user_ctx: core::ptr::null_mut(),
            lcd_cmd_bits: 8,
            lcd_param_bits: 8,
        }
    }
    
    /// Get the verified working panel configuration
    pub fn get_panel_config(rst_pin: i32) -> esp_lcd_panel_dev_config_t {
        esp_lcd_panel_dev_config_t {
            reset_gpio_num: rst_pin,
            __bindgen_anon_1: esp_lcd_panel_dev_config_t__bindgen_ty_1 {
                color_space: lcd_rgb_element_order_t_LCD_RGB_ELEMENT_ORDER_RGB,
            },
            data_endian: lcd_rgb_data_endian_t_LCD_RGB_DATA_ENDIAN_BIG,
            bits_per_pixel: 16,
            flags: esp_lcd_panel_dev_config_t__bindgen_ty_2 {
                _bitfield_1: esp_lcd_panel_dev_config_t__bindgen_ty_2::new_bitfield_1(
                    0, // reset_active_high
                ),
                ..Default::default()
            },
            vendor_config: core::ptr::null_mut(),
        }
    }
    
    /// Summary of the working configuration
    pub fn print_config_summary() {
        log::info!("=== ESP LCD Working Configuration ===");
        log::info!("Clock Speed: 5 MHz");
        log::info!("Bus Width: 8-bit");
        log::info!("Transfer Size: 384 bytes (12-byte aligned)");
        log::info!("Queue Depth: 1 (synchronous)");
        log::info!("SRAM Alignment: 12 bytes");
        log::info!("PSRAM Alignment: 64 bytes");
        log::info!("Color Byte Swap: Enabled");
        log::info!("Display Gap: X=0, Y=35");
        log::info!("Orientation: Landscape (swap_xy=true, mirror_y=true)");
        log::info!("=================================");
    }
}