// Progress indicator components

use crate::display::{Display, Color, FontRenderer};
use crate::animation::{Animation, EasingFunction};
use embassy_time::Duration;

pub struct ProgressBar {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    value: f32,
    target_value: f32,
    animation: Animation,
    border_color: Color,
    fill_color: Color,
    background_color: Color,
    show_percentage: bool,
}

impl ProgressBar {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            value: 0.0,
            target_value: 0.0,
            animation: Animation::new(0.0, 0.0, Duration::from_millis(300), EasingFunction::EaseInOut),
            border_color: Color::WHITE,
            fill_color: Color::PRIMARY_BLUE,
            background_color: Color::BLACK,
            show_percentage: true,
        }
    }
    
    pub fn set_value(&mut self, value: f32) {
        self.target_value = value.clamp(0.0, 100.0);
        self.animation = Animation::new(
            self.value,
            self.target_value,
            Duration::from_millis(300),
            EasingFunction::EaseInOut
        );
        self.animation.start();
    }
    
    pub fn set_colors(&mut self, border: Color, fill: Color, background: Color) {
        self.border_color = border;
        self.fill_color = fill;
        self.background_color = background;
    }
    
    pub fn update(&mut self) {
        if !self.animation.is_completed() {
            self.value = self.animation.update();
        }
    }
    
    pub fn draw(&self, display: &mut Display) {
        // Background
        display.fill_rect(self.x + 1, self.y + 1, self.width - 2, self.height - 2, self.background_color);
        
        // Progress fill
        let fill_width = ((self.width - 2) as f32 * (self.value / 100.0)) as u16;
        if fill_width > 0 {
            display.fill_rect(self.x + 1, self.y + 1, fill_width, self.height - 2, self.fill_color);
        }
        
        // Border
        display.draw_rect(self.x, self.y, self.width, self.height, self.border_color);
        
        // Percentage text
        if self.show_percentage && self.height >= 9 {
            let text = format_percentage(self.value as u8);
            let text_y = self.y + (self.height - 7) / 2;
            display.draw_text_centered_5x7(self.x, text_y, self.width, &text, Color::WHITE);
        }
    }
}

pub struct CircularProgress {
    cx: u16,
    cy: u16,
    radius: u16,
    thickness: u16,
    value: f32,
    target_value: f32,
    animation: Animation,
    color: Color,
    background_color: Color,
    start_angle: f32,
}

impl CircularProgress {
    pub fn new(cx: u16, cy: u16, radius: u16, thickness: u16) -> Self {
        Self {
            cx,
            cy,
            radius,
            thickness,
            value: 0.0,
            target_value: 0.0,
            animation: Animation::new(0.0, 0.0, Duration::from_millis(500), EasingFunction::EaseInOut),
            color: Color::PRIMARY_BLUE,
            background_color: Color(0x2104), // Dark gray
            start_angle: -90.0, // Start from top
        }
    }
    
    pub fn set_value(&mut self, value: f32) {
        self.target_value = value.clamp(0.0, 100.0);
        self.animation = Animation::new(
            self.value,
            self.target_value,
            Duration::from_millis(500),
            EasingFunction::EaseInOut
        );
        self.animation.start();
    }
    
    pub fn update(&mut self) {
        if !self.animation.is_completed() {
            self.value = self.animation.update();
        }
    }
    
    pub fn draw(&self, display: &mut Display) {
        // Draw background circle
        self.draw_arc(display, 0.0, 360.0, self.background_color);
        
        // Draw progress arc
        let angle = 360.0 * (self.value / 100.0);
        self.draw_arc(display, 0.0, angle, self.color);
        
        // Draw percentage in center
        let text = format_percentage(self.value as u8);
        display.draw_text_centered_5x7(
            self.cx - 15,
            self.cy - 3,
            30,
            &text,
            Color::WHITE
        );
    }
    
    fn draw_arc(&self, display: &mut Display, start: f32, sweep: f32, color: Color) {
        let steps = ((sweep / 360.0) * 100.0) as i32;
        
        for i in 0..steps {
            let angle = self.start_angle + start + (sweep * i as f32 / steps as f32);
            let angle_rad = angle * core::f32::consts::PI / 180.0;
            
            // Outer points
            let x1 = self.cx as f32 + self.radius as f32 * libm::cosf(angle_rad);
            let y1 = self.cy as f32 + self.radius as f32 * libm::sinf(angle_rad);
            
            // Inner points
            let inner_radius = self.radius - self.thickness;
            let x2 = self.cx as f32 + inner_radius as f32 * libm::cosf(angle_rad);
            let y2 = self.cy as f32 + inner_radius as f32 * libm::sinf(angle_rad);
            
            // Draw line from inner to outer radius
            display.draw_line(x2 as u16, y2 as u16, x1 as u16, y1 as u16, color);
        }
    }
}

// Helper function to format percentage
fn format_percentage(value: u8) -> heapless::String<4> {
    let mut s = heapless::String::new();
    
    if value >= 100 {
        s.push_str("100").ok();
    } else if value >= 10 {
        s.push((b'0' + value / 10) as char).ok();
        s.push((b'0' + value % 10) as char).ok();
    } else {
        s.push((b'0' + value) as char).ok();
    }
    s.push('%').ok();
    
    s
}