/// Simple LCD_CAM test - verify basic functionality
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use super::colors::{BLACK, WHITE, PRIMARY_RED};

// Direct GPIO pins for comparison
const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
const WR_PIN: u8 = 8;
const DC_PIN: u8 = 7;
const CS_PIN: u8 = 6;
const RST_PIN: u8 = 5;

// ST7789 commands
const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
const CMD_INVON: u8 = 0x21;
const CMD_DISPON: u8 = 0x29;
const CMD_CASET: u8 = 0x2A;
const CMD_RASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_MADCTL: u8 = 0x36;
const CMD_COLMOD: u8 = 0x3A;

pub fn lcd_cam_simple_test(
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
) -> Result<()> {
    log::warn!("Starting LCD_CAM simple test - using direct GPIO for now...");
    
    // Initialize power pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    FreeRtos::delay_ms(100);
    
    // Configure all pins as outputs using direct GPIO
    unsafe {
        // Data pins
        for &pin in &DATA_PINS {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        }
        
        // Control pins
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(WR_PIN as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(DC_PIN as u32);
        gpio_set_direction(DC_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(DC_PIN as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(CS_PIN as u32);
        gpio_set_direction(CS_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(CS_PIN as gpio_num_t, 0); // CS active low
        
        esp_rom_gpio_pad_select_gpio(RST_PIN as u32);
        gpio_set_direction(RST_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(RST_PIN as gpio_num_t, 1);
    }
    
    // Hardware reset
    unsafe {
        gpio_set_level(RST_PIN as gpio_num_t, 1);
        FreeRtos::delay_ms(10);
        gpio_set_level(RST_PIN as gpio_num_t, 0);
        FreeRtos::delay_ms(10);
        gpio_set_level(RST_PIN as gpio_num_t, 1);
        FreeRtos::delay_ms(120);
    }
    
    // Helper to write byte using direct GPIO
    unsafe fn write_byte(data: u8, is_command: bool) {
        // Set DC pin
        gpio_set_level(DC_PIN as gpio_num_t, if is_command { 0 } else { 1 });
        
        // Set data pins
        for i in 0..8 {
            let bit = (data >> i) & 1;
            gpio_set_level(DATA_PINS[i] as gpio_num_t, bit as u32);
        }
        
        // Pulse WR
        gpio_set_level(WR_PIN as gpio_num_t, 0);
        // Small delay for timing
        esp_rom_delay_us(1);
        gpio_set_level(WR_PIN as gpio_num_t, 1);
        esp_rom_delay_us(1);
    }
    
    // Initialize display
    log::info!("Initializing ST7789 display with direct GPIO...");
    unsafe {
        // Software reset
        write_byte(CMD_SWRESET, true);
        FreeRtos::delay_ms(150);
        
        // Sleep out
        write_byte(CMD_SLPOUT, true);
        FreeRtos::delay_ms(120);
        
        // Memory access control
        write_byte(CMD_MADCTL, true);
        write_byte(0x60, false); // Landscape mode
        
        // Pixel format
        write_byte(CMD_COLMOD, true);
        write_byte(0x55, false); // 16-bit RGB565
        
        // Inversion on
        write_byte(CMD_INVON, true);
        
        // Display on
        write_byte(CMD_DISPON, true);
        FreeRtos::delay_ms(20);
    }
    
    log::info!("Display initialized, drawing test pattern...");
    
    // Draw a simple red rectangle
    unsafe {
        // Set column address (x = 50 to 250)
        write_byte(CMD_CASET, true);
        write_byte(0, false); // x start high
        write_byte(60, false); // x start low (50 + 10 offset)
        write_byte(1, false); // x end high
        write_byte(4, false); // x end low (260 total)
        
        // Set row address (y = 50 to 118)
        write_byte(CMD_RASET, true);
        write_byte(0, false); // y start high
        write_byte(86, false); // y start low (50 + 36 offset)
        write_byte(0, false); // y end high
        write_byte(154, false); // y end low (118 + 36 offset)
        
        // Start memory write
        write_byte(CMD_RAMWR, true);
        
        // Fill with red (200x68 pixels)
        let red_high = (PRIMARY_RED >> 8) as u8;
        let red_low = (PRIMARY_RED & 0xFF) as u8;
        
        for _ in 0..(200 * 68) {
            write_byte(red_high, false);
            write_byte(red_low, false);
        }
    }
    
    log::info!("Test pattern drawn. If you see a red rectangle, direct GPIO works!");
    
    // Now let's try LCD_CAM approach with debugging
    log::warn!("Now testing LCD_CAM approach...");
    
    // Configure GPIO matrix for LCD_CAM
    const LCD_DATA_OUT_IDX_BASE: u32 = 133;
    const LCD_PCLK_IDX: u32 = 154;
    const LCD_DC_IDX: u32 = 153;
    const LCD_CS_IDX: u32 = 132;
    
    unsafe {
        // First, disconnect pins from GPIO matrix (set to GPIO function)
        for &pin in &DATA_PINS {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        }
        
        // Now connect to LCD_CAM signals
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_connect_out_signal(pin as u32, LCD_DATA_OUT_IDX_BASE + i as u32, false, false);
            log::info!("Connected GPIO {} to LCD_DATA_OUT{} (signal {})", pin, i, LCD_DATA_OUT_IDX_BASE + i as u32);
        }
        
        esp_rom_gpio_connect_out_signal(WR_PIN as u32, LCD_PCLK_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_PCLK (signal {})", WR_PIN, LCD_PCLK_IDX);
        
        esp_rom_gpio_connect_out_signal(DC_PIN as u32, LCD_DC_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_DC (signal {})", DC_PIN, LCD_DC_IDX);
        
        esp_rom_gpio_connect_out_signal(CS_PIN as u32, LCD_CS_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_CS (signal {})", CS_PIN, LCD_CS_IDX);
    }
    
    // Initialize LCD_CAM peripheral
    const DR_REG_LCD_CAM_BASE: u32 = 0x6004_1000;
    const DR_REG_SYSTEM_BASE: u32 = 0x600C_0000;
    const SYSTEM_PERIP_CLK_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x24;
    const SYSTEM_PERIP_RST_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x28;
    const SYSTEM_LCD_CAM_CLK_EN: u32 = 1 << 31;
    const SYSTEM_LCD_CAM_RST: u32 = 1 << 31;
    
    unsafe {
        // Enable peripheral clock
        let clk_en = core::ptr::read_volatile(SYSTEM_PERIP_CLK_EN1_REG as *const u32);
        core::ptr::write_volatile(SYSTEM_PERIP_CLK_EN1_REG as *mut u32, clk_en | SYSTEM_LCD_CAM_CLK_EN);
        
        // Clear reset
        let rst_en = core::ptr::read_volatile(SYSTEM_PERIP_RST_EN1_REG as *const u32);
        core::ptr::write_volatile(SYSTEM_PERIP_RST_EN1_REG as *mut u32, rst_en & !SYSTEM_LCD_CAM_RST);
        
        esp_rom_delay_us(100);
        
        log::info!("LCD_CAM peripheral enabled");
    }
    
    // Try to read LCD_CAM registers to verify it's accessible
    const LCD_CAM_LCD_USER_REG: u32 = DR_REG_LCD_CAM_BASE + 0x04;
    unsafe {
        let user_reg = core::ptr::read_volatile(LCD_CAM_LCD_USER_REG as *const u32);
        log::info!("LCD_CAM_LCD_USER_REG = 0x{:08x}", user_reg);
    }
    
    // Keep the test running
    loop {
        FreeRtos::delay_ms(1000);
        log::info!("LCD_CAM simple test running...");
        unsafe { esp_task_wdt_reset(); }
    }
}

fn get_pin_number(pin: impl Into<AnyIOPin>) -> Result<u8> {
    let any_pin: AnyIOPin = pin.into();
    let pin_num = unsafe { 
        let ptr = &any_pin as *const _ as *const u8;
        *ptr.offset(0)
    };
    Ok(pin_num)
}