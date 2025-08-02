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
    pub fn get_theme_by_index(index: usize) -> Self {
        match index % 8 {
            0 => Self::default(),      // Dark
            1 => Self::readable(),     // Readable
            2 => Self::high_contrast(),// High Contrast
            3 => Self::cyberpunk(),    // Cyberpunk
            4 => Self::ocean(),        // Ocean
            5 => Self::sunset(),       // Sunset
            6 => Self::matrix(),       // Matrix
            7 => Self::nord(),         // Nord
            _ => Self::default(),
        }
    }
    
    pub fn get_theme_name(index: usize) -> &'static str {
        match index % 8 {
            0 => "Dark",
            1 => "Readable",
            2 => "High Contrast",
            3 => "Cyberpunk",
            4 => "Ocean",
            5 => "Sunset",
            6 => "Matrix",
            7 => "Nord",
            _ => "Dark",
        }
    }
    
    pub const THEME_COUNT: usize = 8;
    
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
    
    pub fn cyberpunk() -> Self {
        Self {
            colors: ColorTheme {
                background: Color(0x0842),   // Deep purple-black
                surface: Color(0x10A3),      // Dark purple surface
                primary: Color(0xF81F),      // Hot pink/magenta
                secondary: Color(0x07FF),    // Cyan
                accent: Color(0xFFE0),       // Yellow
                text_primary: Color(0xF7FF), // Bright cyan-white
                text_secondary: Color(0xA5FF), // Light purple
                border: Color(0xF81F),       // Hot pink border
                success: Color(0x07FF),      // Cyan
                warning: Color(0xFFE0),      // Yellow
                error: Color(0xF800),        // Red
                info: Color(0x851F),         // Purple-blue
            },
            brightness: 100,
            auto_dim: false,
        }
    }
    
    pub fn ocean() -> Self {
        Self {
            colors: ColorTheme {
                background: Color(0x0105),   // Deep ocean blue
                surface: Color(0x0969),      // Dark teal
                primary: Color(0x07FF),      // Bright cyan
                secondary: Color(0x2D5A),    // Sea green
                accent: Color(0xFDE0),       // Sandy yellow
                text_primary: Color(0xE7FF), // Light cyan
                text_secondary: Color(0x9DF7), // Pale blue
                border: Color(0x2D5A),       // Sea green border
                success: Color(0x2FE6),      // Aqua green
                warning: Color(0xFDE0),      // Sandy yellow
                error: Color(0xFC90),        // Coral red
                info: Color(0x3D7F),         // Ocean blue
            },
            brightness: 100,
            auto_dim: false,
        }
    }
    
    pub fn sunset() -> Self {
        Self {
            colors: ColorTheme {
                background: Color(0x2000),   // Deep red-black
                surface: Color(0x5800),      // Dark orange
                primary: Color(0xFBE0),      // Bright orange
                secondary: Color(0xFCE0),    // Light orange
                accent: Color(0xFFE0),       // Yellow
                text_primary: Color(0xFFFF), // White
                text_secondary: Color(0xFE79), // Light peach
                border: Color(0xB400),       // Dark orange border
                success: Color(0x8FE0),      // Light green
                warning: Color(0xFFE0),      // Yellow
                error: Color(0xF800),        // Red
                info: Color(0xFCE0),         // Light orange
            },
            brightness: 100,
            auto_dim: false,
        }
    }
    
    pub fn matrix() -> Self {
        Self {
            colors: ColorTheme {
                background: Color::BLACK,
                surface: Color(0x0200),      // Very dark green
                primary: Color(0x07E0),      // Bright green
                secondary: Color(0x0460),    // Medium green
                accent: Color(0x07E0),       // Bright green
                text_primary: Color(0x07E0), // Bright green
                text_secondary: Color(0x0460), // Medium green
                border: Color(0x0260),       // Dark green border
                success: Color(0x07E0),      // Bright green
                warning: Color(0x0FE0),      // Yellow-green
                error: Color(0x07E0),        // Green (Matrix style)
                info: Color(0x0460),         // Medium green
            },
            brightness: 100,
            auto_dim: false,
        }
    }
    
    pub fn monochrome() -> Self {
        Self {
            colors: ColorTheme {
                background: Color::BLACK,
                surface: Color(0x2124),      // Dark gray
                primary: Color::WHITE,
                secondary: Color(0x8C51),    // Medium gray
                accent: Color::WHITE,
                text_primary: Color::WHITE,
                text_secondary: Color(0xAD75), // Light gray
                border: Color(0x5ACB),       // Gray border
                success: Color::WHITE,
                warning: Color(0xD6BA),      // Light gray
                error: Color::WHITE,
                info: Color(0x8C51),         // Medium gray
            },
            brightness: 100,
            auto_dim: false,
        }
    }
    
    pub fn nord() -> Self {
        Self {
            colors: ColorTheme {
                background: Color(0x18E3),   // Nord dark background
                surface: Color(0x2965),      // Nord surface
                primary: Color(0x5E9F),      // Nord blue
                secondary: Color(0x739C),    // Nord light blue
                accent: Color(0xE5A3),       // Nord orange
                text_primary: Color(0xE73C), // Nord white
                text_secondary: Color(0xCE79), // Nord gray
                border: Color(0x4228),       // Nord border
                success: Color(0x8FE3),      // Nord green
                warning: Color(0xFDE3),      // Nord yellow
                error: Color(0xE126),        // Nord red
                info: Color(0x5E9F),         // Nord blue
            },
            brightness: 100,
            auto_dim: false,
        }
    }
}