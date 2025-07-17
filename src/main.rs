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

// Generate ESP-IDF app descriptor
// Note: This macro generates warnings about cfg conditions but they're harmless
#[allow(unexpected_cfgs)]
mod app_desc {
    esp_idf_sys::esp_app_desc!();
}

// For better crash diagnostics
// extern "C" {
//     fn esp_backtrace_print_app_description();
// }

mod config;
mod display;
mod network;
mod sensors;
mod system;
mod ui;

use crate::display::{DisplayManager, colors};
use crate::network::NetworkManager;
use crate::ui::UiManager;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32-S3 Dashboard v4.11 - UI Rendering Enabled");
    info!("Free heap: {} bytes", unsafe {
        esp_idf_sys::esp_get_free_heap_size()
    });
    
    // Reconfigure watchdog timeout to 5 seconds
    unsafe {
        // First deinit if already initialized
        let _ = esp_idf_sys::esp_task_wdt_deinit();
        
        // Then init with new config
        let wdt_config = esp_idf_sys::esp_task_wdt_config_t {
            timeout_ms: 5000,
            idle_core_mask: 0,
            trigger_panic: false,
        };
        let result = esp_idf_sys::esp_task_wdt_init(&wdt_config as *const _);
        if result == esp_idf_sys::ESP_OK {
            info!("Watchdog timeout set to 5 seconds");
        } else {
            log::warn!("Watchdog reconfiguration failed: {:?}", result);
        }
    }
    
    // Configure power management for dynamic frequency scaling
    unsafe {
        use esp_idf_sys::*;
        let pm_config = esp_pm_config_esp32s3_t {
            max_freq_mhz: 240,  // Maximum frequency
            min_freq_mhz: 80,   // Minimum frequency when idle
            light_sleep_enable: false, // Keep false for responsiveness
        };
        let result = esp_pm_configure(&pm_config as *const esp_pm_config_esp32s3_t as *const core::ffi::c_void);
        if result == ESP_OK {
            info!("Power management configured: 80-240MHz DFS");
        } else {
            log::warn!("Failed to configure power management: {:?}", result);
        }
    }

    // Take peripherals and system event loop
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;

    // Load configuration
    let config = Arc::new(Mutex::new(config::load_or_default()?));
    info!("Configuration loaded");

    // Initialize display
    info!("Initializing display...");
    
    // Initialize display pins and power sequence
    info!("Initializing display with proper pin management...");
    
    // CRITICAL: Wait for power to stabilize before initializing display
    use esp_idf_hal::delay::Ets;
    Ets::delay_ms(500);  // Longer delay for power stability
    
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
        peripherals.pins.gpio15, // LCD Power - CRITICAL!
        peripherals.pins.gpio9,  // RD pin
    )?;
    info!("Display initialized - LCD power and backlight pins kept alive");

    // Test display is working with a simple color fill first
    info!("Testing display with color fill...");
    display_manager.clear(colors::PRIMARY_RED)?;
    display_manager.flush()?;
    Ets::delay_ms(500);
    
    // TEMPORARY: Color toggle test to isolate display vs UI issues
    let color_toggle_test = false; // Set to true to enable test mode
    if color_toggle_test {
        info!("Starting color toggle test loop...");
        let mut color = 0xF800u16; // Start with red
        loop {
            display_manager.clear(color)?;
            display_manager.flush()?;
            color ^= 0xFFFF; // Toggle between red and cyan
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            info!("Color toggled to: 0x{:04X}", color);
        }
    }
    
    // Initialize UI
    let mut ui_manager = UiManager::new(&mut display_manager)?;
    ui_manager.show_boot_screen(&mut display_manager)?;
    display_manager.flush()?;
    info!("Boot screen displayed");
    
    // Keep boot screen visible for a moment
    log::info!("Boot screen complete, waiting 1 second before continuing...");
    Ets::delay_ms(1000);
    log::info!("Continuing with initialization...");

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
    let mut network_manager = NetworkManager::new(
        peripherals.modem,
        sys_loop,
        timer_service,
        network_config.wifi_ssid.clone(),
        network_config.wifi_password.clone(),
        config.clone(),
    )?;
    drop(network_config);

    // Try to connect to WiFi but don't fail if it doesn't work
    match network_manager.connect() {
        Ok(_) => info!("WiFi connected successfully"),
        Err(e) => {
            log::warn!("WiFi connection failed: {:?}", e);
            log::warn!("Continuing without network connectivity");
        }
    }

    // Start web server (needs to be in main thread due to raw pointers)
    let web_server = match network::web_server::WebConfigServer::new(config.clone()) {
        Ok(server) => {
            log::info!("Web configuration server started on port 80");
            Some(server)
        }
        Err(e) => {
            log::error!("Failed to start web server: {:?}", e);
            None
        }
    };

    // Start main application loop
    info!("Starting main loop - UI should now be visible");
    
    // Ensure backlight is on before entering main loop
    display_manager.update_auto_dim()?;
    
    // Display is already initialized and on - no need for additional commands
    
    // Small delay before main loop
    Ets::delay_ms(100);
    
    // Draw a test marker before entering main loop
    info!("Drawing pre-loop test marker at (160, 85)");
    display_manager.fill_rect(160, 85, 10, 10, colors::YELLOW)?;
    display_manager.flush()?;
    
    info!("Entering run_app function now...");
    
    // Run the main app with crash recovery
    match run_app(
        ui_manager,
        display_manager,
        sensor_manager,
        button_manager,
        network_manager,
        config,
        web_server,
    ) {
        Ok(_) => {
            log::warn!("UI loop exited normally (shouldn't happen)");
        }
        Err(e) => {
            log::error!("UI loop crashed: {:?}", e);
            log::error!("Restarting system to recover...");
            unsafe { esp_idf_sys::esp_restart(); }
        }
    }

    Ok(())
}

