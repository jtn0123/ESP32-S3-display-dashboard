// Display module - main interface for LCD operations

use esp_hal::{
    gpio::{GpioPin, Output, PinDriver},
    peripherals::LCD_CAM,
    dma::DmaChannel0,
};

pub struct DisplayPins {
    pub d0: GpioPin<39>,
    pub d1: GpioPin<40>,
    pub d2: GpioPin<41>,
    pub d3: GpioPin<42>,
    pub d4: GpioPin<45>,
    pub d5: GpioPin<46>,
    pub d6: GpioPin<47>,
    pub d7: GpioPin<48>,
    pub wr: GpioPin<8>,
}

// Color constants (BGR565 format)
#[derive(Debug, Clone, Copy)]
pub struct Color(pub u16);

impl Color {
    pub const BLACK: Color = Color(0xFFFF);
    pub const WHITE: Color = Color(0x0000);
    pub const RED: Color = Color(0x07FF);
    pub const GREEN: Color = Color(0xF81F);
    pub const BLUE: Color = Color(0xF8E0);
    pub const YELLOW: Color = Color(0x001F);
    pub const CYAN: Color = Color(0xF800);
    pub const MAGENTA: Color = Color(0x07E0);
    pub const PRIMARY_BLUE: Color = Color(0x2589);
}

pub struct Display {
    framebuffer: [u16; 320 * 170],
    initialized: bool,
}

impl Display {
    pub fn new(
        lcd_cam: LCD_CAM,
        dma_channel: DmaChannel0,
        pins: DisplayPins,
        dc_pin: PinDriver<'static, GpioPin<7>, Output>,
        cs_pin: PinDriver<'static, GpioPin<6>, Output>,
        rst_pin: PinDriver<'static, GpioPin<5>, Output>,
        mut backlight_pin: PinDriver<'static, GpioPin<38>, Output>,
    ) -> Result<Self, &'static str> {
        // Turn on backlight
        backlight_pin.set_high().ok();
        
        // TODO: Initialize LCD_CAM with esp-hal
        // For now, create a simple framebuffer display
        
        Ok(Display {
            framebuffer: [0; 320 * 170],
            initialized: true,
        })
    }
    
    pub fn clear(&mut self, color: Color) {
        for pixel in self.framebuffer.iter_mut() {
            *pixel = color.0;
        }
    }
    
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x < 320 && y < 170 {
            self.framebuffer[(y as usize * 320) + x as usize] = color.0;
        }
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }
    
    pub fn draw_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        // Top and bottom
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h - 1, w, 1, color);
        // Left and right
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w - 1, y, 1, h, color);
    }
    
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: Color) {
        // Simple text rendering placeholder
        // In production, use a proper font library
        let mut cursor_x = x;
        for ch in text.chars() {
            // Draw character placeholder (6x8 block)
            for dy in 0..8 {
                for dx in 0..6 {
                    if dx == 0 || dx == 5 || dy == 0 || dy == 7 {
                        self.set_pixel(cursor_x + dx, y + dy, color);
                    }
                }
            }
            cursor_x += 8;
        }
    }
    
    pub fn draw_number(&mut self, x: u16, y: u16, num: u32, color: Color) {
        self.draw_text(x, y, &num.to_string(), color);
    }
    
    pub fn draw_card(&mut self, x: u16, y: u16, w: u16, h: u16, title: &str, border_color: Color) {
        // Shadow
        self.fill_rect(x + 2, y + 2, w, h, Color(0x2104));
        
        // Main card
        self.fill_rect(x, y, w, h, Color::BLACK);
        
        // Border
        self.draw_rect(x, y, w, h, border_color);
        
        // Title
        if !title.is_empty() {
            self.draw_text(x + 5, y + 2, title, Color::WHITE);
        }
    }
    
    pub async fn flush(&mut self) {
        // TODO: Implement DMA transfer
        // For now, this is a no-op
    }
}