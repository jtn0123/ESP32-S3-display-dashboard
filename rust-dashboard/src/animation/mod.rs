// Animation system for smooth UI transitions

use embassy_time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInElastic,
    EaseOutElastic,
    Bounce,
}

#[derive(Debug)]
pub struct Animation {
    start_value: f32,
    end_value: f32,
    duration: Duration,
    start_time: Option<Instant>,
    easing: EasingFunction,
    completed: bool,
}

impl Animation {
    pub fn new(start: f32, end: f32, duration: Duration, easing: EasingFunction) -> Self {
        Self {
            start_value: start,
            end_value: end,
            duration,
            start_time: None,
            easing,
            completed: false,
        }
    }
    
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.completed = false;
    }
    
    pub fn update(&mut self) -> f32 {
        if let Some(start_time) = self.start_time {
            let elapsed = start_time.elapsed();
            
            if elapsed >= self.duration {
                self.completed = true;
                return self.end_value;
            }
            
            let progress = elapsed.as_millis() as f32 / self.duration.as_millis() as f32;
            let eased = self.apply_easing(progress);
            
            self.start_value + (self.end_value - self.start_value) * eased
        } else {
            self.start_value
        }
    }
    
    pub fn is_completed(&self) -> bool {
        self.completed
    }
    
    pub fn reset(&mut self) {
        self.start_time = None;
        self.completed = false;
    }
    
    pub fn reverse(&mut self) {
        core::mem::swap(&mut self.start_value, &mut self.end_value);
        self.reset();
    }
    
    fn apply_easing(&self, t: f32) -> f32 {
        match self.easing {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => t * (2.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            EasingFunction::EaseInQuad => t * t,
            EasingFunction::EaseOutQuad => t * (2.0 - t),
            EasingFunction::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EasingFunction::EaseInCubic => t * t * t,
            EasingFunction::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            EasingFunction::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            EasingFunction::EaseInElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c4 = (2.0 * core::f32::consts::PI) / 3.0;
                    -(2.0_f32.powf(10.0 * t - 10.0)) * libm::sinf((t * 10.0 - 10.75) * c4)
                }
            }
            EasingFunction::EaseOutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c4 = (2.0 * core::f32::consts::PI) / 3.0;
                    2.0_f32.powf(-10.0 * t) * libm::sinf((t * 10.0 - 0.75) * c4) + 1.0
                }
            }
            EasingFunction::Bounce => {
                let n1 = 7.5625;
                let d1 = 2.75;
                
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
        }
    }
}

// Animation group for coordinating multiple animations
pub struct AnimationGroup {
    animations: heapless::Vec<Animation, 8>,
    parallel: bool,
    current_index: usize,
}

impl AnimationGroup {
    pub fn new(parallel: bool) -> Self {
        Self {
            animations: heapless::Vec::new(),
            parallel,
            current_index: 0,
        }
    }
    
    pub fn add(&mut self, animation: Animation) -> Result<(), Animation> {
        self.animations.push(animation)
    }
    
    pub fn start(&mut self) {
        if self.parallel {
            // Start all animations at once
            for anim in &mut self.animations {
                anim.start();
            }
        } else {
            // Start only the first animation
            if let Some(anim) = self.animations.get_mut(0) {
                anim.start();
            }
        }
        self.current_index = 0;
    }
    
    pub fn update(&mut self) -> bool {
        if self.parallel {
            // Update all animations
            let mut all_completed = true;
            for anim in &mut self.animations {
                anim.update();
                if !anim.is_completed() {
                    all_completed = false;
                }
            }
            all_completed
        } else {
            // Update current animation
            if let Some(anim) = self.animations.get_mut(self.current_index) {
                anim.update();
                
                if anim.is_completed() {
                    self.current_index += 1;
                    
                    // Start next animation if available
                    if let Some(next_anim) = self.animations.get_mut(self.current_index) {
                        next_anim.start();
                        false
                    } else {
                        // All animations completed
                        true
                    }
                } else {
                    false
                }
            } else {
                true
            }
        }
    }
    
    pub fn is_completed(&self) -> bool {
        if self.parallel {
            self.animations.iter().all(|anim| anim.is_completed())
        } else {
            self.current_index >= self.animations.len()
        }
    }
    
    pub fn reset(&mut self) {
        for anim in &mut self.animations {
            anim.reset();
        }
        self.current_index = 0;
    }
}

// Interpolation utilities
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

pub fn lerp_color(start: u16, end: u16, t: f32) -> u16 {
    let start_r = (start >> 11) & 0x1F;
    let start_g = (start >> 5) & 0x3F;
    let start_b = start & 0x1F;
    
    let end_r = (end >> 11) & 0x1F;
    let end_g = (end >> 5) & 0x3F;
    let end_b = end & 0x1F;
    
    let r = lerp(start_r as f32, end_r as f32, t) as u16;
    let g = lerp(start_g as f32, end_g as f32, t) as u16;
    let b = lerp(start_b as f32, end_b as f32, t) as u16;
    
    (r << 11) | (g << 5) | b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_linear_easing() {
        let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::Linear);
        assert_eq!(anim.apply_easing(0.0), 0.0);
        assert_eq!(anim.apply_easing(0.5), 0.5);
        assert_eq!(anim.apply_easing(1.0), 1.0);
    }
    
    #[test]
    fn test_ease_in_out() {
        let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::EaseInOut);
        // Should start slow
        assert!(anim.apply_easing(0.1) < 0.1);
        // Should be roughly linear in the middle
        assert!((anim.apply_easing(0.5) - 0.5).abs() < 0.01);
        // Should end slow
        assert!(anim.apply_easing(0.9) > 0.9);
    }
    
    #[test]
    fn test_color_lerp() {
        // Test interpolating from black to white
        let black = 0xFFFF; // BGR565 black
        let white = 0x0000; // BGR565 white
        
        assert_eq!(lerp_color(black, white, 0.0), black);
        assert_eq!(lerp_color(black, white, 1.0), white);
        
        // Middle value should be gray
        let mid = lerp_color(black, white, 0.5);
        assert_eq!((mid >> 11) & 0x1F, 15); // Half red
        assert_eq!((mid >> 5) & 0x3F, 31);  // Half green
        assert_eq!(mid & 0x1F, 15);          // Half blue
    }
}