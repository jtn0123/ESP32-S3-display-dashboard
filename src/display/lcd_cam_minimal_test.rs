/// Minimal LCD_CAM test to verify register access without hanging
use anyhow::Result;
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

pub fn test_lcd_cam_minimal() -> Result<()> {
    unsafe {
        log::warn!("=== LCD_CAM Minimal Register Test ===");
        
        // Reset watchdog
        esp_task_wdt_reset();
        
        // Step 1: Enable peripheral clock
        log::info!("Enabling LCD_CAM clock...");
        let old_val = reg_read(SYSTEM_PERIP_CLK_EN1_REG);
        log::info!("  PERIP_CLK_EN1 before: 0x{:08x}", old_val);
        
        reg_write(SYSTEM_PERIP_CLK_EN1_REG, old_val | (1 << 31));
        let new_val = reg_read(SYSTEM_PERIP_CLK_EN1_REG);
        log::info!("  PERIP_CLK_EN1 after:  0x{:08x}", new_val);
        
        // Step 2: Clear reset
        log::info!("Clearing LCD_CAM reset...");
        let old_val = reg_read(SYSTEM_PERIP_RST_EN1_REG);
        log::info!("  PERIP_RST_EN1 before: 0x{:08x}", old_val);
        
        reg_write(SYSTEM_PERIP_RST_EN1_REG, old_val & !(1 << 31));
        let new_val = reg_read(SYSTEM_PERIP_RST_EN1_REG);
        log::info!("  PERIP_RST_EN1 after:  0x{:08x}", new_val);
        
        // Reset watchdog
        esp_task_wdt_reset();
        
        // Step 3: Try to read LCD_CAM registers
        log::info!("Reading LCD_CAM registers...");
        
        let clock_val = reg_read(LCD_CAM_LCD_CLOCK_REG);
        log::info!("  LCD_CLOCK: 0x{:08x}", clock_val);
        
        let user_val = reg_read(LCD_CAM_LCD_USER_REG);
        log::info!("  LCD_USER:  0x{:08x}", user_val);
        
        let misc_val = reg_read(LCD_CAM_LCD_MISC_REG);
        log::info!("  LCD_MISC:  0x{:08x}", misc_val);
        
        // Step 4: Try to write and verify
        log::info!("Testing write/read...");
        
        // Try to set clock enable bit
        reg_write(LCD_CAM_LCD_CLOCK_REG, 1 << 31);
        let read_back = reg_read(LCD_CAM_LCD_CLOCK_REG);
        log::info!("  Wrote 0x{:08x}, read back 0x{:08x}", 1u32 << 31, read_back);
        
        if read_back == 0 {
            log::error!("LCD_CAM registers read as zero - shadow register issue!");
            log::info!("Trying LCD_UPDATE bit to sync shadow registers...");
            
            // Try to set update bit
            reg_write(LCD_CAM_LCD_USER_REG, 1 << 20); // LCD_UPDATE
            esp_rom_delay_us(100);
            
            // Read clock register again
            let read_back2 = reg_read(LCD_CAM_LCD_CLOCK_REG);
            log::info!("  After update: 0x{:08x}", read_back2);
        }
        
        log::warn!("Minimal test complete - no hangs detected");
        
        // Reset watchdog one more time
        esp_task_wdt_reset();
    }
    
    Ok(())
}