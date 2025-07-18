use anyhow::Result;
use std::sync::{Arc, Mutex};
use crate::display::{DisplayManager, colors::*};

#[derive(Clone, Copy, Debug)]
pub enum BootStage {
    PowerOn,
    DisplayInit,
    MemoryInit,
    SensorInit,
    NetworkInit,
    UISetup,
    Complete,
}

impl BootStage {
    pub fn progress(&self) -> u8 {
        match self {
            BootStage::PowerOn => 5,
            BootStage::DisplayInit => 20,
            BootStage::MemoryInit => 40,
            BootStage::SensorInit => 55,
            BootStage::NetworkInit => 70,
            BootStage::UISetup => 85,
            BootStage::Complete => 100,
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            BootStage::PowerOn => "System Power On",
            BootStage::DisplayInit => "Initializing Display",
            BootStage::MemoryInit => "Clearing Memory",
            BootStage::SensorInit => "Detecting Sensors",
            BootStage::NetworkInit => "Network Setup",
            BootStage::UISetup => "Loading Interface",
            BootStage::Complete => "Ready",
        }
    }
}

pub struct BootManager {
    current_stage: Arc<Mutex<BootStage>>,
    animation_frame: u32,
    circuit_points: Vec<(u16, u16)>,
}

impl BootManager {
    pub fn new() -> Self {
        // Pre-calculate circuit pattern points for animation
        let mut circuit_points = Vec::new();
        
        // Create a tech circuit pattern
        // Horizontal lines
        for x in (20..280).step_by(40) {
            circuit_points.push((x as u16, 40));
            circuit_points.push((x as u16, 140));
        }
        
        // Vertical connections
        for y in (40..140).step_by(20) {
            circuit_points.push((60, y as u16));
            circuit_points.push((220, y as u16));
        }
        
        // Node points
        for x in [60, 140, 220] {
            for y in [60, 100] {
                circuit_points.push((x, y));
            }
        }
        
        Self {
            current_stage: Arc::new(Mutex::new(BootStage::PowerOn)),
            animation_frame: 0,
            circuit_points,
        }
    }
    
    pub fn set_stage(&self, stage: BootStage) {
        *self.current_stage.lock().unwrap() = stage;
    }
    
    pub fn get_stage(&self) -> BootStage {
        *self.current_stage.lock().unwrap()
    }
    
    pub fn render_boot_screen(&mut self, display: &mut DisplayManager) -> Result<()> {
        let stage = self.get_stage();
        
        // Only clear on first render or stage change
        if self.animation_frame == 0 {
            display.clear(BLACK)?;
        }
        
        // Draw animated background circuit pattern
        self.draw_circuit_pattern(display)?;
        
        // Main content area with subtle gradient effect
        let content_y = 50;
        
        // Title with glow effect
        self.draw_glowing_text(display, 160, content_y, "ESP32-S3", PRIMARY_BLUE, 2)?;
        display.draw_text_centered(content_y + 25, "DASHBOARD", WHITE, None, 1)?;
        
        // Progress section
        let progress_y = content_y + 60;
        
        // Stage description with typewriter effect
        let desc = stage.description();
        let chars_to_show = ((self.animation_frame / 2) as usize).min(desc.len());
        let partial_desc = &desc[..chars_to_show];
        
        // Clear description area
        display.fill_rect(50, progress_y - 5, 200, 20, BLACK)?;
        display.draw_text_centered(progress_y, partial_desc, TEXT_PRIMARY, None, 1)?;
        
        // Animated progress bar with gradient
        self.draw_animated_progress(display, 50, progress_y + 25, 200, 12, stage.progress())?;
        
        // Progress percentage with pulse effect
        let pulse = (self.animation_frame as f32 * 0.1).sin().abs();
        let percent_color = interpolate_color(PRIMARY_BLUE, WHITE, pulse);
        display.draw_text_centered(progress_y + 45, &format!("{}%", stage.progress()), percent_color, None, 1)?;
        
        // Version and build info at bottom
        display.draw_text_centered(155, "v4.31", TEXT_SECONDARY, None, 1)?;
        
        // Animated dots for "loading" effect
        if stage.progress() < 100 {
            self.draw_loading_dots(display, 160, 165)?;
        }
        
        self.animation_frame += 1;
        Ok(())
    }
    
