// Hardware monitoring screen

use crate::display::{Display, Color};
use crate::ui::theme::Theme;
use super::Screen;

pub struct HardwareScreen {
    chip_temp: f32,
    hall_sensor: i16,
}

impl HardwareScreen {
    pub fn new() -> Self {
        Self {
            chip_temp: 45.0,
            hall_sensor: 0,
        }
    }
}

impl Screen for HardwareScreen {
    fn title(&self) -> &str {
        "Hardware"
    }
    
    fn draw(&self, display: &mut Display, theme: &Theme) {
        // Chip info
        display.draw_card(40, 25, 240, 45, "ESP32-S3", theme.colors.primary);
        display.draw_text(45, 40, "Dual-core Xtensa LX7", theme.colors.text_secondary);
        display.draw_text(45, 52, "240MHz, 8MB Flash", theme.colors.text_secondary);
        
        // Temperature
        let temp_color = if self.chip_temp < 60.0 {
            theme.colors.success
        } else if self.chip_temp < 80.0 {
            theme.colors.warning
        } else {
            theme.colors.error
        };
        
        display.draw_card(40, 75, 240, 45, "TEMPERATURE", temp_color);
        display.draw_text(45, 90, "Chip:", theme.colors.text_secondary);
        display.draw_number(80, 90, self.chip_temp as u32, temp_color);
        display.draw_text(105, 90, "°C", theme.colors.text_secondary);
        
        // Sensors
        display.draw_card(40, 125, 240, 35, "SENSORS", theme.colors.info);
        display.draw_text(45, 140, "Hall:", theme.colors.text_secondary);
        display.draw_number(80, 140, self.hall_sensor.abs() as u32, theme.colors.info);
        if self.hall_sensor < 0 {
            display.draw_text(75, 140, "-", theme.colors.info);
        }
    }
    
    fn update(&mut self) {
        // TODO: Read real sensor values
        // Simulate temperature changes
        self.chip_temp += 0.1;
        if self.chip_temp > 50.0 {
            self.chip_temp = 40.0;
        }
    }
}