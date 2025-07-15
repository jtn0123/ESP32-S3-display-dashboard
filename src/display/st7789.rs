// ST7789 display controller driver
// Handles initialization and command interface

use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Output, PinDriver};
use esp_idf_hal::delay::FreeRtos;
use anyhow::Result;
use log::*;

use super::lcd_cam::LcdCam;

pub struct ST7789 {
    dc_pin: PinDriver<'static, Gpio7, Output>,
    cs_pin: PinDriver<'static, Gpio6, Output>, 
    rst_pin: PinDriver<'static, Gpio5, Output>,
}

// ST7789 Commands
const CMD_NOP: u8 = 0x00;
const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
const CMD_COLMOD: u8 = 0x3A;
const CMD_MADCTL: u8 = 0x36;
const CMD_CASET: u8 = 0x2A;
const CMD_RASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_DISPON: u8 = 0x29;

// MADCTL bits
const MADCTL_MY: u8 = 0x80;  // Row order
const MADCTL_MX: u8 = 0x40;  // Column order
const MADCTL_MV: u8 = 0x20;  // Row/Column exchange
const MADCTL_ML: u8 = 0x10;  // Vertical refresh order
const MADCTL_RGB: u8 = 0x00; // RGB order
const MADCTL_BGR: u8 = 0x08; // BGR order

impl ST7789 {
    pub fn new(
        dc_pin: PinDriver<'static, Gpio7, Output>,
        cs_pin: PinDriver<'static, Gpio6, Output>,
        rst_pin: PinDriver<'static, Gpio5, Output>,
        lcd_cam: &LcdCam,
    ) -> Result<Self> {
        let mut st7789 = ST7789 {
            dc_pin,
            cs_pin,
            rst_pin,
        };
        
        st7789.init()?;
        
        Ok(st7789)
    }
    
    fn init(&mut self) -> Result<()> {
        info!("Initializing ST7789 display controller");
        
        // Hardware reset
        self.rst_pin.set_high()?;
        FreeRtos::delay_ms(10);
        self.rst_pin.set_low()?;
        FreeRtos::delay_ms(10);
        self.rst_pin.set_high()?;
        FreeRtos::delay_ms(120);
        
        // Software reset
        self.write_command(CMD_SWRESET)?;
        FreeRtos::delay_ms(150);
        
        // Sleep out
        self.write_command(CMD_SLPOUT)?;
        FreeRtos::delay_ms(120);
        
        // Color mode - 16-bit RGB565
        self.write_command(CMD_COLMOD)?;
        self.write_data(&[0x55])?; // 16-bit color
        
        // Memory data access control
        // Configure for your specific display orientation
        self.write_command(CMD_MADCTL)?;
        self.write_data(&[MADCTL_MX | MADCTL_BGR])?; // Adjust based on your display
        
        // Set display window to full screen (320x170)
        self.set_window(0, 0, 319, 169)?;
        
        // Display on
        self.write_command(CMD_DISPON)?;
        
        info!("ST7789 initialization complete");
        
        Ok(())
    }
    
    pub fn write_command(&mut self, cmd: u8) -> Result<()> {
        self.cs_pin.set_low()?;
        self.dc_pin.set_low()?; // Command mode
        
        // For commands, we need to send single byte
        // This is tricky with LCD_CAM - might need GPIO fallback for commands
        // TODO: Implement command sending via LCD_CAM or GPIO
        
        self.cs_pin.set_high()?;
        Ok(())
    }
    
    pub fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.cs_pin.set_low()?;
        self.dc_pin.set_high()?; // Data mode
        
        // For data, we can use LCD_CAM DMA
        // TODO: Trigger LCD_CAM transfer for data bytes
        
        self.cs_pin.set_high()?;
        Ok(())
    }
    
    pub fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<()> {
        // Column address set
        self.write_command(CMD_CASET)?;
        self.write_data(&[
            (x0 >> 8) as u8,
            (x0 & 0xFF) as u8,
            (x1 >> 8) as u8,
            (x1 & 0xFF) as u8,
        ])?;
        
        // Row address set
        self.write_command(CMD_RASET)?;
        self.write_data(&[
            (y0 >> 8) as u8,
            (y0 & 0xFF) as u8,
            (y1 >> 8) as u8,
            (y1 & 0xFF) as u8,
        ])?;
        
        // Memory write
        self.write_command(CMD_RAMWR)?;
        
        Ok(())
    }
}

// Note: The command/data sending needs special handling since LCD_CAM
// is optimized for bulk data transfers. We might need to:
// 1. Use GPIO bit-banging for commands (slow but works)
// 2. Configure LCD_CAM for single-byte transfers (complex)
// 3. Use a hybrid approach