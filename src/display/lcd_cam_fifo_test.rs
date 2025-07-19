/// LCD_CAM test with proper FIFO configuration
/// Based on community feedback about missing FIFO setup
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};

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

// Bit definitions
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;

// LCD_CTRL bits - CRITICAL FOR FIFO!
const LCD_AFIFO_RST: u32 = 1 << 30;
const LCD_TX_FIFO_RST: u32 = 1 << 25;
const LCD_TX_FIFO_MOD_SHIFT: u32 = 22;

// Safe register access
#[inline(always)]
unsafe fn reg_read(addr: u32) -> u32 {
    compiler_fence(Ordering::SeqCst);
    let val = read_volatile(addr as *const u32);
    compiler_fence(Ordering::SeqCst);
    val
}

#[inline(always)]
unsafe fn reg_write(addr: u32, val: u32) {
    compiler_fence(Ordering::SeqCst);
    write_volatile(addr as *mut u32, val);
    compiler_fence(Ordering::SeqCst);
}

pub fn lcd_cam_fifo_test(
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
    log::warn!("Starting LCD_CAM FIFO test with proper configuration...");
    
    // Power pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    // Step 1: Enable LCD_CAM peripheral
    unsafe {
        log::info!("Step 1: Enabling LCD_CAM clock and clearing reset");
        let val = reg_read(SYSTEM_PERIP_CLK_EN1_REG);
        reg_write(SYSTEM_PERIP_CLK_EN1_REG, val | (1 << 31));
        
        let val = reg_read(SYSTEM_PERIP_RST_EN1_REG);
        reg_write(SYSTEM_PERIP_RST_EN1_REG, val & !(1 << 31));
        
        esp_rom_delay_us(100);
    }
    
    // Step 2: Configure minimal pins - just WR and D0 first
    unsafe {
        log::info!("Step 2: Configuring WR (GPIO8) and D0 (GPIO39) only");
        
        // WR pin to LCD_PCLK
        esp_rom_gpio_pad_select_gpio(8);
        gpio_set_direction(8 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(8, 154, false, false); // LCD_PCLK signal
        
        // D0 pin to LCD_DATA_OUT0
        esp_rom_gpio_pad_select_gpio(39);
        gpio_set_direction(39 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(39, 133, false, false); // LCD_DATA_OUT0 signal
    }
    
    // Step 3: Configure clock with explicit divider
    unsafe {
        log::info!("Step 3: Setting clock divider for ~40ns pulse width");
        // For 80MHz APB: div=2 gives 40MHz = 25ns period, ~12.5ns low time
        // div=3 gives 26.7MHz = 37.5ns period, ~18.75ns low time
        // div=4 gives 20MHz = 50ns period, ~25ns low time
        reg_write(LCD_CAM_LCD_CLOCK_REG, LCD_CLK_EN | 3); // Try div=4 for safer timing
    }
    
    // Step 4: CRITICAL - Configure FIFO!
    unsafe {
        log::info!("Step 4: CRITICAL - Resetting and configuring FIFO");
        
        // Reset FIFOs
        let ctrl = reg_read(LCD_CAM_LCD_CTRL_REG);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl | LCD_AFIFO_RST | LCD_TX_FIFO_RST);
        esp_rom_delay_us(10);
        
        // Clear resets and set TX FIFO mode
        // Mode 1 = DWord (32-bit) mode
        let ctrl_new = (ctrl & !(LCD_AFIFO_RST | LCD_TX_FIFO_RST | (0x3 << LCD_TX_FIFO_MOD_SHIFT))) 
                      | (1 << LCD_TX_FIFO_MOD_SHIFT);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl_new);
        
        log::info!("FIFO configured - CTRL reg: 0x{:08x}", reg_read(LCD_CAM_LCD_CTRL_REG));
    }
    
    // Step 5: Configure user register
    unsafe {
        log::info!("Step 5: Configuring user register for data mode");
        let user_val = LCD_DOUT | LCD_8BITS_ORDER;
        reg_write(LCD_CAM_LCD_USER_REG, user_val);
        
        // Apply update
        reg_write(LCD_CAM_LCD_USER_REG, user_val | LCD_UPDATE);
        esp_rom_delay_us(10);
        reg_write(LCD_CAM_LCD_USER_REG, user_val);
    }
    
    // Step 6: Try single byte transfer
    log::info!("Step 6: Testing single byte transfer with FIFO enabled");
    
    unsafe {
        // Reset watchdog before test
        esp_task_wdt_reset();
        
        // Clear command mode (data mode)
        let user = reg_read(LCD_CAM_LCD_USER_REG);
        reg_write(LCD_CAM_LCD_USER_REG, user & !LCD_CMD);
        
        // Write one byte
        reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0xAA);
        
        // Start transfer
        reg_write(LCD_CAM_LCD_USER_REG, (user & !LCD_CMD) | LCD_START);
        
        // Wait for completion
        let mut timeout = 10000;
        while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
            timeout -= 1;
            if timeout == 0 {
                log::error!("Transfer timeout!");
                break;
            }
            esp_rom_delay_us(1);
        }
        
        if timeout > 0 {
            log::info!("Transfer completed in {} us", 10000 - timeout);
        }
        
        // Reset watchdog after test
        esp_task_wdt_reset();
    }
    
    // Step 7: Add remaining data pins and test full byte
    unsafe {
        log::info!("Step 7: Adding all data pins D1-D7");
        
        const DATA_PINS: [u8; 7] = [40, 41, 42, 45, 46, 47, 48];
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            esp_rom_gpio_connect_out_signal(pin as u32, 134 + i as u32, false, false); // LCD_DATA_OUT1-7
        }
    }
    
    // Step 8: Test pattern with all pins
    log::info!("Step 8: Sending test pattern on all pins");
    
    let test_patterns = [0x00, 0xFF, 0xAA, 0x55, 0x01, 0x80];
    
    for i in 0..10 {
        for &pattern in &test_patterns {
            unsafe {
                reg_write(LCD_CAM_LCD_CMD_VAL_REG, pattern as u32);
                
                let user = reg_read(LCD_CAM_LCD_USER_REG);
                reg_write(LCD_CAM_LCD_USER_REG, user | LCD_START);
                
                // Poll for completion with timeout
                let mut timeout = 1000;
                while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
                    timeout -= 1;
                    if timeout == 0 {
                        log::error!("Pattern transfer timeout!");
                        break;
                    }
                    esp_rom_delay_us(1);
                }
            }
        }
        
        // Reset watchdog frequently
        unsafe { esp_task_wdt_reset(); }
        
        // Log progress
        if i % 2 == 0 {
            log::info!("Test pattern iteration {}/10", i + 1);
        }
        
        FreeRtos::delay_ms(100);
    }
    
    log::info!("Test complete - check logic analyzer for WR pulses and data patterns");
    log::info!("Expected: WR pulses with varying data patterns on D0-D7");
    log::info!("If no output: FIFO might still be misconfigured");
    
    Ok(())
}