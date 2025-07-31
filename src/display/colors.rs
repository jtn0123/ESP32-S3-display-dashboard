// Color definitions for 16-bit RGB565 format
// Note: The T-Display-S3 uses BGR byte order in some cases

// Basic colors - enhanced with better values
pub const BLACK: u16 = 0x0000;
pub const WHITE: u16 = 0xFFFF;
pub const YELLOW: u16 = 0xFFE0;

// Modern UI Theme colors - inspired by contemporary design systems
// Primary colors
pub const PRIMARY_BLUE: u16 = 0x3D7F;     // Modern vibrant blue (#3B82F6)
pub const PRIMARY_GREEN: u16 = 0x2746;    // Fresh green (#10B981)
pub const PRIMARY_PURPLE: u16 = 0x8B17;   // Rich purple (#8B5CF6)
pub const PRIMARY_RED: u16 = 0xE986;      // Vibrant red (#EF4444)

// Surface and background colors
pub const SURFACE_DARK: u16 = 0x18E3;     // Dark surface (#1F2937)
pub const SURFACE_LIGHT: u16 = 0x2965;    // Lighter surface (#374151)

// Text colors
pub const TEXT_PRIMARY: u16 = 0xFFFF;     // Pure white
pub const TEXT_SECONDARY: u16 = 0xCE79;   // Light gray (#D1D5DB)

// Accent colors
pub const ACCENT_ORANGE: u16 = 0xFBE0;    // Bright orange (#F97316)

// UI element colors
pub const BORDER_COLOR: u16 = 0x31A6;     // Subtle border (#374151)


// Helper function to create RGB565 color from RGB values
pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}