fn run_app(
    mut ui_manager: UiManager,
    mut display_manager: DisplayManager,
    mut sensor_manager: sensors::SensorManager,
    mut button_manager: system::ButtonManager,
    network_manager: NetworkManager,
    _config: Arc<Mutex<config::Config>>,
    _web_server: Option<network::web_server::WebConfigServer>,
) -> Result<()> {
    use std::thread;
    use std::time::{Duration, Instant};

    // Note: OTA checker would run in the main loop instead of a separate thread
    // due to thread safety constraints with ESP-IDF HTTP server

    // Main UI loop with performance telemetry
    let target_frame_time = Duration::from_millis(33); // ~30 FPS
    let mut last_sensor_update = Instant::now();
    let sensor_update_interval = Duration::from_secs(5);
    
    // Performance tracking
    let mut frame_count = 0u32;
    let mut last_fps_report = Instant::now();
    let mut total_frame_time = Duration::ZERO;
    let mut max_frame_time = Duration::ZERO;
    
    log::info!("Main render loop started - entering infinite loop");

    loop {
        let frame_start = Instant::now();
        
        // Log first few frames to debug
        if frame_count < 5 {
            log::info!("Rendering frame {}", frame_count);
        }

        // Handle button input
        if let Some(event) = button_manager.poll() {
            ui_manager.handle_button_event(event)?;
            // Reset activity timer on button press
            display_manager.reset_activity_timer();
        }

        // Update sensors periodically
        if last_sensor_update.elapsed() >= sensor_update_interval {
            let sensor_data = sensor_manager.sample()?;
            ui_manager.update_sensor_data(sensor_data);
            
            // Update network status
            ui_manager.update_network_status(
                network_manager.is_connected(),
                network_manager.get_ip(),
                network_manager.get_ssid().to_string()
            );
            
            last_sensor_update = Instant::now();
        }

        // Update and render UI
        ui_manager.update()?;
        if frame_count < 5 {
            log::info!("Main loop: About to render frame {}", frame_count);
        }
        ui_manager.render(&mut display_manager)?;
        if frame_count < 5 {
            log::info!("Main loop: Render complete for frame {}", frame_count);
        }
        
        // Update auto-dim
        display_manager.update_auto_dim()?;
        
        display_manager.flush()?;
        if frame_count < 5 {
            log::info!("Main loop: Flush complete for frame {}", frame_count);
        }

        // Frame timing and telemetry
        let frame_time = frame_start.elapsed();
        frame_count += 1;
        total_frame_time += frame_time;
        max_frame_time = max_frame_time.max(frame_time);
        
        // Report FPS every second
        if last_fps_report.elapsed() >= Duration::from_secs(1) {
            let avg_frame_time = total_frame_time / frame_count;
            let fps = (frame_count as f32) / last_fps_report.elapsed().as_secs_f32();
            
            log::info!("[PERF] FPS: {:.1} | Avg frame: {:?} | Max frame: {:?} | Heap free: {} KB",
                fps,
                avg_frame_time,
                max_frame_time,
                unsafe { esp_idf_sys::esp_get_free_heap_size() } / 1024
            );
            
            // Reset counters
            frame_count = 0;
            total_frame_time = Duration::ZERO;
            max_frame_time = Duration::ZERO;
            last_fps_report = Instant::now();
        }
        
        // Frame rate limiting
        if frame_time < target_frame_time {
            thread::sleep(target_frame_time - frame_time);
        }
    }
}