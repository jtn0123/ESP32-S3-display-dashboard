// Color definitions for 16-bit RGB565 format
// Note: The T-Display-S3 uses BGR byte order in some cases

// Basic colors
pub const BLACK: u16 = 0x0000;
pub const WHITE: u16 = 0xFFFF;
pub const YELLOW: u16 = 0xFFE0;

// UI Theme colors
pub const PRIMARY_BLUE: u16 = 0x2589;
pub const PRIMARY_GREEN: u16 = 0x07E5;
pub const PRIMARY_PURPLE: u16 = 0x7817;
pub const PRIMARY_RED: u16 = 0xF800;
pub const SURFACE_LIGHT: u16 = 0x3186;
pub const TEXT_PRIMARY: u16 = WHITE;
pub const TEXT_SECONDARY: u16 = 0xBDF7;
pub const BORDER_COLOR: u16 = 0x4208;
pub const ACCENT_ORANGE: u16 = 0xC260;

// UI Element Colors
pub const SURFACE_DARK: u16 = 0x10A2;

// Helper function to create RGB565 color from RGB values
pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}