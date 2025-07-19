/// Test LCD_CAM data transfer with GPIO matrix
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use super::lcd_cam_hal::LcdCamHal;

// GPIO Matrix signal indices from ESP-IDF
const LCD_DATA_OUT_IDX_BASE: u32 = 133;  // LCD_DATA_OUT0_IDX through LCD_DATA_OUT7_IDX
const LCD_PCLK_IDX: u32 = 154;           // Write clock
const LCD_DC_IDX: u32 = 153;             // Data/Command
const LCD_CS_IDX: u32 = 132;             // Chip select

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

pub fn lcd_cam_data_test(
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
    log::warn!("Starting LCD_CAM data transfer test...");
    
    // Extract pin numbers
    let pins = [
        get_pin_number(d0)?, get_pin_number(d1)?, 
        get_pin_number(d2)?, get_pin_number(d3)?,
        get_pin_number(d4)?, get_pin_number(d5)?, 
        get_pin_number(d6)?, get_pin_number(d7)?
    ];
    let pin_wr = get_pin_number(wr)?;
    let pin_dc = get_pin_number(dc)?;
    let pin_cs = get_pin_number(cs)?;
    let pin_rst = get_pin_number(rst)?;
    
    log::info!("Pin configuration:");
    log::info!("  Data: {:?}", pins);
    log::info!("  WR: {}, DC: {}, CS: {}, RST: {}", pin_wr, pin_dc, pin_cs, pin_rst);
    
    // Initialize power pins
    unsafe {
        // LCD power
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        // Backlight
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    FreeRtos::delay_ms(100);
    
    // Configure GPIO matrix for LCD_CAM
    unsafe {
        // Configure data pins D0-D7
        for (i, &pin) in pins.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            esp_rom_gpio_connect_out_signal(
                pin as u32,
                LCD_DATA_OUT_IDX_BASE + i as u32,
                false,
                false
            );
            log::info!("Connected GPIO {} to LCD_DATA_OUT{}", pin, i);
        }
        
        // Configure control pins
        esp_rom_gpio_pad_select_gpio(pin_wr as u32);
        gpio_set_direction(pin_wr as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(pin_wr as u32, LCD_PCLK_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_PCLK", pin_wr);
        
        esp_rom_gpio_pad_select_gpio(pin_dc as u32);
        gpio_set_direction(pin_dc as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(pin_dc as u32, LCD_DC_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_DC", pin_dc);
        
        esp_rom_gpio_pad_select_gpio(pin_cs as u32);
        gpio_set_direction(pin_cs as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(pin_cs as u32, LCD_CS_IDX, false, false);
        log::info!("Connected GPIO {} to LCD_CS", pin_cs);
        
        // Configure RST pin (not part of LCD_CAM)
        esp_rom_gpio_pad_select_gpio(pin_rst as u32);
        gpio_set_direction(pin_rst as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(pin_rst as gpio_num_t, 1);
    }
    
    // Hardware reset
    unsafe {
        gpio_set_level(pin_rst as gpio_num_t, 1);
        FreeRtos::delay_ms(10);
        gpio_set_level(pin_rst as gpio_num_t, 0);
        FreeRtos::delay_ms(10);
        gpio_set_level(pin_rst as gpio_num_t, 1);
        FreeRtos::delay_ms(120);
    }
    log::info!("Display reset complete");
    
    // Initialize LCD_CAM
    unsafe {
        LcdCamHal::init().map_err(|e| anyhow::anyhow!("{}", e))?;
        LcdCamHal::configure_i8080_8bit(10_000_000).map_err(|e| anyhow::anyhow!("{}", e))?; // 10 MHz for testing
    }
    
    // Initialize display
    log::info!("Initializing ST7789 display...");
    unsafe {
        // Software reset
        LcdCamHal::send_command(CMD_SWRESET).map_err(|e| anyhow::anyhow!("{}", e))?;
        FreeRtos::delay_ms(150);
        
        // Sleep out
        LcdCamHal::send_command(CMD_SLPOUT).map_err(|e| anyhow::anyhow!("{}", e))?;
        FreeRtos::delay_ms(120);
        
        // Set color mode to 16-bit RGB565
        LcdCamHal::send_command(CMD_COLMOD).map_err(|e| anyhow::anyhow!("{}", e))?;
        // TODO: Send data byte 0x55 for RGB565
        
        // Display inversion on
        LcdCamHal::send_command(CMD_INVON).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Display on
        LcdCamHal::send_command(CMD_DISPON).map_err(|e| anyhow::anyhow!("{}", e))?;
        FreeRtos::delay_ms(20);
    }
    
    log::info!("ST7789 initialized, starting color test...");
    
    // Test pattern - just toggle backlight for now since we need data transfer
    let mut count = 0u32;
    loop {
        // Toggle backlight to show we're running
        unsafe {
            gpio_set_level(38 as gpio_num_t, (count & 1) as u32);
        }
        
        count += 1;
        FreeRtos::delay_ms(500);
        
        if count % 4 == 0 {
            log::info!("LCD_CAM data test running... (toggle {}, backlight: {})", 
                     count / 2, count & 1);
            unsafe { esp_task_wdt_reset(); }
        }
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