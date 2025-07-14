#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    dma::{Dma, DmaPriority},
    embassy,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    timer::TimerGroup,
};
use esp_println::println;
use static_cell::make_static;

mod display;
mod hardware;
mod ui;
mod ota;
mod animation;
mod power;

#[cfg(test)]
mod tests;

use display::{Display, DisplayPins, Color};
use hardware::{ButtonManager, ButtonEvent};
use ui::{Dashboard};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    println!("ESP32-S3 Rust Dashboard Starting...");
    
    // Initialize peripherals
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();
    
    // Initialize Embassy
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);
    
    // Initialize IO
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    
    // Initialize DMA
    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;
    
    // Configure display pins
    let display_pins = DisplayPins {
        d0: io.pins.gpio39.degrade(),
        d1: io.pins.gpio40.degrade(),
        d2: io.pins.gpio41.degrade(),
        d3: io.pins.gpio42.degrade(),
        d4: io.pins.gpio45.degrade(),
        d5: io.pins.gpio46.degrade(),
        d6: io.pins.gpio47.degrade(),
        d7: io.pins.gpio48.degrade(),
        wr: io.pins.gpio8.degrade(),
    };
    
    // Configure control pins
    let dc_pin = io.pins.gpio7.into_push_pull_output();
    let cs_pin = io.pins.gpio6.into_push_pull_output();
    let rst_pin = io.pins.gpio5.into_push_pull_output();
    let backlight_pin = io.pins.gpio38.into_push_pull_output();
    
    // Configure button pins
    let button1_pin = io.pins.gpio0.into_pull_up_input();
    let button2_pin = io.pins.gpio14.into_pull_up_input();
    
    // Initialize display
    println!("Initializing display...");
    let display = match Display::new(
        peripherals.LCD_CAM,
        dma_channel,
        display_pins,
        dc_pin,
        cs_pin,
        rst_pin,
        backlight_pin,
    ) {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to initialize display: {}", e);
            panic!("Display initialization failed");
        }
    };
    
    // Create button manager
    let button_manager = ButtonManager::new(button1_pin, button2_pin);
    
    // Create dashboard
    let dashboard = Dashboard::new(display);
    
    // Make static for tasks
    let dashboard = make_static!(dashboard);
    let button_manager = make_static!(button_manager);
    
    // Spawn tasks
    spawner.spawn(display_task(dashboard)).ok();
    spawner.spawn(button_task(button_manager, dashboard)).ok();
    spawner.spawn(sensor_task()).ok();
    
    println!("Dashboard initialized successfully!");
}

#[embassy_executor::task]
async fn display_task(dashboard: &'static mut Dashboard) {
    println!("Display task started");
    
    // Initial render
    dashboard.render().await;
    
    loop {
        // Update display at 30 FPS
        dashboard.update().await;
        dashboard.render().await;
        Timer::after(Duration::from_millis(33)).await;
    }
}

#[embassy_executor::task]
async fn button_task(
    button_manager: &'static mut ButtonManager,
    dashboard: &'static mut Dashboard,
) {
    println!("Button task started");
    
    loop {
        // Check for button events
        if let Some(event) = button_manager.poll() {
            match event {
                ButtonEvent::Button1Click => {
                    println!("Button 1 clicked - previous screen");
                    dashboard.previous_screen();
                }
                ButtonEvent::Button2Click => {
                    println!("Button 2 clicked - next screen");
                    dashboard.next_screen();
                }
                ButtonEvent::Button1LongPress => {
                    println!("Button 1 long press - menu");
                    dashboard.show_menu();
                }
                ButtonEvent::Button2LongPress => {
                    println!("Button 2 long press - select");
                    dashboard.select();
                }
                _ => {}
            }
        }
        
        Timer::after(Duration::from_millis(10)).await;
    }
}

#[embassy_executor::task]
async fn sensor_task() {
    println!("Sensor task started");
    
    loop {
        // TODO: Read battery voltage
        // TODO: Read temperature
        // TODO: Update sensor data
        
        Timer::after(Duration::from_secs(5)).await;
    }
}

// WiFi and OTA task will be added when we have std support
// For no_std, we need a different approach for networking
// #[embassy_executor::task]
// async fn network_task() {
//     // TODO: Initialize WiFi
//     // TODO: Start OTA web server
//     // TODO: Handle OTA updates
// }