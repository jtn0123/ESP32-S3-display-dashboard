/// Low-level LCD_CAM peripheral access for ESP32-S3
/// Based on ESP32-S3 TRM Chapter 35: LCD Controller
use core::marker::PhantomData;
use esp_idf_sys::*;

// LCD_CAM peripheral base address
const LCD_CAM_BASE: u32 = 0x6004_1000;

// Register offsets
const LCD_CAM_LCD_CLOCK_REG: u32 = LCD_CAM_BASE + 0x0;
const LCD_CAM_LCD_USER_REG: u32 = LCD_CAM_BASE + 0x4;
const LCD_CAM_LCD_MISC_REG: u32 = LCD_CAM_BASE + 0x8;
const LCD_CAM_LCD_CTRL_REG: u32 = LCD_CAM_BASE + 0xC;
const LCD_CAM_LCD_CTRL1_REG: u32 = LCD_CAM_BASE + 0x10;
const LCD_CAM_LCD_CTRL2_REG: u32 = LCD_CAM_BASE + 0x14;
const LCD_CAM_LCD_CMD_VAL_REG: u32 = LCD_CAM_BASE + 0x18;
const LCD_CAM_LCD_DLY_MODE_REG: u32 = LCD_CAM_BASE + 0x30;
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = LCD_CAM_BASE + 0x34;

// System registers for clock/reset control
const SYSTEM_PERIP_CLK_EN1_REG: u32 = 0x600C_0024;
const SYSTEM_PERIP_RST_EN1_REG: u32 = 0x600C_0028;

// Bit definitions for LCD_CAM_LCD_CLOCK_REG
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_CLK_EQU_SYSCLK: u32 = 1 << 27;
const LCD_CLKM_DIV_NUM_SHIFT: u32 = 0;
const LCD_CLKM_DIV_NUM_MASK: u32 = 0xFF;

// Bit definitions for LCD_CAM_LCD_USER_REG  
const LCD_RESET: u32 = 1 << 28;
const LCD_START: u32 = 1 << 27;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_BIT_ORDER: u32 = 1 << 22;
const LCD_BYTE_ORDER: u32 = 1 << 21;
const LCD_2BYTE_EN: u32 = 1 << 20;

// Bit definitions for LCD_CAM_LCD_CTRL_REG
const LCD_RGB_MODE_EN: u32 = 1 << 31;

// Bit definitions for SYSTEM registers
const LCD_CAM_CLK_EN: u32 = 1 << 31;
const LCD_CAM_RST: u32 = 1 << 31;

pub struct LcdCam {
    _private: PhantomData<*const ()>,
}

impl LcdCam {
    /// Initialize LCD_CAM peripheral
    pub unsafe fn new() -> Self {
        // Enable peripheral clock
        let perip_clk_en1 = SYSTEM_PERIP_CLK_EN1_REG as *mut u32;
        let current = perip_clk_en1.read_volatile();
        perip_clk_en1.write_volatile(current | LCD_CAM_CLK_EN);
        
        // Clear reset
        let perip_rst_en1 = SYSTEM_PERIP_RST_EN1_REG as *mut u32;
        let current = perip_rst_en1.read_volatile();
        perip_rst_en1.write_volatile(current & !LCD_CAM_RST);
        
        // Small delay for clock to stabilize
        esp_rom_delay_us(100);
        
        Self { _private: PhantomData }
    }
    
    /// Reset LCD controller
    pub unsafe fn reset(&mut self) {
        let lcd_user_reg = LCD_CAM_LCD_USER_REG as *mut u32;
        let current = lcd_user_reg.read_volatile();
        
        // Set reset bit
        lcd_user_reg.write_volatile(current | LCD_RESET);
        esp_rom_delay_us(10);
        
        // Clear reset bit
        lcd_user_reg.write_volatile(current & !LCD_RESET);
        esp_rom_delay_us(100);
    }
    
