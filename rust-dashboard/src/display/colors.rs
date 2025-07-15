// Color definitions for 16-bit RGB565 format
// Note: The T-Display-S3 uses BGR byte order in some cases

// Basic colors
pub const BLACK: u16 = 0x0000;
pub const WHITE: u16 = 0xFFFF;
pub const RED: u16 = 0xF800;
pub const GREEN: u16 = 0x07E0;
pub const BLUE: u16 = 0x001F;
pub const YELLOW: u16 = 0xFFE0;
pub const CYAN: u16 = 0x07FF;
pub const MAGENTA: u16 = 0xF81F;

// UI Theme colors
pub const PRIMARY_BLUE: u16 = 0x2589;
pub const PRIMARY_GREEN: u16 = 0x07E5;
pub const PRIMARY_PURPLE: u16 = 0x7817;
pub const PRIMARY_RED: u16 = 0xF800;
pub const SURFACE_DARK: u16 = BLACK;
pub const SURFACE_LIGHT: u16 = 0x3186;
pub const TEXT_PRIMARY: u16 = WHITE;
pub const TEXT_SECONDARY: u16 = 0xBDF7;
pub const BORDER_COLOR: u16 = 0x4208;
pub const ACCENT_ORANGE: u16 = 0xC260;

// Helper functions
pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}

pub fn rgb565_to_rgb(color: u16) -> (u8, u8, u8) {
    let r = ((color >> 11) & 0x1F) as u8 * 255 / 31;
    let g = ((color >> 5) & 0x3F) as u8 * 255 / 63;
    let b = (color & 0x1F) as u8 * 255 / 31;
    (r, g, b)
}

pub fn interpolate_color(color1: u16, color2: u16, ratio: f32) -> u16 {
    let (r1, g1, b1) = rgb565_to_rgb(color1);
    let (r2, g2, b2) = rgb565_to_rgb(color2);
    
    let r = (r1 as f32 * (1.0 - ratio) + r2 as f32 * ratio) as u8;
    let g = (g1 as f32 * (1.0 - ratio) + g2 as f32 * ratio) as u8;
    let b = (b1 as f32 * (1.0 - ratio) + b2 as f32 * ratio) as u8;
    
    rgb565(r, g, b)
}