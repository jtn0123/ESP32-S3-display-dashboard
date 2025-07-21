/// Common display trait for both GPIO and ESP_LCD implementations
use anyhow::Result;

pub trait Display {
    fn clear(&mut self, color: u16) -> Result<()>;
    fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()>;
    fn draw_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) -> Result<()>;
    fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) -> Result<()>;
    fn draw_text(&mut self, text: &str, x: u16, y: u16, color: u16, bg_color: u16) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn reset_activity_timer(&mut self);
    fn update_auto_dim(&mut self) -> Result<()>;
    fn enable_frame_buffer(&mut self, enable: bool) -> Result<()>;
    fn is_frame_buffer_enabled(&self) -> bool;
}