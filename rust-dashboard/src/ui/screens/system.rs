// System information screen

use crate::display::{Display, Color};
use crate::ui::theme::Theme;
use super::Screen;
use esp_hal::system::{SystemExt};

pub struct SystemScreen {
    free_heap: u32,
    total_heap: u32,
    cpu_usage: u8,
    uptime_seconds: u64,
}

impl SystemScreen {
    pub fn new() -> Self {
        Self {
            free_heap: 0,
            total_heap: 0,
            cpu_usage: 0,
            uptime_seconds: 0,
        }
    }
    
    fn update_system_info(&mut self) {
        // Get heap information
        // Note: These are placeholder calculations
        // In real implementation, use proper ESP-IDF functions
        self.free_heap = 150_000;  // Placeholder
        self.total_heap = 320_000; // Placeholder
        
        // Simulate CPU usage
        self.cpu_usage = 45; // Placeholder
        
        // Update uptime
        self.uptime_seconds += 1;
    }
}

impl Screen for SystemScreen {
    fn title(&self) -> &str {
        "System"
    }
    
    fn draw(&self, display: &mut Display, theme: &Theme) {
        let y_start = 25;
        let card_height = 45;
        let spacing = 5;
        
        // Memory card
        let mem_y = y_start;
        let mem_percent = (self.free_heap * 100 / self.total_heap) as u8;
        let mem_color = if mem_percent > 50 {
            theme.colors.success
        } else if mem_percent > 25 {
            theme.colors.warning
        } else {
            theme.colors.error
        };
        
        display.draw_card(40, mem_y, 240, card_height, "MEMORY", mem_color);
        display.draw_text(45, mem_y + 15, "Free:", theme.colors.text_secondary);
        display.draw_number(85, mem_y + 15, mem_percent as u32, mem_color);
        display.draw_text(110, mem_y + 15, "%", theme.colors.text_secondary);
        
        let free_kb = self.free_heap / 1024;
        display.draw_number(45, mem_y + 30, free_kb, mem_color);
        display.draw_text(80, mem_y + 30, "KB available", theme.colors.text_secondary);
        
        // CPU card
        let cpu_y = mem_y + card_height + spacing;
        let cpu_color = if self.cpu_usage < 70 {
            theme.colors.success
        } else if self.cpu_usage < 85 {
            theme.colors.warning
        } else {
            theme.colors.error
        };
        
        display.draw_card(40, cpu_y, 240, card_height, "CPU", cpu_color);
        display.draw_text(45, cpu_y + 15, "Usage:", theme.colors.text_secondary);
        display.draw_number(90, cpu_y + 15, self.cpu_usage as u32, cpu_color);
        display.draw_text(115, cpu_y + 15, "%", theme.colors.text_secondary);
        display.draw_text(45, cpu_y + 30, "2 cores @ 240MHz", theme.colors.text_secondary);
        
        // Uptime card
        let uptime_y = cpu_y + card_height + spacing;
        display.draw_card(40, uptime_y, 240, 35, "UPTIME", theme.colors.info);
        
        let hours = self.uptime_seconds / 3600;
        let minutes = (self.uptime_seconds % 3600) / 60;
        let seconds = self.uptime_seconds % 60;
        
        display.draw_number(45, uptime_y + 15, hours as u32, theme.colors.info);
        display.draw_text(70, uptime_y + 15, "h", theme.colors.text_secondary);
        display.draw_number(85, uptime_y + 15, minutes as u32, theme.colors.info);
        display.draw_text(110, uptime_y + 15, "m", theme.colors.text_secondary);
        display.draw_number(125, uptime_y + 15, seconds as u32, theme.colors.info);
        display.draw_text(150, uptime_y + 15, "s", theme.colors.text_secondary);
        
        // Version info
        display.draw_text(45, 145, "Rust Dashboard v0.1.0", theme.colors.primary);
    }
    
    fn update(&mut self) {
        self.update_system_info();
    }
}