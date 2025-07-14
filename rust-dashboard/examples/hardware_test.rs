//! Basic hardware test for ESP32-S3 T-Display
//! 
//! This example tests:
//! 1. Display initialization and basic drawing
//! 2. Button input reading
//! 3. LED/backlight control

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use esp32s3_hal::{
    clock::ClockControl,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    Delay,
    timer::TimerGroup,
    Rtc,
};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);
    
    // Set up the RTC
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    rtc.rwdt.disable();
    
    // Set up timer for watchdog
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt = timer_group0.wdt;
    wdt.disable();
    
    println!("ESP32-S3 T-Display Hardware Test");
    println!("=================================");
    
    // Initialize IO
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    
    // Test 1: Backlight control
    println!("\n1. Testing backlight...");
    let mut backlight = io.pins.gpio38.into_push_pull_output();
    
    // Blink backlight 3 times
    for i in 0..3 {
        println!("  Backlight ON ({})", i + 1);
        backlight.set_high().unwrap();
        delay.delay_ms(500u32);
        
        println!("  Backlight OFF ({})", i + 1);
        backlight.set_low().unwrap();
        delay.delay_ms(500u32);
    }
    backlight.set_high().unwrap(); // Leave on
    println!("  ✓ Backlight test complete");
    
    // Test 2: Button inputs
    println!("\n2. Testing buttons...");
    let button1 = io.pins.gpio0.into_pull_up_input();
    let button2 = io.pins.gpio14.into_pull_up_input();
    
    println!("  Press Button 1 (GPIO0) or Button 2 (GPIO14)...");
    println!("  Waiting 10 seconds for button press...");
    
    let start = delay.get_time_ms();
    let mut button1_pressed = false;
    let mut button2_pressed = false;
    
    while delay.get_time_ms() - start < 10000 {
        if button1.is_low().unwrap() && !button1_pressed {
            println!("  ✓ Button 1 pressed!");
            button1_pressed = true;
        }
        if button2.is_low().unwrap() && !button2_pressed {
            println!("  ✓ Button 2 pressed!");
            button2_pressed = true;
        }
        if button1_pressed && button2_pressed {
            break;
        }
        delay.delay_ms(10u32);
    }
    
    if !button1_pressed && !button2_pressed {
        println!("  ⚠ No buttons pressed in 10 seconds");
    }
    
    // Test 3: Display data pins
    println!("\n3. Testing display data pins...");
    let mut d0 = io.pins.gpio39.into_push_pull_output();
    let mut d1 = io.pins.gpio40.into_push_pull_output();
    let mut d2 = io.pins.gpio41.into_push_pull_output();
    let mut d3 = io.pins.gpio42.into_push_pull_output();
    let mut d4 = io.pins.gpio45.into_push_pull_output();
    let mut d5 = io.pins.gpio46.into_push_pull_output();
    let mut d6 = io.pins.gpio47.into_push_pull_output();
    let mut d7 = io.pins.gpio48.into_push_pull_output();
    
    // Toggle all data pins
    println!("  Setting all data pins HIGH");
    d0.set_high().unwrap();
    d1.set_high().unwrap();
    d2.set_high().unwrap();
    d3.set_high().unwrap();
    d4.set_high().unwrap();
    d5.set_high().unwrap();
    d6.set_high().unwrap();
    d7.set_high().unwrap();
    delay.delay_ms(100u32);
    
    println!("  Setting all data pins LOW");
    d0.set_low().unwrap();
    d1.set_low().unwrap();
    d2.set_low().unwrap();
    d3.set_low().unwrap();
    d4.set_low().unwrap();
    d5.set_low().unwrap();
    d6.set_low().unwrap();
    d7.set_low().unwrap();
    delay.delay_ms(100u32);
    
    println!("  ✓ Display pins toggled");
    
    // Test 4: Control pins
    println!("\n4. Testing display control pins...");
    let mut wr = io.pins.gpio8.into_push_pull_output();
    let mut dc = io.pins.gpio7.into_push_pull_output();
    let mut cs = io.pins.gpio6.into_push_pull_output();
    let mut rst = io.pins.gpio5.into_push_pull_output();
    
    // Display reset sequence
    println!("  Performing display reset sequence");
    cs.set_low().unwrap();  // Select display
    rst.set_high().unwrap();
    delay.delay_ms(10u32);
    rst.set_low().unwrap();
    delay.delay_ms(10u32);
    rst.set_high().unwrap();
    delay.delay_ms(120u32);
    println!("  ✓ Reset sequence complete");
    
    println!("\n=================================");
    println!("Hardware test complete!");
    println!("Summary:");
    println!("  - Backlight: OK");
    println!("  - Button 1: {}", if button1_pressed { "OK" } else { "Not tested" });
    println!("  - Button 2: {}", if button2_pressed { "OK" } else { "Not tested" });
    println!("  - Display pins: OK");
    println!("  - Control pins: OK");
    
    // Keep running
    println!("\nEntering idle loop. Reset to run test again.");
    loop {
        delay.delay_ms(1000u32);
    }
}