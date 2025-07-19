/// LCD_CAM test optimized for logic analyzer debugging
/// Generates clear, predictable patterns for easy verification
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
const LCD_CAM_LCD_CTRL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x0C;
const LCD_CAM_LCD_CTRL1_REG: u32 = DR_REG_LCD_CAM_BASE + 0x10;
const LCD_CAM_LCD_CTRL2_REG: u32 = DR_REG_LCD_CAM_BASE + 0x14;
const LCD_CAM_LCD_CMD_VAL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x18;
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x34;

// Bit definitions
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;
const LCD_AFIFO_RST: u32 = 1 << 30;
const LCD_TX_FIFO_RST: u32 = 1 << 25;
const LCD_DOUT_EN: u32 = 1 << 0;

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

pub fn lcd_cam_logic_analyzer_test(
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
    log::warn!("=== LCD_CAM Logic Analyzer Test ===");
    log::warn!("Connect logic analyzer to:");
    log::warn!("  WR  = GPIO8  (expect: clock pulses)");
    log::warn!("  D0  = GPIO39 (expect: data bit 0)");
    log::warn!("  D7  = GPIO48 (expect: data bit 7)");
    log::warn!("  CS  = GPIO6  (expect: low during transfer)");
    log::warn!("  DC  = GPIO7  (expect: low=cmd, high=data)");
    
    // Power pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    log::info!("Phase 1: Manual GPIO test (baseline)");
    unsafe {
        // Configure pins for manual control first
        const PINS: [u8; 11] = [39, 40, 41, 42, 45, 46, 47, 48, 8, 7, 6];
        for &pin in &PINS {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_drive_capability(pin as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        }
        
        // Generate 10 manual WR pulses with counting pattern
        log::info!("Generating 10 manual pulses (should see on analyzer)...");
        for i in 0..10 {
            // Set data pattern (counting)
            gpio_set_level(39, (i & 0x01) as u32);      // D0
            gpio_set_level(40, ((i >> 1) & 0x01) as u32); // D1
            gpio_set_level(41, ((i >> 2) & 0x01) as u32); // D2
            gpio_set_level(42, ((i >> 3) & 0x01) as u32); // D3
            gpio_set_level(45, ((i >> 4) & 0x01) as u32); // D4
            gpio_set_level(46, ((i >> 5) & 0x01) as u32); // D5
            gpio_set_level(47, ((i >> 6) & 0x01) as u32); // D6
            gpio_set_level(48, ((i >> 7) & 0x01) as u32); // D7
            
            gpio_set_level(6, 0);  // CS low
            gpio_set_level(7, 1);  // DC high (data)
            
            // WR pulse
            gpio_set_level(8, 1);  // WR high
            esp_rom_delay_us(1);
            gpio_set_level(8, 0);  // WR low
            esp_rom_delay_us(1);
            gpio_set_level(8, 1);  // WR high
            
            esp_rom_delay_us(10);
        }
        
        gpio_set_level(6, 1);  // CS high
        
        log::info!("Manual pulses complete - verify on analyzer");
        FreeRtos::delay_ms(1000);
        esp_task_wdt_reset();
    }
    
    log::info!("Phase 2: LCD_CAM setup");
    unsafe {
        // Enable peripheral
        let val = reg_read(SYSTEM_PERIP_CLK_EN1_REG);
        reg_write(SYSTEM_PERIP_CLK_EN1_REG, val | (1 << 31));
        
        let val = reg_read(SYSTEM_PERIP_RST_EN1_REG);
        reg_write(SYSTEM_PERIP_RST_EN1_REG, val & !(1 << 31));
        
        esp_rom_delay_us(100);
        
        // Connect pins to LCD_CAM
        const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_connect_out_signal(pin as u32, 133 + i as u32, false, false);
        }
        
        esp_rom_gpio_connect_out_signal(8, 154, false, false);  // WR -> LCD_PCLK
        esp_rom_gpio_connect_out_signal(7, 153, false, false);  // DC -> LCD_DC
        esp_rom_gpio_connect_out_signal(6, 132, false, false);  // CS -> LCD_CS
        
        // Set clock (slow for analyzer)
        reg_write(LCD_CAM_LCD_CLOCK_REG, LCD_CLK_EN | 79); // ~1MHz for easy capture
        
        // Enable output
        reg_write(LCD_CAM_LCD_CTRL1_REG, LCD_DOUT_EN);
        reg_write(LCD_CAM_LCD_DATA_DOUT_MODE_REG, 0xFF);
        
        // Reset FIFO
        let ctrl = reg_read(LCD_CAM_LCD_CTRL_REG);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl | LCD_AFIFO_RST | LCD_TX_FIFO_RST);
        esp_rom_delay_us(10);
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl & !(LCD_AFIFO_RST | LCD_TX_FIFO_RST) | (1 << 22));
        
        // Configure user register
        reg_write(LCD_CAM_LCD_USER_REG, LCD_DOUT | LCD_8BITS_ORDER);
        reg_write(LCD_CAM_LCD_USER_REG, LCD_DOUT | LCD_8BITS_ORDER | LCD_UPDATE);
        esp_rom_delay_us(10);
        reg_write(LCD_CAM_LCD_USER_REG, LCD_DOUT | LCD_8BITS_ORDER);
    }
    
    log::info!("Phase 3: LCD_CAM single byte test");
    unsafe {
        // Send single byte 0xAA
        log::info!("Sending 0xAA via LCD_CAM...");
        
        esp_task_wdt_reset();
        
        // Data mode
        let user = reg_read(LCD_CAM_LCD_USER_REG);
        reg_write(LCD_CAM_LCD_USER_REG, user & !LCD_CMD);
        
        // Write data
        reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0xAA);
        
        // Start
        reg_write(LCD_CAM_LCD_USER_REG, (user & !LCD_CMD) | LCD_START);
        
        // Wait
        let mut timeout = 100000;
        while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
            timeout -= 1;
        }
        
        if timeout == 0 {
            log::error!("LCD_CAM transfer timeout!");
        } else {
            log::info!("LCD_CAM transfer complete");
        }
        
        FreeRtos::delay_ms(500);
    }
    
    log::info!("Phase 4: LCD_CAM burst test");
    unsafe {
        log::info!("Sending counting pattern 0x00 to 0x0F...");
        
        for i in 0..16u8 {
            esp_task_wdt_reset();
            
            // Data mode
            let user = reg_read(LCD_CAM_LCD_USER_REG);
            reg_write(LCD_CAM_LCD_USER_REG, user & !LCD_CMD);
            
            // Write data
            reg_write(LCD_CAM_LCD_CMD_VAL_REG, i as u32);
            
            // Start
            reg_write(LCD_CAM_LCD_USER_REG, (user & !LCD_CMD) | LCD_START);
            
            // Wait briefly
            let mut timeout = 10000;
            while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
                timeout -= 1;
            }
            
            esp_rom_delay_us(100); // Gap between bytes
        }
        
        log::info!("Burst complete");
    }
    
    log::info!("Phase 5: Command vs Data test");
    unsafe {
        log::info!("Testing DC pin control...");
        
        // Send command 0x01 (DC should go low)
        let user = reg_read(LCD_CAM_LCD_USER_REG);
        reg_write(LCD_CAM_LCD_USER_REG, user | LCD_CMD); // Command mode
        reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0x01);
        reg_write(LCD_CAM_LCD_USER_REG, (user | LCD_CMD) | LCD_START);
        
        let mut timeout = 10000;
        while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
            timeout -= 1;
        }
        
        esp_rom_delay_us(100);
        
        // Send data 0xFF (DC should go high)
        reg_write(LCD_CAM_LCD_USER_REG, user & !LCD_CMD); // Data mode
        reg_write(LCD_CAM_LCD_CMD_VAL_REG, 0xFF);
        reg_write(LCD_CAM_LCD_USER_REG, (user & !LCD_CMD) | LCD_START);
        
        timeout = 10000;
        while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 && timeout > 0 {
            timeout -= 1;
        }
        
        log::info!("Command/Data test complete");
    }
    
    log::info!("=== Test Complete ===");
    log::info!("Expected on logic analyzer:");
    log::info!("1. Manual phase: 10 WR pulses with counting data");
    log::info!("2. LCD_CAM single: 1 WR pulse with data 0xAA");
    log::info!("3. LCD_CAM burst: 16 WR pulses with data 0x00-0x0F");
    log::info!("4. DC test: DC low for cmd, high for data");
    log::info!("");
    log::info!("If you see manual pulses but no LCD_CAM pulses,");
    log::info!("the peripheral isn't outputting to pins.");
    
    // Keep running to allow measurement
    loop {
        unsafe { esp_task_wdt_reset(); }
        FreeRtos::delay_ms(1000);
    }
}