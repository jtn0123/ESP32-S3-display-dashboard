/// LCD_CAM bulk transfer test - optimized pixel drawing
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use super::lcd_cam_hal::LcdCamHal;
use super::colors::{BLACK, WHITE, PRIMARY_RED, PRIMARY_BLUE, PRIMARY_GREEN, rgb565};
use core::ptr::write_volatile;
use core::sync::atomic::{compiler_fence, Ordering};

// GPIO Matrix signal indices
const LCD_DATA_OUT_IDX_BASE: u32 = 133;
const LCD_PCLK_IDX: u32 = 154;
const LCD_DC_IDX: u32 = 153;
const LCD_CS_IDX: u32 = 132;

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

// Display dimensions
const DISPLAY_WIDTH: u16 = 300;
const DISPLAY_HEIGHT: u16 = 168;
const DISPLAY_X_OFFSET: u16 = 10;
const DISPLAY_Y_OFFSET: u16 = 36;

// LCD_CAM registers for direct access
const DR_REG_LCD_CAM_BASE: u32 = 0x6004_1000;
const LCD_CAM_LCD_USER_REG: u32 = DR_REG_LCD_CAM_BASE + 0x04;
const LCD_CAM_LCD_CTRL2_REG: u32 = DR_REG_LCD_CAM_BASE + 0x14;
const LCD_CAM_LCD_CMD_VAL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x18;

// LCD user register bits
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;