    /// Configure for 8-bit parallel output (i8080 mode)
    pub unsafe fn configure_i8080_8bit(&mut self, freq_hz: u32) {
        // Calculate clock divider
        // LCD clock = APB_CLK / (div_num + 1)
        // APB_CLK is typically 80MHz on ESP32-S3
        const APB_FREQ: u32 = 80_000_000;
        let div_num = (APB_FREQ / freq_hz).saturating_sub(1).min(255) as u32;
        
        // Configure clock
        let lcd_clock_reg = LCD_CAM_LCD_CLOCK_REG as *mut u32;
        lcd_clock_reg.write_volatile(
            LCD_CLK_EN | 
            (div_num << LCD_CLKM_DIV_NUM_SHIFT)
        );
        
        // Configure user register for 8-bit mode
        let lcd_user_reg = LCD_CAM_LCD_USER_REG as *mut u32;
        lcd_user_reg.write_volatile(
            LCD_DOUT |        // Output mode
            LCD_8BITS_ORDER | // 8-bit mode
            0                 // MSB first, normal byte order
        );
        
        // Disable RGB mode (use i8080 mode)
        let lcd_ctrl_reg = LCD_CAM_LCD_CTRL_REG as *mut u32;
        let current = lcd_ctrl_reg.read_volatile();
        lcd_ctrl_reg.write_volatile(current & !LCD_RGB_MODE_EN);
    }
    
    /// Configure timing delays
    pub unsafe fn configure_timing(
        &mut self,
        dc_setup_cycles: u8,
        dc_hold_cycles: u8,
        cs_setup_cycles: u8,
        cs_hold_cycles: u8,
    ) {
        let lcd_dly_mode_reg = LCD_CAM_LCD_DLY_MODE_REG as *mut u32;
        
        // Pack timing values into register
        // Exact bit positions would need to be verified from TRM
        let timing_val = ((cs_hold_cycles as u32) << 24) |
                        ((cs_setup_cycles as u32) << 16) |
                        ((dc_hold_cycles as u32) << 8) |
                        (dc_setup_cycles as u32);
                        
        lcd_dly_mode_reg.write_volatile(timing_val);
    }
    
    /// Start a transfer
    pub unsafe fn start_transfer(&mut self) {
        let lcd_user_reg = LCD_CAM_LCD_USER_REG as *mut u32;
        let current = lcd_user_reg.read_volatile();
        lcd_user_reg.write_volatile(current | LCD_START);
    }
    
    /// Check if transfer is complete
    pub unsafe fn is_idle(&self) -> bool {
        let lcd_user_reg = LCD_CAM_LCD_USER_REG as *mut u32;
        (lcd_user_reg.read_volatile() & LCD_START) == 0
    }
}

/// GPIO Matrix configuration for LCD_CAM pins
pub unsafe fn configure_lcd_cam_pins(
    d0: u8, d1: u8, d2: u8, d3: u8,
    d4: u8, d5: u8, d6: u8, d7: u8,
    pclk: u8, dc: u8, cs: u8,
) {
    // LCD data output signals in GPIO matrix
    const LCD_DATA_OUT0_IDX: u32 = 133; // From esp-idf bindings
    
    // Configure data pins
    let data_pins = [d0, d1, d2, d3, d4, d5, d6, d7];
    for (i, &pin) in data_pins.iter().enumerate() {
        esp_rom_gpio_connect_out_signal(
            pin as u32,
            LCD_DATA_OUT0_IDX + i as u32,
            false,
            false
        );
    }
    
    // Configure control pins
    const LCD_PCLK_IDX: u32 = 154;    // From esp-idf bindings
    const LCD_DC_IDX: u32 = 153;      // From esp-idf bindings
    const LCD_CS_IDX: u32 = 132;      // From esp-idf bindings
    
    esp_rom_gpio_connect_out_signal(pclk as u32, LCD_PCLK_IDX, false, false);
    esp_rom_gpio_connect_out_signal(dc as u32, LCD_DC_IDX, false, false);
    esp_rom_gpio_connect_out_signal(cs as u32, LCD_CS_IDX, false, false);
}