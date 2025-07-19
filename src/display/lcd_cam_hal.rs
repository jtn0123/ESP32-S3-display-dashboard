/// Safe HAL wrapper for LCD_CAM peripheral
/// Uses proper memory barriers and access patterns
use esp_idf_sys::*;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};

// Peripheral base addresses from ESP32-S3 TRM
const DR_REG_LCD_CAM_BASE: u32 = 0x6004_1000;
const DR_REG_SYSTEM_BASE: u32 = 0x600C_0000;

// System registers
const SYSTEM_PERIP_CLK_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x24;
const SYSTEM_PERIP_RST_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x28;

// LCD_CAM registers
const LCD_CAM_LCD_CLOCK_REG: u32 = DR_REG_LCD_CAM_BASE + 0x00;
const LCD_CAM_LCD_USER_REG: u32 = DR_REG_LCD_CAM_BASE + 0x04;
const LCD_CAM_LCD_CTRL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x0C;
const LCD_CAM_LCD_CTRL1_REG: u32 = DR_REG_LCD_CAM_BASE + 0x10;
const LCD_CAM_LCD_CTRL2_REG: u32 = DR_REG_LCD_CAM_BASE + 0x14;

// Bit positions
const SYSTEM_LCD_CAM_CLK_EN: u32 = 1 << 31;
const SYSTEM_LCD_CAM_RST: u32 = 1 << 31;

// LCD clock register bits
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_CLKM_DIV_NUM_SHIFT: u32 = 0;
const LCD_CLKM_DIV_NUM_MASK: u32 = 0xFF;

// LCD user register bits
const LCD_RESET: u32 = 1 << 28;
const LCD_START: u32 = 1 << 27;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;
const LCD_CMD: u32 = 1 << 26;

// LCD control register bits
const LCD_RGB_MODE_EN: u32 = 1 << 31;

/// Read a 32-bit register with proper memory barriers
#[inline(always)]
unsafe fn reg_read(addr: u32) -> u32 {
    compiler_fence(Ordering::SeqCst);
    let val = read_volatile(addr as *const u32);
    compiler_fence(Ordering::SeqCst);
    val
}

/// Write a 32-bit register with proper memory barriers
#[inline(always)]
unsafe fn reg_write(addr: u32, val: u32) {
    compiler_fence(Ordering::SeqCst);
    write_volatile(addr as *mut u32, val);
    compiler_fence(Ordering::SeqCst);
}

/// Set bits in a register
#[inline(always)]
unsafe fn reg_set_bits(addr: u32, mask: u32) {
    let val = reg_read(addr);
    reg_write(addr, val | mask);
}

/// Clear bits in a register
#[inline(always)]
unsafe fn reg_clear_bits(addr: u32, mask: u32) {
    let val = reg_read(addr);
    reg_write(addr, val & !mask);
}

pub struct LcdCamHal;

impl LcdCamHal {
    /// Initialize LCD_CAM peripheral with proper sequencing
    pub unsafe fn init() -> Result<(), &'static str> {
        log::info!("LCD_CAM HAL: Starting initialization");
        
        // Step 1: Enable peripheral clock
        log::info!("LCD_CAM HAL: Enabling peripheral clock");
        reg_set_bits(SYSTEM_PERIP_CLK_EN1_REG, SYSTEM_LCD_CAM_CLK_EN);
        
        // Small delay for clock to stabilize
        esp_rom_delay_us(10);
        
        // Step 2: Clear reset (active low)
        log::info!("LCD_CAM HAL: Clearing reset");
        reg_clear_bits(SYSTEM_PERIP_RST_EN1_REG, SYSTEM_LCD_CAM_RST);
        
        // Wait for peripheral to be ready
        esp_rom_delay_us(100);
        
        // Step 3: Verify we can read a register
        log::info!("LCD_CAM HAL: Verifying register access");
        let clock_reg = reg_read(LCD_CAM_LCD_CLOCK_REG);
        log::info!("LCD_CAM HAL: Clock register value: 0x{:08x}", clock_reg);
        
        // Step 4: Reset LCD controller
        log::info!("LCD_CAM HAL: Resetting LCD controller");
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_RESET);
        esp_rom_delay_us(10);
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_RESET);
        esp_rom_delay_us(100);
        
        log::info!("LCD_CAM HAL: Initialization complete");
        Ok(())
    }
    
    /// Configure for 8-bit i8080 mode
    pub unsafe fn configure_i8080_8bit(freq_hz: u32) -> Result<(), &'static str> {
        log::info!("LCD_CAM HAL: Configuring i8080 8-bit mode at {} Hz", freq_hz);
        
        // Calculate clock divider
        const APB_FREQ: u32 = 80_000_000;
        let div_num = (APB_FREQ / freq_hz).saturating_sub(1).min(255) as u32;
        log::info!("LCD_CAM HAL: Clock divider: {}", div_num);
        
        // Configure clock
        let clock_val = LCD_CLK_EN | (div_num << LCD_CLKM_DIV_NUM_SHIFT);
        reg_write(LCD_CAM_LCD_CLOCK_REG, clock_val);
        
        // Configure user register for 8-bit output mode
        let user_val = LCD_DOUT | LCD_8BITS_ORDER;
        reg_write(LCD_CAM_LCD_USER_REG, user_val);
        
        // Disable RGB mode (use i8080)
        reg_clear_bits(LCD_CAM_LCD_CTRL_REG, LCD_RGB_MODE_EN);
        
        // Apply settings
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
        esp_rom_delay_us(10);
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
        
        log::info!("LCD_CAM HAL: i8080 configuration complete");
        Ok(())
    }
    
    /// Send a command byte
    pub unsafe fn send_command(cmd: u8) -> Result<(), &'static str> {
        // Set command mode (DC = 0)
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
        
        // Write command to CTRL2 register (lower 8 bits)
        reg_write(LCD_CAM_LCD_CTRL2_REG, cmd as u32);
        
        // Start transfer
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
        
        // Wait for completion
        let mut timeout = 1000;
        while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
            timeout -= 1;
            if timeout == 0 {
                return Err("LCD_CAM command timeout");
            }
            esp_rom_delay_us(1);
        }
        
        // Clear command mode
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
        
        Ok(())
    }
    
    /// Send data bytes
    pub unsafe fn send_data(data: &[u8]) -> Result<(), &'static str> {
        // Clear command mode (DC = 1 for data)
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
        
        // Send data bytes one by one
        // Note: This is inefficient - real implementation should use DMA
        for &byte in data {
            // Write data to CTRL2 register
            reg_write(LCD_CAM_LCD_CTRL2_REG, byte as u32);
            
            // Start transfer
            reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
            
            // Wait for completion
            let mut timeout = 1000;
            while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
                timeout -= 1;
                if timeout == 0 {
                    return Err("LCD_CAM data timeout");
                }
                esp_rom_delay_us(1);
            }
        }
        
        Ok(())
    }
    
    /// Check if LCD_CAM is idle
    pub unsafe fn is_idle() -> bool {
        (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) == 0
    }
}