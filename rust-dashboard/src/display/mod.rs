// Display module - main interface for LCD operations

pub mod font;

use esp_hal::{
    gpio::{AnyPin, Input, Output, PinDriver},
    peripherals::LCD_CAM,
    dma::Channel0,
};
use esp_println::println;

pub use font::{Font5x7, FontRenderer};

pub struct DisplayPins {
    pub d0: AnyPin,
    pub d1: AnyPin,
    pub d2: AnyPin,
    pub d3: AnyPin,
    pub d4: AnyPin,
    pub d5: AnyPin,
    pub d6: AnyPin,
    pub d7: AnyPin,
    pub wr: AnyPin,
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
    framebuffer: &'static mut [u16; 320 * 170],
    initialized: bool,
}

impl Display {
    pub fn new(
        lcd_cam: LCD_CAM,
        dma_channel: Channel0,
        pins: DisplayPins,
        mut dc_pin: PinDriver<'static, AnyPin, Output>,
        mut cs_pin: PinDriver<'static, AnyPin, Output>,
        mut rst_pin: PinDriver<'static, AnyPin, Output>,
        mut backlight_pin: PinDriver<'static, AnyPin, Output>,
    ) -> Result<Self, &'static str> {
        // Turn on backlight
        backlight_pin.set_high().ok();
        
        // Reset display
        rst_pin.set_high().ok();
        esp_hal::delay::Delay::new_default().delay_millis(10);
        rst_pin.set_low().ok();
        esp_hal::delay::Delay::new_default().delay_millis(10);
        rst_pin.set_high().ok();
        esp_hal::delay::Delay::new_default().delay_millis(120);
        
        // TODO: Initialize LCD_CAM with esp-hal
        // For now, create a simple framebuffer display
        
        // Allocate framebuffer statically
        static mut FRAMEBUFFER: [u16; 320 * 170] = [0; 320 * 170];
        let framebuffer = unsafe { &mut FRAMEBUFFER };
        
        println!("Display initialized");
        
        Ok(Display {
            framebuffer,
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
    
    // Legacy text drawing (replaced by FontRenderer trait)
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: Color) {
        // Use the new font system
        self.draw_text_5x7(x, y, text, color);
    }
    
    pub fn draw_number(&mut self, x: u16, y: u16, num: u32, color: Color) {
        // Convert number to string and draw
        let mut buffer = [0u8; 10];
        let text = num_to_str(num, &mut buffer);
        self.draw_text(x, y, text, color);
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
    
    // Graphics primitives
    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: Color) {
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
    
    pub fn draw_circle(&mut self, cx: u16, cy: u16, radius: u16, color: Color) {
        // Midpoint circle algorithm
        let mut x = radius as i32;
        let mut y = 0i32;
        let mut err = 0i32;
        
        while x >= y {
            self.set_pixel((cx as i32 + x) as u16, (cy as i32 + y) as u16, color);
            self.set_pixel((cx as i32 + y) as u16, (cy as i32 + x) as u16, color);
            self.set_pixel((cx as i32 - y) as u16, (cy as i32 + x) as u16, color);
            self.set_pixel((cx as i32 - x) as u16, (cy as i32 + y) as u16, color);
            self.set_pixel((cx as i32 - x) as u16, (cy as i32 - y) as u16, color);
            self.set_pixel((cx as i32 - y) as u16, (cy as i32 - x) as u16, color);
            self.set_pixel((cx as i32 + y) as u16, (cy as i32 - x) as u16, color);
            self.set_pixel((cx as i32 + x) as u16, (cy as i32 - y) as u16, color);
            
            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }
}

// Helper function to convert number to string
fn num_to_str(mut num: u32, buffer: &mut [u8]) -> &str {
    if num == 0 {
        buffer[0] = b'0';
        return unsafe { core::str::from_utf8_unchecked(&buffer[..1]) };
    }
    
    let mut i = 0;
    while num > 0 && i < buffer.len() {
        buffer[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    
    // Reverse the digits
    buffer[..i].reverse();
    unsafe { core::str::from_utf8_unchecked(&buffer[..i]) }
}