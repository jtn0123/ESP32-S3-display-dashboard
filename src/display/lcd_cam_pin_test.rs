/// LCD_CAM pin toggle test - simplest possible test
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use core::ptr::{read_volatile, write_volatile};

// LCD_CAM registers
const DR_REG_LCD_CAM_BASE: u32 = 0x6004_1000;
const DR_REG_SYSTEM_BASE: u32 = 0x600C_0000;

const SYSTEM_PERIP_CLK_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x24;
const SYSTEM_PERIP_RST_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x28;

const LCD_CAM_LCD_CLOCK_REG: u32 = DR_REG_LCD_CAM_BASE + 0x00;
const LCD_CAM_LCD_USER_REG: u32 = DR_REG_LCD_CAM_BASE + 0x04;
const LCD_CAM_LCD_MISC_REG: u32 = DR_REG_LCD_CAM_BASE + 0x08;
const LCD_CAM_LCD_CTRL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x0C;
const LCD_CAM_LCD_CTRL1_REG: u32 = DR_REG_LCD_CAM_BASE + 0x10;
const LCD_CAM_LCD_CTRL2_REG: u32 = DR_REG_LCD_CAM_BASE + 0x14;
const LCD_CAM_LCD_CMD_VAL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x18;

// Additional registers that might be important
const LCD_CAM_LCD_RGB_YUV_REG: u32 = DR_REG_LCD_CAM_BASE + 0x1C;
const LCD_CAM_LCD_DLY_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x30;
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x34;

pub fn lcd_cam_pin_test(
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
    log::warn!("Starting LCD_CAM pin toggle test...");
    
    // Power pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    // Configure GPIO matrix
    const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
    const WR_PIN: u8 = 8;
    
    unsafe {
        // Data pins to LCD_DATA_OUT0-7 (signals 133-140)
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            esp_rom_gpio_connect_out_signal(pin as u32, 133 + i as u32, false, false);
        }
        
        // WR to LCD_PCLK (signal 154)
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(WR_PIN as u32, 154, false, false);
    }
    
    // Enable LCD_CAM peripheral
    unsafe {
        // Enable clock
        let val = read_volatile(SYSTEM_PERIP_CLK_EN1_REG as *const u32);
        write_volatile(SYSTEM_PERIP_CLK_EN1_REG as *mut u32, val | (1 << 31));
        
        // Clear reset
        let val = read_volatile(SYSTEM_PERIP_RST_EN1_REG as *const u32);
        write_volatile(SYSTEM_PERIP_RST_EN1_REG as *mut u32, val & !(1 << 31));
        
        esp_rom_delay_us(100);
    }
    
    // Dump all LCD_CAM registers
    unsafe {
        log::info!("LCD_CAM register dump:");
        log::info!("  CLOCK: 0x{:08x}", read_volatile(LCD_CAM_LCD_CLOCK_REG as *const u32));
        log::info!("  USER:  0x{:08x}", read_volatile(LCD_CAM_LCD_USER_REG as *const u32));
        log::info!("  MISC:  0x{:08x}", read_volatile(LCD_CAM_LCD_MISC_REG as *const u32));
        log::info!("  CTRL:  0x{:08x}", read_volatile(LCD_CAM_LCD_CTRL_REG as *const u32));
        log::info!("  CTRL1: 0x{:08x}", read_volatile(LCD_CAM_LCD_CTRL1_REG as *const u32));
        log::info!("  CTRL2: 0x{:08x}", read_volatile(LCD_CAM_LCD_CTRL2_REG as *const u32));
        log::info!("  DLY_MODE: 0x{:08x}", read_volatile(LCD_CAM_LCD_DLY_MODE_REG as *const u32));
        log::info!("  DOUT_MODE: 0x{:08x}", read_volatile(LCD_CAM_LCD_DATA_DOUT_MODE_REG as *const u32));
    }
    
    // Try different configurations
    unsafe {
        // Enable clock
        write_volatile(LCD_CAM_LCD_CLOCK_REG as *mut u32, 0x80000001); // CLK_EN | DIV=1
        
        // Try configuration from ESP-IDF
        // Set data output mode - always output
        write_volatile(LCD_CAM_LCD_DATA_DOUT_MODE_REG as *mut u32, 0xFF);
        
        // Configure control register
        write_volatile(LCD_CAM_LCD_CTRL_REG as *mut u32, 0x00000000); // i8080 mode
        
        // Configure CTRL1 - this might be important!
        // Bits here control output enable and other signals
        write_volatile(LCD_CAM_LCD_CTRL1_REG as *mut u32, 0x00000000);
        
        // Configure user register
        const LCD_DOUT: u32 = 1 << 24;
        const LCD_8BITS_ORDER: u32 = 1 << 23;
        const LCD_UPDATE: u32 = 1 << 20;
        const LCD_START: u32 = 1 << 27;
        
        let user_val = LCD_DOUT | LCD_8BITS_ORDER;
        write_volatile(LCD_CAM_LCD_USER_REG as *mut u32, user_val);
        
        // Apply update
        write_volatile(LCD_CAM_LCD_USER_REG as *mut u32, user_val | LCD_UPDATE);
        esp_rom_delay_us(10);
        write_volatile(LCD_CAM_LCD_USER_REG as *mut u32, user_val);
    }
    
    log::info!("LCD_CAM configured. Testing data output...");
    
    // Try to output data
    let mut toggle = 0u8;
    loop {
        unsafe {
            // Write data to CTRL2
            write_volatile(LCD_CAM_LCD_CTRL2_REG as *mut u32, toggle as u32);
            
            // Start transfer
            let user = read_volatile(LCD_CAM_LCD_USER_REG as *const u32);
            write_volatile(LCD_CAM_LCD_USER_REG as *mut u32, user | LCD_START);
            
            // Wait a bit
            esp_rom_delay_us(10);
            
            // Check if START cleared
            let user_after = read_volatile(LCD_CAM_LCD_USER_REG as *const u32);
            if (user_after & LCD_START) == 0 {
                log::info!("Transfer complete for data 0x{:02x}", toggle);
            } else {
                log::warn!("Transfer still active!");
            }
            
            // Also try direct write to CMD_VAL
            write_volatile(LCD_CAM_LCD_CMD_VAL_REG as *mut u32, 0xAAAAAAAA);
            
            esp_task_wdt_reset();
        }
        
        toggle = if toggle == 0xFF { 0x00 } else { 0xFF };
        FreeRtos::delay_ms(500);
    }
}