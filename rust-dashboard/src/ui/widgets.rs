// Reusable UI widgets

use crate::display::{Display, Color};
use crate::ui::theme::Theme;

pub struct ProgressBar {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    value: u8,
    max: u8,
}

impl ProgressBar {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            value: 0,
            max: 100,
        }
    }
    
    pub fn set_value(&mut self, value: u8) {
        self.value = value.min(self.max);
    }
    
    pub fn draw(&self, display: &mut Display, theme: &Theme) {
        // Draw border
        display.draw_rect(self.x, self.y, self.width, self.height, theme.colors.border);
        
        // Draw fill
        let fill_width = ((self.width - 2) * self.value as u16) / self.max as u16;
        if fill_width > 0 {
            display.fill_rect(
                self.x + 1,
                self.y + 1,
                fill_width,
                self.height - 2,
                theme.colors.accent,
            );
        }
    }
}

pub struct Button {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    label: &'static str,
    selected: bool,
}

impl Button {
    pub fn new(x: u16, y: u16, width: u16, height: u16, label: &'static str) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label,
            selected: false,
        }
    }
    
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
    
    pub fn draw(&self, display: &mut Display, theme: &Theme) {
        let bg_color = if self.selected {
            theme.colors.accent
        } else {
            theme.colors.surface
        };
        
        let text_color = if self.selected {
            Color::BLACK
        } else {
            theme.colors.text_primary
        };
        
        // Draw button background
        display.fill_rect(self.x, self.y, self.width, self.height, bg_color);
        display.draw_rect(self.x, self.y, self.width, self.height, theme.colors.border);
        
        // Center text
        let text_x = self.x + (self.width / 2) - (self.label.len() as u16 * 3);
        let text_y = self.y + (self.height / 2) - 4;
        display.draw_text(text_x, text_y, self.label, text_color);
    }
}