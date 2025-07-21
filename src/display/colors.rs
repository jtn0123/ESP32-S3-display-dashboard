// Color definitions for 16-bit RGB565 format
// Note: The T-Display-S3 uses BGR byte order in some cases

// Basic colors - enhanced with better values
pub const BLACK: u16 = 0x0000;
pub const WHITE: u16 = 0xFFFF;
pub const YELLOW: u16 = 0xFFE0;
pub const RED: u16 = 0xF800;
pub const GREEN: u16 = 0x07E0;
pub const BLUE: u16 = 0x001F;
pub const CYAN: u16 = 0x07FF;
pub const MAGENTA: u16 = 0xF81F;
pub const ORANGE: u16 = 0xFD20;
pub const GRAY: u16 = 0x8430;
pub const DARK_GRAY: u16 = 0x4208;
pub const LIGHT_GRAY: u16 = 0xC618;

// Modern UI Theme colors - inspired by contemporary design systems
// Primary colors
pub const PRIMARY_BLUE: u16 = 0x3D7F;     // Modern vibrant blue (#3B82F6)
pub const PRIMARY_GREEN: u16 = 0x2746;    // Fresh green (#10B981)
pub const PRIMARY_PURPLE: u16 = 0x8B17;   // Rich purple (#8B5CF6)
pub const PRIMARY_RED: u16 = 0xE986;      // Vibrant red (#EF4444)

// Surface and background colors
pub const SURFACE_DARK: u16 = 0x18E3;     // Dark surface (#1F2937)
pub const SURFACE_LIGHT: u16 = 0x2965;    // Lighter surface (#374151)
pub const SURFACE_CARD: u16 = 0x2124;     // Card background (#2B3441)

// Text colors
pub const TEXT_PRIMARY: u16 = 0xFFFF;     // Pure white
pub const TEXT_SECONDARY: u16 = 0xCE79;   // Light gray (#D1D5DB)
pub const TEXT_MUTED: u16 = 0x8C71;       // Muted text (#9CA3AF)

// Accent colors
pub const ACCENT_ORANGE: u16 = 0xFBE0;    // Bright orange (#F97316)
pub const ACCENT_CYAN: u16 = 0x07FF;      // Cyan (#06B6D4)
pub const ACCENT_PINK: u16 = 0xF81F;      // Pink (#EC4899)
pub const ACCENT_AMBER: u16 = 0xFDE0;     // Amber (#F59E0B)

// UI element colors
pub const BORDER_COLOR: u16 = 0x31A6;     // Subtle border (#374151)
pub const BORDER_LIGHT: u16 = 0x4A49;     // Light border (#4B5563)
pub const SUCCESS_GREEN: u16 = 0x2746;    // Success indicator
pub const WARNING_YELLOW: u16 = 0xFDE0;   // Warning indicator
pub const ERROR_RED: u16 = 0xE986;        // Error indicator
pub const INFO_BLUE: u16 = 0x3D7F;       // Info indicator

// Interactive states
pub const HOVER_OVERLAY: u16 = 0x2965;    // Hover state overlay
pub const PRESSED_OVERLAY: u16 = 0x18E3;  // Pressed state

// Chart and graph colors - for data visualization
pub const CHART_LINE_1: u16 = 0x3D7F;    // Primary blue
pub const CHART_LINE_2: u16 = 0x2746;    // Green
pub const CHART_LINE_3: u16 = 0xFBE0;    // Orange
pub const CHART_LINE_4: u16 = 0x8B17;    // Purple
pub const CHART_GRID: u16 = 0x2124;      // Subtle grid lines

// Helper function to create RGB565 color from RGB values
pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}

// Helper function to create a color with transparency effect (blend with background)
pub fn blend_color(foreground: u16, background: u16, alpha: u8) -> u16 {
    let fg_r = (foreground >> 11) & 0x1F;
    let fg_g = (foreground >> 5) & 0x3F;
    let fg_b = foreground & 0x1F;
    
    let bg_r = (background >> 11) & 0x1F;
    let bg_g = (background >> 5) & 0x3F;
    let bg_b = background & 0x1F;
    
    let alpha_256 = alpha as u16;
    let inv_alpha = 255 - alpha as u16;
    
    let r = ((fg_r * alpha_256 + bg_r * inv_alpha) / 255) & 0x1F;
    let g = ((fg_g * alpha_256 + bg_g * inv_alpha) / 255) & 0x3F;
    let b = ((fg_b * alpha_256 + bg_b * inv_alpha) / 255) & 0x1F;
    
    (r << 11) | (g << 5) | b
}