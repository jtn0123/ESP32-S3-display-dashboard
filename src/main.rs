use anyhow::Result;
use esp_idf_hal::prelude::*;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    log::EspLogger,
    timer::EspTaskTimerService,
};
use esp_idf_sys as _; // Binstart
use std::sync::{Arc, Mutex};

use log::info;

mod config;
mod display;
mod network;
mod sensors;
mod system;
mod ui;

use crate::display::DisplayManager;
use crate::network::NetworkManager;
use crate::ui::UiManager;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32-S3 Dashboard starting...");
    info!("Free heap: {} bytes", unsafe {
        esp_idf_sys::esp_get_free_heap_size()
    });

    // Take peripherals and system event loop
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;

    // Load configuration
    let config = Arc::new(Mutex::new(config::load_or_default()?));
    info!("Configuration loaded");

    // Initialize display
    info!("Initializing display...");
    
    // Turn on LCD power first (GPIO 15)
    use esp_idf_hal::gpio::PinDriver;
    let mut lcd_power = PinDriver::output(peripherals.pins.gpio15)?;
    lcd_power.set_high()?;
    info!("LCD power enabled on GPIO 15");
    
    // Small delay to ensure LCD power is stable
    use esp_idf_hal::delay::Ets;
    Ets::delay_ms(10);
    
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
    )?;
    info!("Display initialized");

    // Initialize UI
    let mut ui_manager = UiManager::new(&mut display_manager)?;
    ui_manager.show_boot_screen(&mut display_manager)?;

    // Initialize sensors
    let battery_pin = peripherals.pins.gpio4;
    let sensor_manager = sensors::SensorManager::new(battery_pin)?;

    // Initialize buttons
    let button1 = peripherals.pins.gpio0;
    let button2 = peripherals.pins.gpio14;
    let button_manager = system::ButtonManager::new(button1, button2)?;

    // Initialize network (WiFi + OTA)
    info!("Initializing network...");
    let network_config = config.lock().unwrap();
    let network_manager = NetworkManager::new(
        peripherals.modem,
        sys_loop,
        timer_service,
        network_config.wifi_ssid.clone(),
        network_config.wifi_password.clone(),
        config.clone(),
    )?;
    drop(network_config);

    // Start main application loop
    info!("Starting main loop");
    run_app(
        ui_manager,
        display_manager,
        sensor_manager,
        button_manager,
        network_manager,
        config,
    )?;

    Ok(())
}

fn run_app(
    mut ui_manager: UiManager,
    mut display_manager: DisplayManager,
    mut sensor_manager: sensors::SensorManager,
    mut button_manager: system::ButtonManager,
    mut network_manager: NetworkManager,
    config: Arc<Mutex<config::Config>>,
) -> Result<()> {
    use std::thread;
    use std::time::{Duration, Instant};

    // Spawn network thread
    let config_clone = config.clone();
    thread::spawn(move || {
        if let Err(e) = network_manager.run(config_clone) {
            log::error!("Network thread error: {:?}", e);
        }
    });

    // Main UI loop
    let target_frame_time = Duration::from_millis(33); // ~30 FPS
    let mut last_sensor_update = Instant::now();
    let sensor_update_interval = Duration::from_secs(5);

    loop {
        let frame_start = Instant::now();

        // Handle button input
        if let Some(event) = button_manager.poll() {
            ui_manager.handle_button_event(event)?;
        }

        // Update sensors periodically
        if last_sensor_update.elapsed() >= sensor_update_interval {
            let sensor_data = sensor_manager.read_all()?;
            ui_manager.update_sensor_data(sensor_data);
            last_sensor_update = Instant::now();
        }

        // Update and render UI
        ui_manager.update()?;
        ui_manager.render(&mut display_manager)?;
        display_manager.flush()?;

        // Frame rate limiting
        let frame_time = frame_start.elapsed();
        if frame_time < target_frame_time {
            thread::sleep(target_frame_time - frame_time);
        } else {
            log::warn!("Frame took {:?} (target: {:?})", frame_time, target_frame_time);
        }
    }
}