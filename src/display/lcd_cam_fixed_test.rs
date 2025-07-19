/// LCD_CAM implementation with proper LCD_MISC configuration
/// Based on fault tree analysis and ESP-IDF lcd_ll implementation
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

// Clock register bits
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_CLK_SEL: u32 = 3 << 29;  // Clock source selection
const LCD_CLKM_DIV_B_SHIFT: u32 = 23;
const LCD_CLKM_DIV_A_SHIFT: u32 = 17;
const LCD_CLKM_DIV_NUM_SHIFT: u32 = 9;
const LCD_CLK_EQU_SYSCLK: u32 = 1 << 8;

// User register bits
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;
const LCD_BIT_ORDER: u32 = 1 << 19;
const LCD_BYTE_ORDER: u32 = 1 << 18;
const LCD_2BYTE_EN: u32 = 1 << 17;

// MISC register bits - CRITICAL for output enable!
const LCD_CD_CMD_SET: u32 = 1 << 30;      // Command mode output enable
const LCD_CD_DUMMY_SET: u32 = 1 << 29;    // Dummy mode output enable  
const LCD_CD_DATA_SET: u32 = 1 << 28;     // Data mode output enable
const LCD_CD_IDLE_EDGE: u32 = 1 << 27;    // CD idle level
const LCD_AFIFO_ADDR_BRIDGE_EN: u32 = 1 << 26;  // AFIFO address bridge enable
const LCD_AFIFO_RESET: u32 = 1 << 25;     // AFIFO reset

// CTRL register bits
const LCD_TX_FIFO_RST: u32 = 1 << 13;
const LCD_TX_FIFO_MOD_SHIFT: u32 = 10;

// CTRL1 register bits
const LCD_DOUT_EN: u32 = 1 << 0;  // Output enable

// CTRL2 register bits
const LCD_CD_MODE: u32 = 1 << 31;         // CD mode: 0=data, 1=cmd
const LCD_LCD_VSYNC_WIDTH_SHIFT: u32 = 7;
const LCD_LCD_VSYNC_IDLE_POL: u32 = 1 << 6;
const LCD_LCD_DE_IDLE_POL: u32 = 1 << 5;
const LCD_LCD_HS_BLANK_EN: u32 = 1 << 4;
const LCD_LCD_HSYNC_IDLE_POL: u32 = 1 << 3;
const LCD_LCD_HSYNC_POSITION: u32 = 1 << 2;

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

#[inline(always)]
unsafe fn reg_set_bits(addr: u32, bits: u32) {
    let val = reg_read(addr);
    reg_write(addr, val | bits);
}

#[inline(always)]
unsafe fn reg_clear_bits(addr: u32, bits: u32) {
    let val = reg_read(addr);
    reg_write(addr, val & !bits);
}

