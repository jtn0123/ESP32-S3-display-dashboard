/// GPIO bit-bang display backend implementation
use super::display_backend::DisplayBackend;
use super::DisplayManager;
use anyhow::Result;

impl DisplayBackend for DisplayManager {
    fn clear(&mut self, color: u16) -> Result<()> {
        self.clear(color)
    }
    
    fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        self.draw_pixel(x, y, color)
    }
    
    fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) -> Result<()> {
        self.fill_rect(x, y, width, height, color)
    }
    
    fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> Result<()> {
        self.draw_line(x0, y0, x1, y1, color)
    }
    
    fn draw_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) -> Result<()> {
        self.draw_rect(x, y, width, height, color)
    }
    
    fn draw_char(&mut self, x: u16, y: u16, c: char, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        self.draw_char(x, y, c, color, bg_color, scale)
    }
    
    fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        self.draw_text(x, y, text, color, bg_color, scale)
    }
    
    fn draw_text_centered(&mut self, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        self.draw_text_centered(y, text, color, bg_color, scale)
    }
    
    fn flush(&mut self) -> Result<()> {
        self.flush()
    }
    
    fn flush_region(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        self.flush_region(x, y, width, height)
    }
    
    fn width(&self) -> u16 {
        self.width
    }
    
    fn height(&self) -> u16 {
        self.height
    }
    
    fn update_auto_dim(&mut self) -> Result<()> {
        self.update_auto_dim()
    }
    
    fn reset_activity_timer(&mut self) {
        self.reset_activity_timer()
    }
    
    fn ensure_display_on(&mut self) -> Result<()> {
        self.ensure_display_on()
    }
    
    fn backend_name(&self) -> &'static str {
        "GPIO Bit-bang"
    }
    
    fn get_fps(&self) -> Option<f32> {
        Some(10.0) // Approximate GPIO performance
    }
}