// display.rs - Safe Rust wrapper around C display driver

use std::ffi::CString;

// FFI bindings to C functions
#[link(name = "display_driver")]
extern "C" {
    fn display_init();
    fn display_set_brightness(brightness: u8);
    fn display_clear(color: u16);
    fn display_draw_pixel(x: u16, y: u16, color: u16);
    fn display_fill_rect(x: u16, y: u16, w: u16, h: u16, color: u16);
    fn display_draw_line(x0: u16, y0: u16, x1: u16, y1: u16, color: u16);
    fn display_draw_string(x: u16, y: u16, text: *const i8, color: u16);
    fn display_draw_string_transparent(x: u16, y: u16, text: *const i8, color: u16);
    fn display_update();
    fn display_flush();
}

// Color constants (matching your BGR format)
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
    
    // Your custom UI colors
    pub const PRIMARY_BLUE: Color = Color(0x2589);
    pub const PRIMARY_GREEN: Color = Color(0x07E5);
    pub const PRIMARY_RED: Color = Color(0xF800);
    pub const TEXT_PRIMARY: Color = Color(0xFFFF);
    pub const TEXT_SECONDARY: Color = Color(0xBDF7);
}

// Safe Rust display interface
pub struct Display {
    initialized: bool,
}

impl Display {
    pub fn new() -> Result<Self, &'static str> {
        unsafe { display_init(); }
        Ok(Display { initialized: true })
    }
    
    pub fn set_brightness(&mut self, brightness: u8) {
        unsafe { display_set_brightness(brightness); }
    }
    
    pub fn clear(&mut self, color: Color) {
        unsafe { display_clear(color.0); }
    }
    
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: Color) {
        unsafe { display_draw_pixel(x, y, color.0); }
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        unsafe { display_fill_rect(x, y, w, h, color.0); }
    }
    
    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: Color) {
        unsafe { display_draw_line(x0, y0, x1, y1, color.0); }
    }
    
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: Color) -> Result<(), &'static str> {
        let c_string = CString::new(text).map_err(|_| "Invalid string")?;
        unsafe { display_draw_string(x, y, c_string.as_ptr(), color.0); }
        Ok(())
    }
    
    pub fn draw_text_transparent(&mut self, x: u16, y: u16, text: &str, color: Color) -> Result<(), &'static str> {
        let c_string = CString::new(text).map_err(|_| "Invalid string")?;
        unsafe { display_draw_string_transparent(x, y, c_string.as_ptr(), color.0); }
        Ok(())
    }
    
    pub fn update(&mut self) {
        unsafe { display_update(); }
    }
    
    pub fn flush(&mut self) {
        unsafe { display_flush(); }
    }
}

// Higher-level drawing helpers
impl Display {
    pub fn draw_card(&mut self, x: u16, y: u16, w: u16, h: u16, title: &str, border_color: Color) {
        // Shadow
        self.fill_rect(x + 2, y + 2, w, h, Color(0x2104));
        
        // Main card
        self.fill_rect(x, y, w, h, Color::BLACK);
        
        // Border
        self.fill_rect(x, y, w, 1, border_color);         // Top
        self.fill_rect(x, y + h - 1, w, 1, border_color); // Bottom
        self.fill_rect(x, y, 1, h, border_color);         // Left
        self.fill_rect(x + w - 1, y, 1, h, border_color); // Right
        
        // Title
        if !title.is_empty() {
            self.draw_text(x + 5, y + 2, title, Color::TEXT_PRIMARY).ok();
        }
    }
    
    pub fn draw_button(&mut self, x: u16, y: u16, w: u16, h: u16, text: &str, selected: bool) {
        let bg_color = if selected { Color::PRIMARY_BLUE } else { Color(0x3186) };
        let text_color = if selected { Color::WHITE } else { Color::TEXT_PRIMARY };
        
        self.fill_rect(x, y, w, h, bg_color);
        
        // Center text
        let text_x = x + (w / 2) - (text.len() as u16 * 3);
        let text_y = y + (h / 2) - 4;
        self.draw_text(text_x, text_y, text, text_color).ok();
    }
}

// Implement Drop to ensure cleanup
impl Drop for Display {
    fn drop(&mut self) {
        // Could add display cleanup here if needed
        self.clear(Color::BLACK);
    }
}