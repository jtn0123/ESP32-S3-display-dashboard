// Example demonstrating the animation system

use rust_dashboard::animation::*;

fn main() {
    println!("Animation System Demo");
    println!("====================");
    
    // Create a simple animation
    let mut anim = Animation::new(0.0, 100.0, Duration::from_secs(1), EasingFunction::EaseInOut);
    
    println!("\nLinear interpolation test:");
    println!("lerp(0, 100, 0.0) = {}", lerp(0.0, 100.0, 0.0));
    println!("lerp(0, 100, 0.5) = {}", lerp(0.0, 100.0, 0.5));
    println!("lerp(0, 100, 1.0) = {}", lerp(0.0, 100.0, 1.0));
    
    println!("\nColor interpolation test:");
    let black = 0xFFFF; // BGR565 black
    let white = 0x0000; // BGR565 white
    let mid = lerp_color(black, white, 0.5);
    println!("Black: 0x{:04X}", black);
    println!("White: 0x{:04X}", white);
    println!("Mid (gray): 0x{:04X}", mid);
    
    println!("\nAnimation test:");
    anim.start();
    println!("Animation started, initial value: {}", anim.update());
    println!("Is completed: {}", anim.is_completed());
    
    println!("\nAnimation group test:");
    let mut group = AnimationGroup::new(false); // Sequential
    group.add(Animation::new(0.0, 50.0, Duration::from_millis(500), EasingFunction::Linear)).unwrap();
    group.add(Animation::new(50.0, 100.0, Duration::from_millis(500), EasingFunction::EaseOut)).unwrap();
    println!("Created animation group with 2 sequential animations");
    
    group.start();
    println!("Group started, is completed: {}", group.is_completed());
    
    println!("\nDuration test:");
    let d = Duration::from_secs(5);
    println!("5 seconds = {} milliseconds", d.as_millis());
}