// UI theme definitions

use crate::display::Color;

#[derive(Debug, Clone, Copy)]
pub struct ColorTheme {
    pub background: Color,
    pub surface: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub border: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        // Modern dark theme with vibrant accents
        Self {
            background: Color::BLACK,
            surface: Color(0x18E3),      // SURFACE_DARK - modern dark surface
            primary: Color(0x3D7F),      // PRIMARY_BLUE - vibrant blue
            secondary: Color(0x2965),    // SURFACE_LIGHT - lighter surface
            accent: Color(0xFBE0),       // ACCENT_ORANGE - bright orange
            text_primary: Color(0xFFFF), // Pure white
            text_secondary: Color(0xCE79), // TEXT_SECONDARY - light gray
            border: Color(0x31A6),       // BORDER_COLOR - subtle border
            success: Color(0x2746),      // SUCCESS_GREEN - fresh green
            warning: Color(0xFDE0),      // WARNING_YELLOW - amber
            error: Color(0xE986),        // ERROR_RED - vibrant red
            info: Color(0x3D7F),         // INFO_BLUE - matches primary
        }
    }
}

pub struct Theme {
    pub colors: ColorTheme,
    pub brightness: u8,
    pub auto_dim: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: ColorTheme::default(),
            brightness: 100,
            auto_dim: false,
        }
    }
}

impl Theme {
    pub fn readable() -> Self {
        Self {
            colors: ColorTheme {
                background: Color::BLACK,
                surface: Color(0x2124),      // SURFACE_CARD - better contrast
                primary: Color(0x3D7F),      // PRIMARY_BLUE
                secondary: Color(0x2965),    // SURFACE_LIGHT
                accent: Color(0x07FF),       // ACCENT_CYAN - high visibility
                text_primary: Color(0xFFFF), // WHITE
                text_secondary: Color(0xCE79), // TEXT_SECONDARY
                border: Color(0x4A49),       // BORDER_LIGHT - more visible
                success: Color(0x2746),      // SUCCESS_GREEN
                warning: Color(0xFDE0),      // WARNING_YELLOW
                error: Color(0xE986),        // ERROR_RED
                info: Color(0x3D7F),         // INFO_BLUE
            },
            brightness: 100,
            auto_dim: false,
        }
    }
    
    pub fn high_contrast() -> Self {
        Self {
            colors: ColorTheme {
                background: Color::BLACK,
                surface: Color::BLACK,
                primary: Color::WHITE,
                secondary: Color(0xC618),    // LIGHT_GRAY for some contrast
                accent: Color(0xFFE0),       // YELLOW - maximum visibility
                text_primary: Color::WHITE,
                text_secondary: Color::WHITE,
                border: Color::WHITE,
                success: Color(0x07E0),      // Pure GREEN
                warning: Color(0xFFE0),      // Pure YELLOW
                error: Color(0xF800),        // Pure RED
                info: Color(0x07FF),         // CYAN
            },
            brightness: 100,
            auto_dim: false,
        }
    }
}