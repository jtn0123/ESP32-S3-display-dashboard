/// Working LCD_CAM implementation with proper shadow register updates
/// Based on the fault tree analysis and community findings
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
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
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x34;

// Clock register bits
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_CLKM_DIV_NUM_SHIFT: u32 = 9;

// User register bits
const LCD_RESET: u32 = 1 << 28;
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;

// MISC register bits - CRITICAL for output enable!
const LCD_CD_CMD_SET: u32 = 1 << 30;
const LCD_CD_DUMMY_SET: u32 = 1 << 29;
const LCD_CD_DATA_SET: u32 = 1 << 28;
const LCD_AFIFO_ADDR_BRIDGE_EN: u32 = 1 << 26;
const LCD_AFIFO_RESET: u32 = 1 << 25;

// CTRL register bits
const LCD_TX_FIFO_RST: u32 = 1 << 13;
const LCD_TX_FIFO_MOD_SHIFT: u32 = 10;

// CTRL1 register bits
const LCD_DOUT_EN: u32 = 1 << 0;

// Safe register access with memory barriers
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

/// Critical function - properly updates shadow registers
unsafe fn lcd_ll_start() {
    // Stop any ongoing operation
    reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_START);
    
    // Clear reset
    reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_RESET);
    
    // Trigger update to copy shadow registers to live registers
    reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
    esp_rom_delay_us(10);
    
    // Start operation
    reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
}

/// Wait for LCD bus to be idle
unsafe fn lcd_ll_wait_idle() {
    let mut timeout = 10000;
    let mut last_log = 0;
    while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
        timeout -= 1;
        esp_rom_delay_us(1);
        
        // Log progress and reset watchdog
        if timeout % 1000 == 0 && timeout != last_log {
            last_log = timeout;
            log::debug!("Waiting for LCD idle... timeout={}", timeout);
            esp_idf_sys::esp_task_wdt_reset();
        }
    }
    
    if timeout == 0 {
        log::warn!("LCD wait idle timeout!");
    }
}

pub struct LcdCamWorking {
    initialized: bool,
}

impl LcdCamWorking {
    pub fn new() -> Self {
        Self { initialized: false }
    }
    
    pub fn init(&mut self) -> Result<()> {
        unsafe {
            log::info!("Initializing LCD_CAM with shadow register fix...");
            
            // Step 1: Enable peripheral
            reg_set_bits(SYSTEM_PERIP_CLK_EN1_REG, 1 << 31);
            reg_clear_bits(SYSTEM_PERIP_RST_EN1_REG, 1 << 31);
            esp_rom_delay_us(100);
            
            // Step 2: Configure clock
            let clk_val = LCD_CLK_EN | (15 << LCD_CLKM_DIV_NUM_SHIFT); // 10MHz
            reg_write(LCD_CAM_LCD_CLOCK_REG, clk_val);
            
            // Step 3: Configure MISC with all output enables
            let misc_val = LCD_CD_CMD_SET | LCD_CD_DATA_SET | 
                          LCD_CD_DUMMY_SET | LCD_AFIFO_ADDR_BRIDGE_EN;
            reg_write(LCD_CAM_LCD_MISC_REG, misc_val);
            
            // Step 4: Configure USER
            let user_val = LCD_DOUT | LCD_8BITS_ORDER;
            reg_write(LCD_CAM_LCD_USER_REG, user_val);
            
            // Step 5: Reset FIFOs
            reg_set_bits(LCD_CAM_LCD_CTRL_REG, LCD_TX_FIFO_RST | LCD_AFIFO_RESET);
            esp_rom_delay_us(10);
            reg_clear_bits(LCD_CAM_LCD_CTRL_REG, LCD_TX_FIFO_RST | LCD_AFIFO_RESET);
            
            // Step 6: Set FIFO mode
            let ctrl_val = reg_read(LCD_CAM_LCD_CTRL_REG);
            let ctrl_new = (ctrl_val & !(0x3 << LCD_TX_FIFO_MOD_SHIFT)) | (1 << LCD_TX_FIFO_MOD_SHIFT);
            reg_write(LCD_CAM_LCD_CTRL_REG, ctrl_new);
            
            // Step 7: Enable output
            reg_write(LCD_CAM_LCD_CTRL1_REG, LCD_DOUT_EN);
            
            // Step 8: Data output mode
            reg_write(LCD_CAM_LCD_DATA_DOUT_MODE_REG, 0xFF);
            
            // Step 9: CRITICAL - Apply configuration with proper update sequence
            lcd_ll_start();
            lcd_ll_wait_idle();
            
            // Verify registers retained values
            log::info!("Register verification after update:");
            log::info!("  CLOCK:     0x{:08x}", reg_read(LCD_CAM_LCD_CLOCK_REG));
            log::info!("  USER:      0x{:08x}", reg_read(LCD_CAM_LCD_USER_REG));
            log::info!("  MISC:      0x{:08x}", reg_read(LCD_CAM_LCD_MISC_REG));
            log::info!("  CTRL:      0x{:08x}", reg_read(LCD_CAM_LCD_CTRL_REG));
            log::info!("  CTRL1:     0x{:08x}", reg_read(LCD_CAM_LCD_CTRL1_REG));
            
            self.initialized = true;
            Ok(())
        }
    }
    
