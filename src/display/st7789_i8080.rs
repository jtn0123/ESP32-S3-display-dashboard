// ST7789 controller driver for I8080 interface
// Handles initialization and command/data protocol

use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Output, PinDriver};
use esp_idf_hal::delay::FreeRtos;
use anyhow::Result;
use log::*;

use super::lcd_cam_i8080::I8080Display;

pub struct ST7789<'d> {
    display: I8080Display<'d>,
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
const MADCTL_MY: u8 = 0x80;
const MADCTL_MX: u8 = 0x40;
const MADCTL_MV: u8 = 0x20;
const MADCTL_ML: u8 = 0x10;
const MADCTL_BGR: u8 = 0x08;

impl<'d> ST7789<'d> {
    pub fn new(
        mut display: I8080Display<'d>,
        dc_pin: PinDriver<'static, Gpio7, Output>,
        cs_pin: PinDriver<'static, Gpio6, Output>,
        rst_pin: PinDriver<'static, Gpio5, Output>,
    ) -> Result<Self> {
        let mut st7789 = ST7789 {
            display,
            dc_pin,
            cs_pin,
            rst_pin,
        };
        
        st7789.init()?;
        Ok(st7789)
    }
    
    fn init(&mut self) -> Result<()> {
        info!("Initializing ST7789 display");
        
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
        self.write_data(&[0x55])?;
        
        // Memory data access control
        self.write_command(CMD_MADCTL)?;
        self.write_data(&[MADCTL_MX | MADCTL_BGR])?;
        
        // Set display window
        self.set_window(0, 0, 319, 169)?;
        
        // Display on
        self.write_command(CMD_DISPON)?;
        
        info!("ST7789 initialization complete");
        Ok(())
    }
    
    pub fn write_command(&mut self, cmd: u8) -> Result<()> {
        self.cs_pin.set_low()?;
        self.dc_pin.set_low()?; // Command mode
        
        // Send command via I8080
        self.display.send_command(cmd)?;
        
        self.cs_pin.set_high()?;
        Ok(())
    }
    
    pub fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.cs_pin.set_low()?;
        self.dc_pin.set_high()?; // Data mode
        
        // Send data via I8080
        self.display.send_data(data)?;
        
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
        
        Ok(())
    }
    
    pub fn start_pixels(&mut self) -> Result<()> {
        self.write_command(CMD_RAMWR)?;
        self.cs_pin.set_low()?;
        self.dc_pin.set_high()?;
        Ok(())
    }
    
    pub fn end_pixels(&mut self) -> Result<()> {
        self.cs_pin.set_high()?;
        Ok(())
    }
    
    pub fn update_screen(&mut self) -> Result<()> {
        // Set window to full screen
        self.set_window(0, 0, 319, 169)?;
        
        // Start pixel write
        self.write_command(CMD_RAMWR)?;
        
        // Switch to data mode for bulk transfer
        self.cs_pin.set_low()?;
        self.dc_pin.set_high()?;
        
        // Send framebuffer via DMA
        self.display.update_framebuffer()?;
        
        self.cs_pin.set_high()?;
        Ok(())
    }
    
    pub fn clear(&mut self, color: u16) -> Result<()> {
        self.display.clear(color);
        self.update_screen()
    }
    
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) {
        self.display.set_pixel(x, y, color);
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }
}

// Graphics helper functions
impl<'d> ST7789<'d> {
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16) {
        // Simple 8x8 font rendering
        // In production, use a proper font library
        let mut cursor_x = x;
        for ch in text.chars() {
            // Draw character (placeholder)
            self.fill_rect(cursor_x, y, 6, 8, color);
            cursor_x += 8;
        }
    }
    
    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) {
        // Bresenham's line algorithm
        let dx = (x1 as i32 - x0 as i32).abs();
        let dy = (y1 as i32 - y0 as i32).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0 as i32;
        let mut y = y0 as i32;
        
        loop {
            self.set_pixel(x as u16, y as u16, color);
            
            if x == x1 as i32 && y == y1 as i32 {
                break;
            }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}