    fn draw_circuit_pattern(&self, display: &mut DisplayManager) -> Result<()> {
        // Draw fading circuit lines based on animation frame
        let fade_factor = ((self.animation_frame as f32 * 0.05).sin().abs() * 0.5 + 0.5) * 255.0;
        let line_color = rgb565(0, fade_factor as u8 / 4, fade_factor as u8 / 2);
        
        // Draw connections between points
        for i in 0..self.circuit_points.len() {
            let (x, y) = self.circuit_points[i];
            
            // Draw small nodes at intersection points
            if i % 3 == 0 {
                let node_active = (self.animation_frame + i as u32 * 10) % 60 < 20;
                let node_color = if node_active { PRIMARY_BLUE } else { line_color };
                display.fill_circle(x, y, 2, node_color)?;
            }
            
            // Draw connecting lines (simplified)
            if i > 0 && i % 2 == 0 {
                let (prev_x, prev_y) = self.circuit_points[i - 1];
                // Only draw if points are close enough
                let dx = (x as i32 - prev_x as i32).abs();
                let dy = (y as i32 - prev_y as i32).abs();
                if dx < 50 && dy < 50 {
                    display.draw_line(prev_x, prev_y, x, y, line_color)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn draw_glowing_text(&self, display: &mut DisplayManager, _x: u16, y: u16, text: &str, color: u16, scale: u8) -> Result<()> {
        // Create glow effect with multiple layers
        let glow_intensity = (self.animation_frame as f32 * 0.1).sin().abs() * 0.5 + 0.5;
        
        // Outer glow
        let glow_color = interpolate_color(BLACK, color, glow_intensity * 0.3);
        display.draw_text_centered(y - 1, text, glow_color, None, scale)?;
        display.draw_text_centered(y + 1, text, glow_color, None, scale)?;
        
        // Main text
        display.draw_text_centered(y, text, color, None, scale)?;
        
        Ok(())
    }
    
    fn draw_animated_progress(&self, display: &mut DisplayManager, x: u16, y: u16, w: u16, h: u16, progress: u8) -> Result<()> {
        // Border with rounded corners effect
        display.draw_rect(x, y, w, h, BORDER_COLOR)?;
        
        // Background
        display.fill_rect(x + 1, y + 1, w - 2, h - 2, SURFACE_DARK)?;
        
        // Calculate actual progress width
        let progress_width = ((w - 2) as u32 * progress as u32 / 100) as u16;
        
        if progress_width > 0 {
            // Create gradient effect in progress bar
            for i in 0..progress_width {
                let gradient_factor = i as f32 / progress_width as f32;
                let color = interpolate_color(PRIMARY_PURPLE, PRIMARY_BLUE, gradient_factor);
                display.fill_rect(x + 1 + i, y + 1, 1, h - 2, color)?;
            }
            
            // Add shimmer effect
            let shimmer_pos = (self.animation_frame * 3) % (progress_width as u32 + 20);
            if shimmer_pos < progress_width as u32 {
                let shimmer_x = x + 1 + shimmer_pos as u16;
                display.fill_rect(shimmer_x, y + 1, 2.min(progress_width - shimmer_pos as u16), h - 2, WHITE)?;
            }
        }
        
        Ok(())
    }
    
    fn draw_loading_dots(&self, display: &mut DisplayManager, x: u16, y: u16) -> Result<()> {
        let dots = [".", "..", "...", ""];
        let dot_index = (self.animation_frame / 10) as usize % dots.len();
        
        // Clear the area first
        display.fill_rect(x - 20, y, 40, 10, BLACK)?;
        
        // Draw dots
        display.draw_text(x - 15, y, dots[dot_index], TEXT_SECONDARY, None, 1)?;
        
        Ok(())
    }
}

// Helper function to interpolate between two colors
fn interpolate_color(color1: u16, color2: u16, factor: f32) -> u16 {
    let r1 = (color1 >> 11) & 0x1F;
    let g1 = (color1 >> 5) & 0x3F;
    let b1 = color1 & 0x1F;
    
    let r2 = (color2 >> 11) & 0x1F;
    let g2 = (color2 >> 5) & 0x3F;
    let b2 = color2 & 0x1F;
    
    let r = (r1 as f32 * (1.0 - factor) + r2 as f32 * factor) as u16;
    let g = (g1 as f32 * (1.0 - factor) + g2 as f32 * factor) as u16;
    let b = (b1 as f32 * (1.0 - factor) + b2 as f32 * factor) as u16;
    
    (r << 11) | (g << 5) | b
}

// Helper to create RGB565 color from RGB values
fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}