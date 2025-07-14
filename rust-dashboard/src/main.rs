use esp_idf_sys as _; // If using esp-idf-sys (bindings need this)
use esp_idf_hal::prelude::*;

use anyhow::Result;
use log::*;

mod display;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-sys/issues/139
    esp_idf_sys::link_patches();
    
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    
    info!("ESP32-S3 Rust Dashboard Starting...");
    
    // Take peripherals
    let peripherals = Peripherals::take()?;
    
    // Initialize display with LCD_CAM
    info!("Initializing LCD_CAM display driver...");
    
    // TODO: Initialize display
    // let mut display = display::Display::new(peripherals)?;
    
    info!("Starting main loop");
    
    loop {
        // Main application loop
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}