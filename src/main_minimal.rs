use anyhow::Result;
use esp_idf_hal::prelude::*;
use esp_idf_svc::{
    log::EspLogger,
};
use esp_idf_sys as _; // Binstart
use log::info;

// Generate ESP-IDF app descriptor
#[allow(unexpected_cfgs)]
mod app_desc {
    esp_idf_sys::esp_app_desc!();
}

mod display;
use crate::display::{DisplayManager, colors};

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32-S3 Minimal Test v1.0");
    
    // Take peripherals
    let peripherals = Peripherals::take()?;
    
    // Wait for stability
    use esp_idf_hal::delay::Ets;
    Ets::delay_ms(1000);
    
    // Initialize display
    info!("Initializing display...");
    let mut display_manager = DisplayManager::new(
        peripherals.pins.gpio39, // D0
        peripherals.pins.gpio40, // D1
        peripherals.pins.gpio41, // D2
        peripherals.pins.gpio42, // D3
        peripherals.pins.gpio45, // D4
        peripherals.pins.gpio46, // D5
        peripherals.pins.gpio47, // D6
        peripherals.pins.gpio48, // D7
        peripherals.pins.gpio8,  // WR
        peripherals.pins.gpio7,  // DC
        peripherals.pins.gpio6,  // CS
        peripherals.pins.gpio5,  // RST
        peripherals.pins.gpio38, // Backlight
        peripherals.pins.gpio15, // LCD Power
        peripherals.pins.gpio9,  // RD pin
    )?;
    
    info!("Display initialized");
    
    // Simple test pattern
    display_manager.clear(colors::BLACK)?;
    display_manager.flush()?;
    Ets::delay_ms(500);
    
    // Draw a simple rectangle
    display_manager.fill_rect(50, 50, 200, 68, colors::PRIMARY_BLUE)?;
    display_manager.flush()?;
    
    // Draw text
    display_manager.draw_text_centered(84, "Display Test OK", colors::WHITE, None, 2)?;
    display_manager.flush()?;
    
    info!("Test pattern displayed");
    
    // Keep the display on
    loop {
        Ets::delay_ms(1000);
        info!("Still running...");
    }
}