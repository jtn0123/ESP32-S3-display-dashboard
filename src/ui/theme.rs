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
        // Dark theme (matching Arduino implementation)
        Self {
            background: Color::BLACK,
            surface: Color(0x1082),
            primary: Color(0x2589),
            secondary: Color(0x3186),
            accent: Color(0xC260),
            text_primary: Color(0xFFFF),
            text_secondary: Color(0xBDF7),
            border: Color(0x4208),
            success: Color(0x07E5),
            warning: Color(0x001F),
            error: Color(0xF800),
            info: Color(0x7817),
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
                surface: Color(0x1082),
                primary: Color(0x2589),
                secondary: Color(0x3186),
                accent: Color(0xC260),
                text_primary: Color(0xFFFF),
                text_secondary: Color(0xBDF7),
                border: Color(0x4208),
                success: Color(0x07E5),
                warning: Color(0x001F),
                error: Color(0xF800),
                info: Color(0x7817),
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
                secondary: Color::WHITE,
                accent: Color::YELLOW,
                text_primary: Color::WHITE,
                text_secondary: Color::WHITE,
                border: Color::WHITE,
                success: Color::GREEN,
                warning: Color::YELLOW,
                error: Color::RED,
                info: Color::CYAN,
            },
            brightness: 100,
            auto_dim: false,
        }
    }
}