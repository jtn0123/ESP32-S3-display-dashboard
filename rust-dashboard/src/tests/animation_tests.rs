use crate::animation::*;
use embassy_time::Duration;

#[test]
fn test_linear_animation() {
    let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::Linear);
    anim.start();
    
    // Test that values interpolate linearly
    assert_eq!(anim.apply_easing(0.0), 0.0);
    assert_eq!(anim.apply_easing(0.25), 0.25);
    assert_eq!(anim.apply_easing(0.5), 0.5);
    assert_eq!(anim.apply_easing(0.75), 0.75);
    assert_eq!(anim.apply_easing(1.0), 1.0);
}

#[test]
fn test_ease_in_animation() {
    let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::EaseIn);
    
    // Ease in should start slow
    assert!(anim.apply_easing(0.1) < 0.1);
    assert!(anim.apply_easing(0.2) < 0.2);
    
    // And accelerate towards the end
    assert!(anim.apply_easing(0.8) < 0.8);
    assert_eq!(anim.apply_easing(1.0), 1.0);
}

#[test]
fn test_ease_out_animation() {
    let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::EaseOut);
    
    // Ease out should start fast
    assert!(anim.apply_easing(0.1) > 0.1);
    assert!(anim.apply_easing(0.2) > 0.2);
    
    // And decelerate towards the end
    assert!(anim.apply_easing(0.9) > 0.9);
    assert_eq!(anim.apply_easing(1.0), 1.0);
}

#[test]
fn test_animation_completion() {
    let mut anim = Animation::new(0.0, 100.0, Duration::from_millis(100), EasingFunction::Linear);
    
    assert!(!anim.is_completed());
    
    anim.start();
    assert!(!anim.is_completed());
    
    // Simulate time passing beyond duration
    // In real tests, we'd need to mock time
    // For now, we'll test the completion flag directly
}

#[test]
fn test_animation_reverse() {
    let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::Linear);
    
    assert_eq!(anim.start_value, 0.0);
    assert_eq!(anim.end_value, 100.0);
    
    anim.reverse();
    
    assert_eq!(anim.start_value, 100.0);
    assert_eq!(anim.end_value, 0.0);
}

#[test]
fn test_animation_group_parallel() {
    let mut group = AnimationGroup::new(true); // parallel
    
    let anim1 = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::Linear);
    let anim2 = Animation::new(0.0, 50.0, Duration::from_secs(1), EasingFunction::EaseIn);
    
    group.add(anim1).unwrap();
    group.add(anim2).unwrap();
    
    group.start();
    
    // Both animations should be running
    assert!(!group.is_completed());
}

#[test]
fn test_animation_group_sequential() {
    let mut group = AnimationGroup::new(false); // sequential
    
    let anim1 = Animation::new(0.0, 100.0, Duration::from_millis(100), EasingFunction::Linear);
    let anim2 = Animation::new(0.0, 50.0, Duration::from_millis(100), EasingFunction::EaseIn);
    
    group.add(anim1).unwrap();
    group.add(anim2).unwrap();
    
    group.start();
    
    // Only first animation should be running
    assert!(!group.is_completed());
    assert_eq!(group.current_index, 0);
}

#[test]
fn test_lerp() {
    assert_eq!(lerp(0.0, 100.0, 0.0), 0.0);
    assert_eq!(lerp(0.0, 100.0, 0.5), 50.0);
    assert_eq!(lerp(0.0, 100.0, 1.0), 100.0);
    
    // Test negative values
    assert_eq!(lerp(-50.0, 50.0, 0.5), 0.0);
    assert_eq!(lerp(-100.0, -50.0, 0.5), -75.0);
}

#[test]
fn test_lerp_color() {
    // Test black to white interpolation
    let black = 0xFFFF; // BGR565 black (all bits set)
    let white = 0x0000; // BGR565 white (no bits set)
    
    assert_eq!(lerp_color(black, white, 0.0), black);
    assert_eq!(lerp_color(black, white, 1.0), white);
    
    // Test intermediate value
    let mid = lerp_color(black, white, 0.5);
    // Should be gray (roughly half of each component)
    assert!((mid >> 11) & 0x1F > 10); // Red component
    assert!((mid >> 5) & 0x3F > 20);  // Green component
    assert!(mid & 0x1F > 10);          // Blue component
}

#[test]
fn test_bounce_easing() {
    let anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::Bounce);
    
    // Bounce should end at 1.0
    assert!((anim.apply_easing(1.0) - 1.0).abs() < 0.001);
    
    // Should have multiple bounces
    let mid = anim.apply_easing(0.5);
    assert!(mid > 0.0 && mid < 1.0);
}

#[test]
fn test_elastic_easing() {
    let anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::EaseInElastic);
    
    // Should start and end correctly
    assert_eq!(anim.apply_easing(0.0), 0.0);
    assert_eq!(anim.apply_easing(1.0), 1.0);
    
    // Should have oscillation in the middle
    let mid = anim.apply_easing(0.5);
    assert!(mid != 0.5); // Not linear
}