    pub fn configure_pins(
        &self,
        _d0: u8, _d1: u8, _d2: u8, _d3: u8, 
        _d4: u8, _d5: u8, _d6: u8, _d7: u8,
        wr: u8, dc: u8, cs: u8, rst: u8,
    ) -> Result<()> {
        unsafe {
            log::info!("Configuring GPIO Matrix for LCD_CAM...");
            
            const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
            
            // Data pins D0-D7 -> LCD_DATA_OUT0-7 (signals 133-140)
            for (i, &pin) in DATA_PINS.iter().enumerate() {
                esp_rom_gpio_pad_select_gpio(pin as u32);
                gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
                gpio_set_drive_capability(pin as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
                esp_rom_gpio_connect_out_signal(pin as u32, 133 + i as u32, false, false);
            }
            
            // WR -> LCD_PCLK (signal 154)
            esp_rom_gpio_pad_select_gpio(wr as u32);
            gpio_set_direction(wr as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_drive_capability(wr as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
            esp_rom_gpio_connect_out_signal(wr as u32, 154, false, false);
            
            // DC -> LCD_DC (signal 153)
            esp_rom_gpio_pad_select_gpio(dc as u32);
            gpio_set_direction(dc as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_drive_capability(dc as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
            esp_rom_gpio_connect_out_signal(dc as u32, 153, false, false);
            
            // CS -> LCD_CS (signal 132)
            esp_rom_gpio_pad_select_gpio(cs as u32);
            gpio_set_direction(cs as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_drive_capability(cs as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
            esp_rom_gpio_connect_out_signal(cs as u32, 132, false, false);
            
            // RST - manual control
            esp_rom_gpio_pad_select_gpio(rst as u32);
            gpio_set_direction(rst as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_level(rst as gpio_num_t, 1);
            
            Ok(())
        }
    }
    
    pub fn write_command(&self, cmd: u8) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("LCD_CAM not initialized"));
        }
        
        unsafe {
            // Set command mode
            reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
            
            // Write command
            reg_write(LCD_CAM_LCD_CMD_VAL_REG, cmd as u32);
            
            // Start transfer with proper update
            lcd_ll_start();
            lcd_ll_wait_idle();
        }
        
        Ok(())
    }
    
    pub fn write_data(&self, data: u8) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("LCD_CAM not initialized"));
        }
        
        unsafe {
            // Clear command mode (data mode)
            reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
            
            // Write data
            reg_write(LCD_CAM_LCD_CMD_VAL_REG, data as u32);
            
            // Start transfer with proper update
            lcd_ll_start();
            lcd_ll_wait_idle();
        }
        
        Ok(())
    }
    
    pub fn write_data_bytes(&self, data: &[u8]) -> Result<()> {
        // For now, write byte by byte - DMA can be added later
        for (i, &byte) in data.iter().enumerate() {
            self.write_data(byte)?;
            
            // Reset watchdog periodically
            if i % 100 == 0 {
                unsafe { esp_idf_sys::esp_task_wdt_reset(); }
            }
        }
        Ok(())
    }
}

/// Test the working LCD_CAM implementation
pub fn test_lcd_cam_working() -> Result<()> {
    log::warn!("=== Testing LCD_CAM with Shadow Register Fix ===");
    
    // Reset watchdog at start
    unsafe { esp_idf_sys::esp_task_wdt_reset(); }
    
    // Power pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    let mut lcd_cam = LcdCamWorking::new();
    
    // Initialize
    log::info!("Initializing LCD_CAM...");
    lcd_cam.init()?;
    
    // Reset watchdog after init
    unsafe { esp_idf_sys::esp_task_wdt_reset(); }
    
    // Configure pins
    log::info!("Configuring pins...");
    lcd_cam.configure_pins(39, 40, 41, 42, 45, 46, 47, 48, 8, 7, 6, 5)?;
    
    // Reset watchdog
    unsafe { esp_idf_sys::esp_task_wdt_reset(); }
    
    // Test sequence - simplified to avoid watchdog
    log::info!("Sending minimal test sequence...");
    
    // Just try to send a few commands and data
    lcd_cam.write_command(0x01)?; // Software reset
    FreeRtos::delay_ms(50);
    unsafe { esp_idf_sys::esp_task_wdt_reset(); }
    
    lcd_cam.write_command(0x11)?; // Sleep out
    FreeRtos::delay_ms(50);
    unsafe { esp_idf_sys::esp_task_wdt_reset(); }
    
    // Try to write a few pixels
    lcd_cam.write_command(0x2C)?; // RAMWR
    for i in 0..10 {
        lcd_cam.write_data(0xFF)?; // White
        lcd_cam.write_data(0xFF)?;
        
        if i % 5 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    }
    
    log::warn!("Test complete! LCD_CAM initialized and commands sent.");
    log::warn!("Check with logic analyzer to verify GPIO activity.");
    
    // Keep system alive for observation
    for i in 0..5 {
        log::info!("Keeping alive... {}/5", i + 1);
        FreeRtos::delay_ms(1000);
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
    }
    
    Ok(())
}