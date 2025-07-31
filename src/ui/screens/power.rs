// Power management screen

use crate::display::{Display, Color};
use crate::ui::theme::Theme;
use super::Screen;

pub struct PowerScreen {
    battery_voltage: u16,
    battery_percent: u8,
    on_usb: bool,
    is_charging: bool,
}

impl PowerScreen {
    pub fn new() -> Self {
        Self {
            battery_voltage: 4200,
            battery_percent: 100,
            on_usb: true,
            is_charging: false,
        }
    }
}

impl Screen for PowerScreen {
    fn title(&self) -> &str {
        "Power"
    }
    
    fn draw(&self, display: &mut Display, theme: &Theme) {
        // Power status card
        let status_color = if self.is_charging {
            Color::YELLOW
        } else if self.on_usb {
            theme.colors.info
        } else {
            theme.colors.success
        };
        
        display.draw_card(40, 25, 240, 45, "POWER STATUS", status_color);
        
        let status_text = if !self.on_usb {
            "Battery Power"
        } else if self.is_charging {
            "USB Charging"
        } else {
            "USB Power"
        };
        
        display.draw_text(45, 40, status_text, status_color);
        
        // Battery level
        display.draw_card(40, 75, 240, 45, "BATTERY", theme.colors.info);
        display.draw_number(45, 90, self.battery_percent as u32, theme.colors.success);
        display.draw_text(70, 90, "%", theme.colors.text_secondary);
        display.draw_number(100, 90, self.battery_voltage as u32, theme.colors.info);
        display.draw_text(140, 90, "mV", theme.colors.text_secondary);
        
        // Battery icon
        display.draw_rect(45, 105, 40, 12, theme.colors.border);
        display.fill_rect(85, 107, 2, 8, theme.colors.border);
        
        // Battery fill
        let fill_width = (36 * self.battery_percent as u16) / 100;
        if fill_width > 0 {
            display.fill_rect(47, 107, fill_width, 8, theme.colors.success);
        }
        
        // Estimate
        let est_hours = (self.battery_percent / 15) + 1;
        display.draw_text(95, 107, "~", theme.colors.text_secondary);
        display.draw_number(105, 107, est_hours as u32, theme.colors.info);
        display.draw_text(120, 107, "hours", theme.colors.text_secondary);
    }
    
    fn update(&mut self) {
        // Battery data is updated from main loop via ProcessedData
    }
}