pub fn lcd_cam_fixed_test(
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
    log::warn!("=== LCD_CAM Fixed Implementation Test ===");
    log::warn!("Implementing missing LCD_MISC register bits");
    
    // Power and backlight pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    // Step 1: Enable LCD_CAM peripheral clock and release reset
    unsafe {
        log::info!("Step 1: Enable LCD_CAM clock and release reset");
        reg_set_bits(SYSTEM_PERIP_CLK_EN1_REG, 1 << 31);
        reg_clear_bits(SYSTEM_PERIP_RST_EN1_REG, 1 << 31);
        esp_rom_delay_us(100);
    }
    
    // Step 2: Configure GPIO Matrix routing
    unsafe {
        log::info!("Step 2: Configure GPIO Matrix for LCD signals");
        
        const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
        const WR_PIN: u8 = 8;
        const DC_PIN: u8 = 7;
        const CS_PIN: u8 = 6;
        const RST_PIN: u8 = 5;
        
        // Data pins D0-D7 -> LCD_DATA_OUT0-7 (signals 133-140)
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_drive_capability(pin as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
            esp_rom_gpio_connect_out_signal(pin as u32, 133 + i as u32, false, false);
        }
        
        // WR -> LCD_PCLK (signal 154)
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_drive_capability(WR_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(WR_PIN as u32, 154, false, false);
        
        // DC -> LCD_DC (signal 153)
        esp_rom_gpio_pad_select_gpio(DC_PIN as u32);
        gpio_set_direction(DC_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_drive_capability(DC_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(DC_PIN as u32, 153, false, false);
        
        // CS -> LCD_CS (signal 132)
        esp_rom_gpio_pad_select_gpio(CS_PIN as u32);
        gpio_set_direction(CS_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_drive_capability(CS_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(CS_PIN as u32, 132, false, false);
        
        // RST - manual control
        esp_rom_gpio_pad_select_gpio(RST_PIN as u32);
        gpio_set_direction(RST_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(RST_PIN as gpio_num_t, 1);
    }
    
    // Step 3: Configure clock - 10MHz for safe operation
    unsafe {
        log::info!("Step 3: Configure LCD clock (10MHz)");
        // Clock = 160MHz / (1 + 15) = 10MHz
        let clk_val = LCD_CLK_EN | (15 << LCD_CLKM_DIV_NUM_SHIFT);
        reg_write(LCD_CAM_LCD_CLOCK_REG, clk_val);
    }
    
    // Step 4: Configure LCD_MISC - CRITICAL NEW BITS!
    unsafe {
        log::info!("Step 4: Configure LCD_MISC with output enables");
        let misc_val = LCD_CD_CMD_SET |        // Enable CD output in command mode
                      LCD_CD_DATA_SET |         // Enable CD output in data mode
                      LCD_CD_DUMMY_SET |        // Enable CD output in dummy mode
                      LCD_AFIFO_ADDR_BRIDGE_EN; // Enable AFIFO address bridge
        reg_write(LCD_CAM_LCD_MISC_REG, misc_val);
        log::info!("  LCD_MISC = 0x{:08x}", misc_val);
    }
    
    // Step 5: Configure LCD_USER for 8-bit mode
    unsafe {
        log::info!("Step 5: Configure LCD_USER for 8-bit output");
        let user_val = LCD_DOUT |              // Enable data output
                      LCD_8BITS_ORDER |         // 8-bit mode
                      0;                        // Clear other bits
        reg_write(LCD_CAM_LCD_USER_REG, user_val);
    }
    
    // Step 6: Configure CTRL registers
    unsafe {
        log::info!("Step 6: Configure CTRL registers");
        
        // Reset FIFOs
        reg_set_bits(LCD_CAM_LCD_CTRL_REG, LCD_TX_FIFO_RST | LCD_AFIFO_RESET);
        esp_rom_delay_us(10);
        reg_clear_bits(LCD_CAM_LCD_CTRL_REG, LCD_TX_FIFO_RST | LCD_AFIFO_RESET);
        
        // Set TX FIFO mode to 1 (best for 8-bit)
        let ctrl_val = reg_read(LCD_CAM_LCD_CTRL_REG);
        let ctrl_new = (ctrl_val & !(0x3 << LCD_TX_FIFO_MOD_SHIFT)) | (1 << LCD_TX_FIFO_MOD_SHIFT);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl_new);
        
        // Enable output in CTRL1
        reg_write(LCD_CAM_LCD_CTRL1_REG, LCD_DOUT_EN);
        
        // Configure CTRL2 for i8080 mode
        reg_write(LCD_CAM_LCD_CTRL2_REG, 0);  // All default values
    }
    
    // Step 7: Configure data output mode
    unsafe {
        log::info!("Step 7: Configure data output mode");
        reg_write(LCD_CAM_LCD_DATA_DOUT_MODE_REG, 0xFF);  // All pins always output
    }
    
    // Step 8: Apply LCD_UPDATE to make settings take effect
    unsafe {
        log::info!("Step 8: Apply LCD_UPDATE");
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
        esp_rom_delay_us(10);
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
    }
    
    // Reset watchdog
    unsafe { esp_task_wdt_reset(); }
    
    // Step 9: Dump registers before test
    unsafe {
        log::info!("Register state before test:");
        log::info!("  CLOCK:     0x{:08x}", reg_read(LCD_CAM_LCD_CLOCK_REG));
        log::info!("  USER:      0x{:08x}", reg_read(LCD_CAM_LCD_USER_REG));
        log::info!("  MISC:      0x{:08x}", reg_read(LCD_CAM_LCD_MISC_REG));
        log::info!("  CTRL:      0x{:08x}", reg_read(LCD_CAM_LCD_CTRL_REG));
        log::info!("  CTRL1:     0x{:08x}", reg_read(LCD_CAM_LCD_CTRL1_REG));
        log::info!("  CTRL2:     0x{:08x}", reg_read(LCD_CAM_LCD_CTRL2_REG));
        log::info!("  DOUT_MODE: 0x{:08x}", reg_read(LCD_CAM_LCD_DATA_DOUT_MODE_REG));
    }
    
    // Step 10: Test single byte write
    log::info!("Step 10: Testing single byte write");
    unsafe {
        // Write test pattern 0x5A
        reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0x5A);
        
        // Start transfer
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
        
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
            log::info!("Transfer complete! Check logic analyzer for WR pulse with 0x5A on D[7:0]");
        }
    }
    
    // Step 11: Test command/data sequences
    log::info!("Step 11: Testing command/data sequences");
    for i in 0..5 {
        unsafe {
            esp_task_wdt_reset();
            
            // Send command (DC low)
            reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
            reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0x2C + i);  // Different commands
            reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
            
            // Wait
            let mut timeout = 10000;
            while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
                timeout -= 1;
                esp_rom_delay_us(1);
            }
            
            // Send data (DC high)
            reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
            reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0xAA + i * 0x11);  // Different data
            reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
            
            // Wait
            timeout = 10000;
            while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
                timeout -= 1;
                esp_rom_delay_us(1);
            }
            
            log::info!("Sent cmd 0x{:02X}, data 0x{:02X}", 0x2C + i, 0xAA + i * 0x11);
        }
        
        FreeRtos::delay_ms(10);
    }
    
    log::warn!("Test complete!");
    log::warn!("Key additions from fault tree:");
    log::warn!("  - LCD_CD_CMD_SET/DATA_SET in LCD_MISC");
    log::warn!("  - LCD_AFIFO_ADDR_BRIDGE_EN in LCD_MISC");
    log::warn!("  - Proper LCD_UPDATE pulse after config");
    log::warn!("Check logic analyzer - should see WR pulses and data!");
    
    Ok(())
}