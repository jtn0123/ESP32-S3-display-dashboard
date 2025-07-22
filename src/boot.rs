use anyhow::Result;
use std::sync::{Arc, Mutex};
use crate::display::{DisplayImpl as DisplayManager, colors::*};

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
        // Pre-calculate random sparkle distribution
        let mut circuit_points = Vec::new();
        
        // Use multiple distribution methods for true randomness
        let num_sparkles = 60; // More stars for better coverage
        
        // Method 1: Poisson disk sampling approximation
        // This ensures stars don't cluster too much
        let min_distance = 15.0; // Minimum distance between stars
        let mut placed_points: Vec<(f32, f32)> = Vec::new();
        
        // Try to place stars with minimum spacing
        for i in 0..num_sparkles * 3 { // Try 3x attempts to fill space
            // Use multiple primes for better randomness
            let seed_x = (i * 137 + i * i * 7) % 1000;
            let seed_y = (i * 223 + i * i * 11) % 1000;
            
            let x = 20.0 + (seed_x as f32 / 1000.0) * 280.0;
            let y = 25.0 + (seed_y as f32 / 1000.0) * 120.0;
            
            // Check distance to existing points
            let mut too_close = false;
            for &(px, py) in placed_points.iter() {
                let dist = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
                if dist < min_distance {
                    too_close = true;
                    break;
                }
            }
            
            if !too_close && circuit_points.len() < num_sparkles {
                circuit_points.push((x as u16, y as u16));
                placed_points.push((x, y));
            }
        }
        
        // Method 2: Add some truly random scattered stars
        // These can be closer together for variety
        for i in 0..20 {
            let hash1 = (i * 73 + 29) % 997;
            let hash2 = (i * 97 + 53) % 883;
            
            let x = 15 + (hash1 % 290) as u16;
            let y = 20 + (hash2 % 130) as u16;
            
            circuit_points.push((x, y));
        }
        
        // Method 3: Add edge stars for full coverage
        // These ensure the edges aren't empty
        let edge_positions = [
            // Top edge
            (30, 25), (80, 23), (130, 26), (180, 24), (230, 25), (280, 23),
            // Bottom edge
            (40, 145), (100, 143), (160, 146), (220, 144), (270, 145),
            // Left edge
            (18, 50), (20, 80), (17, 110),
            // Right edge
            (302, 45), (300, 75), (303, 105), (301, 135)
        ];
        for &pos in edge_positions.iter() {
            circuit_points.push(pos);
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
        
        // Title with cleaner design
        display.draw_text_centered(content_y, "ESP32-S3", PRIMARY_BLUE, None, 2)?;
        display.draw_text_centered(content_y + 25, "DASHBOARD", TEXT_PRIMARY, None, 1)?;
        
        // Subtle separator line
        let line_y = content_y + 45;
        let line_width = 100;
        let line_x = (320 - line_width) / 2;
        display.draw_line(line_x, line_y, line_x + line_width, line_y, BORDER_COLOR)?;
        
        // Progress section
        let progress_y = content_y + 60;
        
        // Stage description with typewriter effect
        let desc = stage.description();
        let chars_to_show = ((self.animation_frame / 2) as usize).min(desc.len());
        let partial_desc = &desc[..chars_to_show];
        
        // Clear description area (full width to prevent overlap)
        display.fill_rect(0, progress_y - 5, 320, 20, BLACK)?;
        display.draw_text_centered(progress_y, partial_desc, TEXT_PRIMARY, None, 1)?;
        
        // Animated progress bar with gradient
        self.draw_animated_progress(display, 50, progress_y + 25, 200, 12, stage.progress())?;
        
        // Progress percentage with subtle color
        display.fill_rect(130, progress_y + 42, 60, 16, BLACK)?; // Clear area first
        display.draw_text_centered(progress_y + 45, &format!("{}%", stage.progress()), PRIMARY_BLUE, None, 1)?;
        
        // Version and build info at bottom
        display.fill_rect(100, 152, 120, 16, BLACK)?; // Clear area first
        display.draw_text_centered(155, crate::version::DISPLAY_VERSION, TEXT_SECONDARY, None, 1)?;
        
        // Animated dots for "loading" effect
        if stage.progress() < 100 {
            self.draw_loading_dots(display, 160, 165)?;
        } else {
            // Clear the dots area when complete
            display.fill_rect(140, 165, 40, 10, BLACK)?;
        }
        
        self.animation_frame += 1;
        Ok(())
    }
    
    fn draw_circuit_pattern(&self, display: &mut DisplayManager) -> Result<()> {
        // Base color for inactive sparkles (subtle blue)
        let fade_factor = ((self.animation_frame as f32 * 0.03).sin().abs() * 0.3 + 0.2) * 255.0;
        let _line_color = rgb565(0, fade_factor as u8 / 8, fade_factor as u8 / 2);
        
        // Draw sparkle nodes with chaotic twinkling
        for (i, &(x, y)) in self.circuit_points.iter().enumerate() {
            // Create pseudo-random behavior using multiple prime numbers
            let seed1 = (i as u32 * 31 + self.animation_frame * 7) as f32;
            let seed2 = (i as u32 * 47 + self.animation_frame * 13) as f32;
            let seed3 = (i as u32 * 23 + self.animation_frame * 5) as f32;
            
            // Chaotic oscillations for more natural twinkling
            let twinkle = (seed1 * 0.017).sin() * (seed2 * 0.023).cos() + (seed3 * 0.011).sin();
            let brightness = ((twinkle + 1.5) * 0.4).max(0.0).min(1.0);
            
            // Quick flashes - occasional bright pulses
            let flash_chance = ((seed1 * 0.003).sin() + (seed2 * 0.007).cos()) > 1.85;
            let flash_brightness = if flash_chance { 1.0 } else { brightness };
            
            // Color varies from deep blue through cyan to white
            let color_phase = (seed3 * 0.019).sin() * 0.5 + 0.5;
            let intensity = flash_brightness * 255.0;
            
            // Deep blue -> cyan -> white spectrum
            let r = (intensity * flash_brightness * 0.3) as u8;  // Only bright sparkles have red
            let g = (intensity * (0.3 + color_phase * 0.7)) as u8;  // Green varies with phase
            let b = (intensity * (0.7 + flash_brightness * 0.3)) as u8;  // Blue always strong
            
            // Only draw visible sparkles
            if brightness > 0.15 {
                let sparkle_color = rgb565(r, g, b);
                
                // Smaller sizes - mostly single pixels with occasional 2-pixel stars
                let size = if brightness > 0.8 && ((seed1 * 0.05).sin() > 0.7) { 2 } else { 1 };
                
                display.fill_circle(x, y, size, sparkle_color)?;
                
                // Rare bright halos for magical effect
                if flash_brightness > 0.9 {
                    let halo = rgb565(r/3, g/3, b/2);
                    display.draw_circle(x, y, size + 1, halo)?;
                    // Tiny cross for super bright ones
                    if flash_chance {
                        display.draw_pixel(x - 1, y, halo)?;
                        display.draw_pixel(x + 1, y, halo)?;
                        display.draw_pixel(x, y - 1, halo)?;
                        display.draw_pixel(x, y + 1, halo)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    
    fn draw_animated_progress(&self, display: &mut DisplayManager, x: u16, y: u16, w: u16, h: u16, progress: u8) -> Result<()> {
        // Draw shadow for depth
        display.fill_rect(x + 2, y + 2, w, h, rgb565(10, 10, 10))?;
        
        // Border with subtle highlight
        display.draw_rect(x, y, w, h, BORDER_COLOR)?;
        display.draw_line(x + 1, y + 1, x + w - 2, y + 1, rgb565(60, 60, 60))?; // Top highlight
        
        // Background with subtle gradient
        display.fill_rect(x + 1, y + 1, w - 2, h - 2, SURFACE_DARK)?;
        
        // Calculate actual progress width
        let progress_width = ((w - 2) as u32 * progress as u32 / 100) as u16;
        
        if progress_width > 0 {
            // Main progress fill with smooth gradient
            for i in 0..progress_width {
                // Create a smooth gradient from left to right
                let gradient_factor = i as f32 / w as f32;
                let base_color = interpolate_color(PRIMARY_PURPLE, PRIMARY_BLUE, gradient_factor);
                
                // Add subtle vertical gradient for 3D effect
                for j in 0..(h - 2) {
                    let vertical_factor = 1.0 - (j as f32 / (h - 2) as f32) * 0.3;
                    let color = interpolate_color(SURFACE_DARK, base_color, vertical_factor);
                    display.draw_pixel(x + 1 + i, y + 1 + j, color)?;
                }
            }
            
            // Add subtle pulse glow at the end of progress
            let pulse = (self.animation_frame as f32 * 0.15).sin().abs();
            if progress_width > 3 {
                let glow_width = 3.min(progress_width);
                let glow_color = interpolate_color(PRIMARY_BLUE, WHITE, pulse * 0.5);
                display.fill_rect(x + progress_width - glow_width + 1, y + 1, glow_width, h - 2, glow_color)?;
            }
            
            // Add edge highlight for definition
            if progress_width > 1 {
                display.draw_line(x + progress_width, y + 1, x + progress_width, y + h - 2, interpolate_color(PRIMARY_BLUE, WHITE, 0.3))?;
            }
        }
        
        // Progress track markers every 20%
        for i in 1..5 {
            let marker_x = x + (w as u32 * i * 20 / 100) as u16;
            display.draw_line(marker_x, y, marker_x, y + h - 1, rgb565(40, 40, 40))?;
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