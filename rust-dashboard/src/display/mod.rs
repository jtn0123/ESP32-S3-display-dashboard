// Display module - coordinates LCD_CAM DMA driver with ST7789 display

pub mod lcd_cam;
pub mod st7789;
pub mod graphics;

use esp_idf_hal::gpio::{Gpio5, Gpio6, Gpio7, Output, PinDriver};
use esp_idf_hal::prelude::*;
use anyhow::Result;

use self::lcd_cam::LcdCam;
use self::st7789::ST7789;

pub struct Display {
    lcd_cam: LcdCam,
    controller: ST7789,
}

impl Display {
    pub fn new(
        dc_pin: PinDriver<'static, Gpio7, Output>,
        cs_pin: PinDriver<'static, Gpio6, Output>,
        rst_pin: PinDriver<'static, Gpio5, Output>,
    ) -> Result<Self> {
        // Initialize LCD_CAM peripheral
        let lcd_cam = unsafe { LcdCam::new()? };
        
        // Initialize ST7789 controller
        let controller = ST7789::new(dc_pin, cs_pin, rst_pin, &lcd_cam)?;
        
        Ok(Display {
            lcd_cam,
            controller,
        })
    }
    
    pub fn clear(&mut self, color: u16) {
        // Fill framebuffer with color
        let fb = self.lcd_cam.get_framebuffer_mut();
        for pixel in fb.iter_mut() {
            *pixel = color;
        }
        self.flush();
    }
    
    pub fn flush(&mut self) {
        // Trigger DMA transfer
        self.lcd_cam.start_transfer();
        self.lcd_cam.wait_transfer_done();
    }
    
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) {
        if x < 320 && y < 170 {
            let fb = self.lcd_cam.get_framebuffer_mut();
            fb[(y as usize * 320) + x as usize] = color;
        }
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        let fb = self.lcd_cam.get_framebuffer_mut();
        
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px < 320 && py < 170 {
                    fb[(py as usize * 320) + px as usize] = color;
                }
            }
        }
    }
}

// Color constants (BGR565 format for your display)
pub const COLOR_BLACK: u16 = 0xFFFF;
pub const COLOR_WHITE: u16 = 0x0000;
pub const COLOR_RED: u16 = 0x07FF;
pub const COLOR_GREEN: u16 = 0xF81F;
pub const COLOR_BLUE: u16 = 0xF8E0;