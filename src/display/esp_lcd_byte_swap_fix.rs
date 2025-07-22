use anyhow::Result;
use esp_idf_sys::*;
use log::info;

/// Fix byte swapping issue in ESP LCD configuration
pub unsafe fn apply_byte_swap_fix(
    bus_handle: esp_lcd_i80_bus_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== ESP LCD BYTE SWAP FIX ===");
    
    // The jumbled display is likely due to byte order mismatch
    // ESP32-S3 I80 interface has flags for byte/bit swapping
    
    info!("Current issue: Display shows jumbled output");
    info!("This indicates data is being transmitted but in wrong format");
    
    // Common fixes for jumbled display:
    
    // 1. Try swapping bytes in 16-bit RGB565 data
    info!("Fix 1: Enable swap_color_bytes flag in panel IO config");
    info!("This swaps bytes within each 16-bit color value");
    
    // 2. Try different bit order
    info!("Fix 2: Enable reverse_color_bits flag");
    info!("This reverses bit order within each byte");
    
    // 3. Try different data endianness
    info!("Fix 3: Change data_endian in bus config");
    info!("Switch between BIG and LITTLE endian");
    
    info!("\n=== RECOMMENDED FIX ===");
    info!("In esp_lcd_panel_io_i80_config_t, set:");
    info!("  flags.swap_color_bytes = 1");
    info!("This is the most common fix for jumbled RGB565 display");
    
    Ok(())
}

/// Test different byte order configurations
pub fn get_byte_swap_config() -> esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
    // Return flags with byte swapping enabled
    esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
        _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
            0, // cs_active_high
            0, // reverse_color_bits - try 1 if still jumbled
            1, // swap_color_bytes - THIS IS THE KEY FIX!
            0, // pclk_active_neg
            0, // pclk_idle_low
        ),
        ..Default::default()
    }
}

/// Alternative configurations to try
pub fn get_alternative_configs() -> Vec<(&'static str, esp_lcd_panel_io_i80_config_t__bindgen_ty_2)> {
    vec![
        ("Swap bytes only", esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
            _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
                0, 0, 1, 0, 0
            ),
            ..Default::default()
        }),
        ("Reverse bits only", esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
            _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
                0, 1, 0, 0, 0
            ),
            ..Default::default()
        }),
        ("Swap bytes + reverse bits", esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
            _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
                0, 1, 1, 0, 0
            ),
            ..Default::default()
        }),
        ("No swapping (original)", esp_lcd_panel_io_i80_config_t__bindgen_ty_2 {
            _bitfield_1: esp_lcd_panel_io_i80_config_t__bindgen_ty_2::new_bitfield_1(
                0, 0, 0, 0, 0
            ),
            ..Default::default()
        }),
    ]
}