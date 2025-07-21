/// Display backend trait for abstracting display implementations
use anyhow::Result;

/// Common trait for display backends (GPIO bit-bang vs ESP LCD DMA)
pub trait DisplayBackend {
    /// Clear the entire screen with a color
    fn clear(&mut self, color: u16) -> Result<()>;
    
    /// Draw a single pixel
    fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()>;
    
    /// Fill a rectangle with a color
    fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) -> Result<()>;
    
    /// Draw a line between two points
    fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> Result<()>;
    
    /// Draw a rectangle outline
    fn draw_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) -> Result<()>;
    
    /// Draw a character at a position
    fn draw_char(&mut self, x: u16, y: u16, c: char, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()>;
    
    /// Draw text at a position
    fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()>;
    
    /// Draw centered text
    fn draw_text_centered(&mut self, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()>;
    
    /// Flush any pending operations to the display
    fn flush(&mut self) -> Result<()>;
    
    /// Flush a specific region
    fn flush_region(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()>;
    
    /// Get display width
    fn width(&self) -> u16;
    
    /// Get display height
    fn height(&self) -> u16;
    
    /// Update backlight/power management
    fn update_auto_dim(&mut self) -> Result<()>;
    
    /// Reset activity timer for auto-dim
    fn reset_activity_timer(&mut self);
    
    /// Ensure display is on
    fn ensure_display_on(&mut self) -> Result<()>;
    
    /// Get backend type name for debugging
    fn backend_name(&self) -> &'static str;
    
    /// Get performance metrics if available
    fn get_fps(&self) -> Option<f32> {
        None
    }
}