pub fn lcd_cam_bulk_test(
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
    log::warn!("Starting LCD_CAM bulk transfer test...");
    
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
    
    // Configure GPIO matrix
    unsafe {
        for (i, &pin) in pins.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            esp_rom_gpio_connect_out_signal(pin as u32, LCD_DATA_OUT_IDX_BASE + i as u32, false, false);
        }
        
        esp_rom_gpio_pad_select_gpio(pin_wr as u32);
        gpio_set_direction(pin_wr as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(pin_wr as u32, LCD_PCLK_IDX, false, false);
        
        esp_rom_gpio_pad_select_gpio(pin_dc as u32);
        gpio_set_direction(pin_dc as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(pin_dc as u32, LCD_DC_IDX, false, false);
        
        esp_rom_gpio_pad_select_gpio(pin_cs as u32);
        gpio_set_direction(pin_cs as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(pin_cs as u32, LCD_CS_IDX, false, false);
        
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
    
    // Initialize LCD_CAM
    unsafe {
        LcdCamHal::init().map_err(|e| anyhow::anyhow!("{}", e))?;
        LcdCamHal::configure_i8080_8bit(40_000_000).map_err(|e| anyhow::anyhow!("{}", e))?; // 40 MHz
    }
    
    // Initialize ST7789
    log::info!("Initializing ST7789 display...");
    unsafe {
        // Software reset
        LcdCamHal::send_command(CMD_SWRESET).map_err(|e| anyhow::anyhow!("{}", e))?;
        FreeRtos::delay_ms(150);
        
        // Sleep out
        LcdCamHal::send_command(CMD_SLPOUT).map_err(|e| anyhow::anyhow!("{}", e))?;
        FreeRtos::delay_ms(120);
        
        // Memory access control (landscape mode)
        LcdCamHal::send_command(CMD_MADCTL).map_err(|e| anyhow::anyhow!("{}", e))?;
        LcdCamHal::send_data(&[0x60]).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Pixel format (16-bit RGB565)
        LcdCamHal::send_command(CMD_COLMOD).map_err(|e| anyhow::anyhow!("{}", e))?;
        LcdCamHal::send_data(&[0x55]).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Inversion on
        LcdCamHal::send_command(CMD_INVON).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Display on
        LcdCamHal::send_command(CMD_DISPON).map_err(|e| anyhow::anyhow!("{}", e))?;
        FreeRtos::delay_ms(20);
    }
    
    log::info!("ST7789 initialized, starting bulk transfer test...");
    
    // Helper to write register with barriers
    #[inline(always)]
    unsafe fn reg_write(addr: u32, val: u32) {
        compiler_fence(Ordering::SeqCst);
        write_volatile(addr as *mut u32, val);
        compiler_fence(Ordering::SeqCst);
    }
    
    // Helper to read register
    #[inline(always)]
    unsafe fn reg_read(addr: u32) -> u32 {
        compiler_fence(Ordering::SeqCst);
        let val = core::ptr::read_volatile(addr as *const u32);
        compiler_fence(Ordering::SeqCst);
        val
    }
    
    // Helper to set bits
    #[inline(always)]
    unsafe fn reg_set_bits(addr: u32, mask: u32) {
        let val = reg_read(addr);
        reg_write(addr, val | mask);
    }
    
    // Helper to clear bits
    #[inline(always)]
    unsafe fn reg_clear_bits(addr: u32, mask: u32) {
        let val = reg_read(addr);
        reg_write(addr, val & !mask);
    }
    
    // Optimized bulk data transfer
    unsafe fn send_bulk_data(data: &[u16]) -> Result<()> {
        // Clear command mode (DC = 1 for data)
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
        
        // Send data in 32-bit chunks for efficiency
        let mut i = 0;
        while i < data.len() {
            if i + 1 < data.len() {
                // Send two 16-bit pixels as one 32-bit write
                let pixel1 = data[i];
                let pixel2 = data[i + 1];
                let combined = ((pixel2 as u32) << 16) | (pixel1 as u32);
                
                // Write to CMD_VAL register (32-bit capable)
                reg_write(LCD_CAM_LCD_CMD_VAL_REG, combined);
                
                // Trigger transfer
                reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
                
                // Wait for completion
                while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
                    core::hint::spin_loop();
                }
                
                i += 2;
            } else {
                // Last pixel if odd count
                reg_write(LCD_CAM_LCD_CTRL2_REG, data[i] as u32);
                reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
                while (reg_read(LCD_CAM_LCD_USER_REG) & LCD_START) != 0 {
                    core::hint::spin_loop();
                }
                i += 1;
            }
        }
        
        Ok(())
    }
    
    // Helper function to set drawing window
    unsafe fn set_window(x0: u16, y0: u16, x1: u16, y1: u16) -> Result<()> {
        // Column address set
        LcdCamHal::send_command(CMD_CASET).map_err(|e| anyhow::anyhow!("{}", e))?;
        let x0_off = x0 + DISPLAY_X_OFFSET;
        let x1_off = x1 + DISPLAY_X_OFFSET;
        LcdCamHal::send_data(&[
            (x0_off >> 8) as u8,
            (x0_off & 0xFF) as u8,
            (x1_off >> 8) as u8,
            (x1_off & 0xFF) as u8,
        ]).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Row address set
        LcdCamHal::send_command(CMD_RASET).map_err(|e| anyhow::anyhow!("{}", e))?;
        let y0_off = y0 + DISPLAY_Y_OFFSET;
        let y1_off = y1 + DISPLAY_Y_OFFSET;
        LcdCamHal::send_data(&[
            (y0_off >> 8) as u8,
            (y0_off & 0xFF) as u8,
            (y1_off >> 8) as u8,
            (y1_off & 0xFF) as u8,
        ]).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        Ok(())
    }
    
    // Helper function to fill rectangle with bulk transfer
    unsafe fn fill_rect_bulk(x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        set_window(x, y, x + w - 1, y + h - 1)?;
        
        // Start memory write
        LcdCamHal::send_command(CMD_RAMWR).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Create buffer of pixels
        let pixels = (w as usize) * (h as usize);
        const CHUNK_SIZE: usize = 256;
        let mut buffer = [color; CHUNK_SIZE];
        
        let mut remaining = pixels;
        while remaining > 0 {
            let chunk = remaining.min(CHUNK_SIZE);
            send_bulk_data(&buffer[..chunk])?;
            remaining -= chunk;
        }
        
        Ok(())
    }
    
    // Clear screen to black
    unsafe {
        log::info!("Clearing screen to black...");
        fill_rect_bulk(0, 0, DISPLAY_WIDTH, DISPLAY_HEIGHT, BLACK)?;
    }
    
    let mut frame_count = 0u32;
    let start_time = unsafe { esp_timer_get_time() };
    
    // Color test pattern
    loop {
        // Draw colored rectangles
        unsafe {
            // Red rectangle (top-left)
            fill_rect_bulk(10, 10, 100, 60, PRIMARY_RED)?;
            
            // Green rectangle (top-right)
            fill_rect_bulk(190, 10, 100, 60, PRIMARY_GREEN)?;
            
            // Blue rectangle (bottom-left)
            fill_rect_bulk(10, 98, 100, 60, PRIMARY_BLUE)?;
            
            // White rectangle (bottom-right)
            fill_rect_bulk(190, 98, 100, 60, WHITE)?;
            
            // Center rectangle cycles through colors
            let center_color = match frame_count % 4 {
                0 => rgb565(255, 255, 0),  // Yellow
                1 => rgb565(255, 0, 255),  // Magenta
                2 => rgb565(0, 255, 255),  // Cyan
                _ => rgb565(128, 128, 128), // Gray
            };
            fill_rect_bulk(100, 54, 100, 60, center_color)?;
        }
        
        frame_count += 1;
        
        // Report performance every 10 frames
        if frame_count % 10 == 0 {
            let elapsed_us = unsafe { esp_timer_get_time() } - start_time;
            let fps = (frame_count as i64 * 1_000_000) / elapsed_us;
            log::info!("LCD_CAM bulk test: {} frames, {} FPS", frame_count, fps);
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