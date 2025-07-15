// Loading spinner animations

use crate::display::{Display, Color};
use crate::animation::{Animation, EasingFunction};
use embassy_time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum SpinnerStyle {
    Dots,
    Ring,
    Pulse,
    Wave,
}

pub struct LoadingSpinner {
    cx: u16,
    cy: u16,
    size: u16,
    style: SpinnerStyle,
    color: Color,
    animation: Animation,
    start_time: Instant,
}

impl LoadingSpinner {
    pub fn new(cx: u16, cy: u16, size: u16, style: SpinnerStyle) -> Self {
        Self {
            cx,
            cy,
            size,
            style,
            color: Color::PRIMARY_BLUE,
            animation: Animation::new(0.0, 360.0, Duration::from_secs(2), EasingFunction::Linear),
            start_time: Instant::now(),
        }
    }
    
    pub fn start(&mut self) {
        self.animation.start();
        self.start_time = Instant::now();
    }
    
    pub fn update(&mut self) {
        self.animation.update();
    }
    
    pub fn draw(&self, display: &mut Display) {
        match self.style {
            SpinnerStyle::Dots => self.draw_dots(display),
            SpinnerStyle::Ring => self.draw_ring(display),
            SpinnerStyle::Pulse => self.draw_pulse(display),
            SpinnerStyle::Wave => self.draw_wave(display),
        }
    }
    
    fn draw_dots(&self, display: &mut Display) {
        let angle = self.animation.update() * core::f32::consts::PI / 180.0;
        let num_dots = 8;
        
        for i in 0..num_dots {
            let dot_angle = angle + (2.0 * core::f32::consts::PI * i as f32 / num_dots as f32);
            let x = self.cx as f32 + (self.size as f32 * libm::cosf(dot_angle));
            let y = self.cy as f32 + (self.size as f32 * libm::sinf(dot_angle));
            
            // Fade dots based on position
            let fade = ((i as f32 / num_dots as f32) * 255.0) as u8;
            let color = self.fade_color(fade);
            
            // Draw dot
            display.fill_rect(x as u16 - 1, y as u16 - 1, 3, 3, color);
        }
    }
    
    fn draw_ring(&self, display: &mut Display) {
        let progress = self.animation.update() / 360.0;
        let start_angle = progress * 360.0;
        let sweep_angle = 90.0;
        
        // Draw arc
        let steps = 20;
        for i in 0..steps {
            let angle = (start_angle + (sweep_angle * i as f32 / steps as f32)) * core::f32::consts::PI / 180.0;
            let x = self.cx as f32 + (self.size as f32 * libm::cosf(angle));
            let y = self.cy as f32 + (self.size as f32 * libm::sinf(angle));
            
            display.fill_rect(x as u16 - 1, y as u16 - 1, 3, 3, self.color);
        }
    }
    
    fn draw_pulse(&self, display: &mut Display) {
        let t = self.start_time.elapsed().as_millis() as f32 / 1000.0;
        let pulse = (libm::sinf(t * 2.0 * core::f32::consts::PI) + 1.0) / 2.0;
        
        let radius = (self.size as f32 * (0.5 + 0.5 * pulse)) as u16;
        let alpha = ((1.0 - pulse) * 255.0) as u8;
        let color = self.fade_color(alpha);
        
        display.draw_circle(self.cx, self.cy, radius, color);
        display.draw_circle(self.cx, self.cy, radius + 1, color);
    }
    
    fn draw_wave(&self, display: &mut Display) {
        let t = self.start_time.elapsed().as_millis() as f32 / 1000.0;
        let num_bars = 5;
        let bar_width = 3;
        let spacing = 2;
        let total_width = num_bars * (bar_width + spacing) - spacing;
        let start_x = self.cx - total_width as u16 / 2;
        
        for i in 0..num_bars {
            let phase = t * 2.0 * core::f32::consts::PI + (i as f32 * 0.5);
            let height = ((libm::sinf(phase) + 1.0) / 2.0 * self.size as f32) as u16 + 3;
            
            let x = start_x + (i * (bar_width + spacing)) as u16;
            let y = self.cy - height / 2;
            
            display.fill_rect(x, y, bar_width as u16, height, self.color);
        }
    }
    
    fn fade_color(&self, alpha: u8) -> Color {
        // Simple fade by reducing intensity
        let r = ((self.color.0 >> 11) & 0x1F) as u8;
        let g = ((self.color.0 >> 5) & 0x3F) as u8;
        let b = (self.color.0 & 0x1F) as u8;
        
        let fade_factor = alpha as u16 / 255;
        let r_faded = (r as u16 * fade_factor / 255) as u16;
        let g_faded = (g as u16 * fade_factor / 255) as u16;
        let b_faded = (b as u16 * fade_factor / 255) as u16;
        
        Color((r_faded << 11) | (g_faded << 5) | b_faded)
    }
}