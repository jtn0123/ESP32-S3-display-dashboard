// Settings screen

use crate::display::{Display, Color};
use crate::ui::theme::Theme;
use super::Screen;

pub struct SettingsScreen {
    brightness: u8,
    auto_dim: bool,
    theme_index: usize,
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            brightness: 100,
            auto_dim: false,
            theme_index: 0,
        }
    }
}

impl Screen for SettingsScreen {
    fn title(&self) -> &str {
        "Settings"
    }
    
    fn draw(&self, display: &mut Display, theme: &Theme) {
        // Display settings
        display.draw_card(40, 25, 240, 45, "DISPLAY", theme.colors.primary);
        
        display.draw_text(45, 40, "Brightness:", theme.colors.text_secondary);
        display.draw_number(115, 40, self.brightness as u32, theme.colors.info);
        display.draw_text(140, 40, "%", theme.colors.text_secondary);
        
        // Brightness bar
        display.draw_rect(45, 55, 102, 8, theme.colors.border);
        let fill_width = self.brightness as u16;
        display.fill_rect(46, 56, fill_width, 6, theme.colors.info);
        
        // Theme selection
        display.draw_card(40, 75, 240, 35, "THEME", theme.colors.secondary);
        let theme_names = ["Dark", "Readable", "High Contrast"];
        display.draw_text(45, 90, theme_names[self.theme_index], theme.colors.accent);
        
        // System settings
        display.draw_card(40, 115, 240, 45, "SYSTEM", theme.colors.info);
        display.draw_text(45, 130, "Auto-dim:", theme.colors.text_secondary);
        display.draw_text(105, 130, if self.auto_dim { "ON" } else { "OFF" }, 
                          if self.auto_dim { theme.colors.success } else { theme.colors.error });
        
        display.draw_text(45, 145, "Version: 0.1.0", theme.colors.text_secondary);
    }
    
    fn update(&mut self) {
        // Settings are updated via user interaction
    }
}