/// LCD_CAM test with proper output enable and pin configuration
/// This addresses the missing output enable that might be causing black screen
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
const LCD_CAM_LCD_DLY_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x30;
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x34;

// Bit definitions
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;

// LCD_CTRL bits
const LCD_AFIFO_RST: u32 = 1 << 30;
const LCD_TX_FIFO_RST: u32 = 1 << 25;
const LCD_TX_FIFO_MOD_SHIFT: u32 = 22;

// LCD_CTRL1 bits - CRITICAL for output enable!
const LCD_VB_FRONT: u32 = 0; // Vertical blank front
const LCD_HA_FRONT: u32 = 0; // Horizontal blank front
const LCD_DOUT_EN: u32 = 1 << 0; // Output enable!

// LCD_MISC bits
const LCD_LCD_NEXT_EN: u32 = 1 << 6; // Next frame enable
const LCD_EXT_MEM_BK_SIZE_SHIFT: u32 = 8;

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

pub fn lcd_cam_output_test(
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
    log::warn!("Starting LCD_CAM output enable test...");
    
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
    
    // Step 2: Configure ALL pins with proper output enable
    unsafe {
        log::info!("Step 2: Configuring all pins with output enable");
        
        const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
        const WR_PIN: u8 = 8;
        const DC_PIN: u8 = 7;
        const CS_PIN: u8 = 6;
        const RST_PIN: u8 = 5;
        
        // Data pins to LCD_DATA_OUT0-7 (signals 133-140)
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_level(pin as gpio_num_t, 0); // Start low
            
            // Set drive strength to maximum (3)
            gpio_set_drive_capability(pin as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
            
            // Connect to LCD_CAM peripheral
            esp_rom_gpio_connect_out_signal(pin as u32, 133 + i as u32, false, false);
        }
        
        // WR pin to LCD_PCLK (signal 154)
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(WR_PIN as gpio_num_t, 1); // WR idle high
        gpio_set_drive_capability(WR_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(WR_PIN as u32, 154, false, false);
        
        // DC pin to LCD_DC (signal 153)
        esp_rom_gpio_pad_select_gpio(DC_PIN as u32);
        gpio_set_direction(DC_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(DC_PIN as gpio_num_t, 1); // DC high for data
        gpio_set_drive_capability(DC_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(DC_PIN as u32, 153, false, false);
        
        // CS pin to LCD_CS (signal 132)
        esp_rom_gpio_pad_select_gpio(CS_PIN as u32);
        gpio_set_direction(CS_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(CS_PIN as gpio_num_t, 0); // CS active low
        gpio_set_drive_capability(CS_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(CS_PIN as u32, 132, false, false);
        
        // RST pin - manual control
        esp_rom_gpio_pad_select_gpio(RST_PIN as u32);
        gpio_set_direction(RST_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(RST_PIN as gpio_num_t, 1); // RST high (not in reset)
    }
    
    // Step 3: Configure clock with safer divider
    unsafe {
        log::info!("Step 3: Setting clock divider for 50ns pulse width");
        // For 80MHz APB: div=7 gives 10MHz = 100ns period, ~50ns low time
        reg_write(LCD_CAM_LCD_CLOCK_REG, LCD_CLK_EN | 7);
    }
    
    // Step 4: Configure output enable in CTRL1 - CRITICAL!
    unsafe {
        log::info!("Step 4: CRITICAL - Setting output enable in CTRL1");
        // Enable output on data pins
        reg_write(LCD_CAM_LCD_CTRL1_REG, LCD_DOUT_EN);
    }
    
    // Step 5: Configure data output mode to always output
    unsafe {
        log::info!("Step 5: Setting data output mode to always output");
        // Set all data pins to always output mode (0xFF)
        reg_write(LCD_CAM_LCD_DATA_DOUT_MODE_REG, 0xFF);
    }
    
    // Step 6: Configure FIFO
    unsafe {
        log::info!("Step 6: Resetting and configuring FIFO");
        
        // Reset FIFOs
        let ctrl = reg_read(LCD_CAM_LCD_CTRL_REG);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl | LCD_AFIFO_RST | LCD_TX_FIFO_RST);
        esp_rom_delay_us(10);
        
        // Clear resets and set TX FIFO mode to 1 (DWord mode)
        let ctrl_new = (ctrl & !(LCD_AFIFO_RST | LCD_TX_FIFO_RST | (0x3 << LCD_TX_FIFO_MOD_SHIFT))) 
                      | (1 << LCD_TX_FIFO_MOD_SHIFT);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl_new);
    }
    
    // Step 7: Configure user register
    unsafe {
        log::info!("Step 7: Configuring user register for 8-bit output");
        let user_val = LCD_DOUT | LCD_8BITS_ORDER;
        reg_write(LCD_CAM_LCD_USER_REG, user_val);
        
        // Apply update
        reg_write(LCD_CAM_LCD_USER_REG, user_val | LCD_UPDATE);
        esp_rom_delay_us(10);
        reg_write(LCD_CAM_LCD_USER_REG, user_val);
    }
    
    // Reset watchdog
    unsafe { esp_task_wdt_reset(); }
    
    // Step 8: Manual GPIO toggle test first
    log::info!("Step 8: Manual GPIO toggle test to verify pins work");
    unsafe {
        for _ in 0..5 {
            // Toggle all data pins manually
            for &pin in &[39, 40, 41, 42, 45, 46, 47, 48] {
                gpio_set_level(pin as gpio_num_t, 1);
            }
            esp_rom_delay_us(10);
            
            for &pin in &[39, 40, 41, 42, 45, 46, 47, 48] {
                gpio_set_level(pin as gpio_num_t, 0);
            }
            esp_rom_delay_us(10);
        }
        
        log::info!("Manual toggle complete - pins should have toggled");
    }
    
    // Step 9: Try LCD_CAM transfer
    log::info!("Step 9: Testing LCD_CAM data transfer with output enabled");
    
    for i in 0..10 {
        unsafe {
            // Reset watchdog
            esp_task_wdt_reset();
            
            // Test pattern
            let pattern = if i % 2 == 0 { 0xFF } else { 0xAA };
            
            // Clear command mode (data mode)
            let user = reg_read(LCD_CAM_LCD_USER_REG);
            reg_write(LCD_CAM_LCD_USER_REG, user & !LCD_CMD);
            
            // Write data
            reg_write(LCD_CAM_LCD_CMD_VAL_REG, pattern as u32);
            
            // Start transfer
            reg_write(LCD_CAM_LCD_USER_REG, (user & !LCD_CMD) | LCD_START);
            
            // Wait with timeout
            let mut timeout = 10000;
            while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
                timeout -= 1;
                if timeout == 0 {
                    log::error!("Transfer timeout at iteration {}!", i);
                    break;
                }
                esp_rom_delay_us(1);
            }
            
            if timeout > 0 {
                log::info!("Transfer {} complete - pattern 0x{:02x}", i, pattern);
            }
        }
        
        FreeRtos::delay_ms(100);
    }
    
    // Step 10: Dump all registers for debugging
    unsafe {
        log::info!("Register dump after configuration:");
        log::info!("  CLOCK: 0x{:08x}", reg_read(LCD_CAM_LCD_CLOCK_REG));
        log::info!("  USER:  0x{:08x}", reg_read(LCD_CAM_LCD_USER_REG));
        log::info!("  MISC:  0x{:08x}", reg_read(LCD_CAM_LCD_MISC_REG));
        log::info!("  CTRL:  0x{:08x}", reg_read(LCD_CAM_LCD_CTRL_REG));
        log::info!("  CTRL1: 0x{:08x}", reg_read(LCD_CAM_LCD_CTRL1_REG));
        log::info!("  CTRL2: 0x{:08x}", reg_read(LCD_CAM_LCD_CTRL2_REG));
        log::info!("  DLY_MODE: 0x{:08x}", reg_read(LCD_CAM_LCD_DLY_MODE_REG));
        log::info!("  DOUT_MODE: 0x{:08x}", reg_read(LCD_CAM_LCD_DATA_DOUT_MODE_REG));
    }
    
    log::info!("Test complete - check display and logic analyzer");
    log::info!("Key additions: Output enable in CTRL1, data output mode, pin drive strength");
    
    Ok(())
}