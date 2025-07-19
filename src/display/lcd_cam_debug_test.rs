/// LCD_CAM debug test - verify pins are being driven
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use super::lcd_cam_hal::LcdCamHal;

// Direct GPIO pins
const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
const WR_PIN: u8 = 8;
const DC_PIN: u8 = 7;
const CS_PIN: u8 = 6;
const RST_PIN: u8 = 5;

// GPIO Matrix signal indices
const LCD_DATA_OUT_IDX_BASE: u32 = 133;
const LCD_PCLK_IDX: u32 = 154;
const LCD_DC_IDX: u32 = 153;
const LCD_CS_IDX: u32 = 132;

// LCD_CAM registers
const DR_REG_LCD_CAM_BASE: u32 = 0x6004_1000;
const LCD_CAM_LCD_MISC_REG: u32 = DR_REG_LCD_CAM_BASE + 0x08;
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x34;

pub fn lcd_cam_debug_test(
    _d0: impl Into<AnyIOPin>,
    _d1: impl Into<AnyIOPin>,
    _d2: impl Into<AnyIOPin>,
    _d3: impl Into<AnyIOPin>,
    _d4: impl Into<AnyIOPin>,
    _d5: impl Into<AnyIOPin>,
    _d6: impl Into<AnyIOPin>,
    _d7: impl Into<AnyIOPin>,
    _wr: impl Into<AnyIOPin>,
    _dc: impl Into<AnyIOPin>,
    _cs: impl Into<AnyIOPin>,
    _rst: impl Into<AnyIOPin>,
) -> Result<()> {
    log::warn!("Starting LCD_CAM debug test...");
    
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
    
    // First, let's test direct GPIO control to verify pins work
    log::info!("Testing direct GPIO control first...");
    unsafe {
        // Configure all pins as outputs
        for &pin in &DATA_PINS {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        }
        
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        
        // Toggle data pins to verify they work
        for i in 0..3 {
            log::info!("Direct GPIO toggle {}/3", i + 1);
            
            // Set all data pins high
            for &pin in &DATA_PINS {
                gpio_set_level(pin as gpio_num_t, 1);
            }
            gpio_set_level(WR_PIN as gpio_num_t, 1);
            FreeRtos::delay_ms(100);
            
            // Set all data pins low
            for &pin in &DATA_PINS {
                gpio_set_level(pin as gpio_num_t, 0);
            }
            gpio_set_level(WR_PIN as gpio_num_t, 0);
            FreeRtos::delay_ms(100);
        }
    }
    
    log::info!("Direct GPIO test complete. Now testing LCD_CAM...");
    
    // Configure GPIO matrix for LCD_CAM
    unsafe {
        // Connect data pins to LCD_CAM signals
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            esp_rom_gpio_connect_out_signal(pin as u32, LCD_DATA_OUT_IDX_BASE + i as u32, false, false);
            log::info!("Connected GPIO {} to LCD_DATA_OUT{}", pin, i);
        }
        
        // Connect WR pin
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(WR_PIN as u32, LCD_PCLK_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_PCLK", WR_PIN);
        
        // Connect DC pin
        esp_rom_gpio_pad_select_gpio(DC_PIN as u32);
        gpio_set_direction(DC_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(DC_PIN as u32, LCD_DC_IDX, false, false);
        
        // Connect CS pin
        esp_rom_gpio_pad_select_gpio(CS_PIN as u32);
        gpio_set_direction(CS_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(CS_PIN as u32, LCD_CS_IDX, false, false);
    }
    
    // Initialize LCD_CAM
    unsafe {
        LcdCamHal::init().map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Check LCD_CAM_LCD_MISC_REG
        let misc_reg = core::ptr::read_volatile(LCD_CAM_LCD_MISC_REG as *const u32);
        log::info!("LCD_CAM_LCD_MISC_REG = 0x{:08x}", misc_reg);
        
        // Check LCD_CAM_LCD_DATA_DOUT_MODE_REG
        let dout_mode = core::ptr::read_volatile(LCD_CAM_LCD_DATA_DOUT_MODE_REG as *const u32);
        log::info!("LCD_CAM_LCD_DATA_DOUT_MODE_REG = 0x{:08x}", dout_mode);
        
        // Set data output mode to always output (not just during valid phase)
        core::ptr::write_volatile(LCD_CAM_LCD_DATA_DOUT_MODE_REG as *mut u32, 0xFF);
        
        // Configure for 8-bit mode but very slow for debugging
        LcdCamHal::configure_i8080_8bit(1_000_000).map_err(|e| anyhow::anyhow!("{}", e))?; // 1 MHz
    }
    
    // Now try to send data through LCD_CAM
    log::info!("Attempting to send data through LCD_CAM...");
    
    let mut toggle = false;
    loop {
        unsafe {
            // Send alternating patterns
            let data = if toggle { 0xFF } else { 0x00 };
            
            // Try sending as command first
            match LcdCamHal::send_command(data) {
                Ok(_) => log::info!("Sent command 0x{:02x}", data),
                Err(e) => log::error!("Failed to send command: {}", e),
            }
            
            // Try sending as data
            match LcdCamHal::send_data(&[data]) {
                Ok(_) => log::info!("Sent data 0x{:02x}", data),
                Err(e) => log::error!("Failed to send data: {}", e),
            }
            
            // Check if we can read pin states
            for &pin in &DATA_PINS {
                let level = gpio_get_level(pin as gpio_num_t);
                log::info!("GPIO {} level: {}", pin, level);
            }
            
            esp_task_wdt_reset();
        }
        
        toggle = !toggle;
        FreeRtos::delay_ms(1000);
    }
}