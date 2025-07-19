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

mod boot;
mod config;
mod display;
// mod hardware;  // Temporarily disabled - API changes
mod network;
mod ota;
mod sensors;
mod system;
mod ui;
mod version;

use crate::boot::{BootManager, BootStage};
use crate::display::{DisplayManager, colors};
use crate::network::NetworkManager;
use crate::ota::OtaManager;
use crate::ui::UiManager;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32-S3 Dashboard {} - OTA on Port 80", crate::version::full_version());
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
            
            // Add current task to watchdog monitoring
            let add_result = esp_idf_sys::esp_task_wdt_add(std::ptr::null_mut());
            if add_result == esp_idf_sys::ESP_OK {
                info!("Current task added to watchdog monitoring");
            } else {
                log::warn!("Failed to add task to watchdog: {:?}", add_result);
            }
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

    // Initialize boot manager for animated boot experience
    info!("Starting enhanced boot sequence...");
    let mut boot_manager = BootManager::new();
    
    // Show initial boot screen
    boot_manager.set_stage(BootStage::DisplayInit);
    log::info!("Boot: Setting stage to DisplayInit");
    boot_manager.render_boot_screen(&mut display_manager)?;
    display_manager.flush()?;
    log::info!("Boot: Initial boot screen rendered");
    
    // Animate for a moment while display stabilizes
    for i in 0..10 {
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
        
        // Reset watchdog every few frames
        if i % 3 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
        
        Ets::delay_ms(50);
    }
    
    // Memory initialization with progress
    boot_manager.set_stage(BootStage::MemoryInit);
    log::info!("Boot: Setting stage to MemoryInit");
    for i in 0..5 {
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
        
        // Reset watchdog
        if i % 2 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
        
        Ets::delay_ms(100);
    }
    log::info!("Boot: Memory init animation complete");
    
    // Initialize UI
    info!("Creating UI manager...");
    boot_manager.set_stage(BootStage::UISetup);
    boot_manager.render_boot_screen(&mut display_manager)?;
    display_manager.flush()?;
    
    let ui_manager = UiManager::new(&mut display_manager)?;
    info!("UI manager created");

    // Initialize sensors
    boot_manager.set_stage(BootStage::SensorInit);
    log::info!("Boot: Setting stage to SensorInit");
    boot_manager.render_boot_screen(&mut display_manager)?;
    display_manager.flush()?;
    log::info!("Boot: Sensor init screen rendered");
    
    let battery_pin = peripherals.pins.gpio4;
    let adc1 = peripherals.adc1;
    let sensor_manager = sensors::SensorManager::new(adc1, battery_pin)?;
    
    // Animate progress
    for i in 0..3 {
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
        
        // Reset watchdog
        if i == 1 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
        
        Ets::delay_ms(50);
    }

    // Initialize buttons
    let button1 = peripherals.pins.gpio0;
    let button2 = peripherals.pins.gpio14;
    let button_manager = system::ButtonManager::new(button1, button2)?;

    // Initialize network (WiFi + OTA)
    info!("Initializing network...");
    boot_manager.set_stage(BootStage::NetworkInit);
    log::info!("Boot: Setting stage to NetworkInit");
    boot_manager.render_boot_screen(&mut display_manager)?;
    display_manager.flush()?;
    log::info!("Boot: Network init screen rendered - display should still be visible");
    
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
    
    // Keep animating during network init
    log::info!("Boot: Keeping display alive during network stage");
    for i in 0..10 {
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
        display_manager.update_auto_dim()?; // Keep display alive
        
        // Extra safety - ensure power pins stay high
        display_manager.ensure_display_on()?;
        
        // Reset watchdog
        if i % 2 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
        
        if i % 5 == 0 {
            log::info!("Boot: Display still active during network init loop {}", i);
        }
        Ets::delay_ms(50);
    }
    log::info!("Boot: Network animation complete, display should still be on");

    // Try to connect to WiFi with display keep-alive
    info!("Attempting WiFi connection...");
    
    // Connect to WiFi with periodic display updates to prevent timeout
    let wifi_result = {
        let mut network_manager = network_manager; // Make it mutable for connection
        
        // Try to connect - this is a blocking operation
        match network_manager.connect() {
            Ok(_) => {
                info!("WiFi connected successfully");
                Ok(network_manager)
            }
            Err(e) => {
                log::warn!("WiFi connection failed: {:?}", e);
                log::warn!("Continuing without network connectivity");
                Err(network_manager)
            }
        }
    };
    
    // Extract network manager back
    let network_manager = match wifi_result {
        Ok(mgr) => mgr,
        Err(mgr) => mgr,
    };

    // Initialize OTA manager - always create wrapper even if manager fails
    log::info!("Initializing OTA manager...");
    let ota_manager = match ota::OtaManager::new() {
        Ok(manager) => {
            log::info!("OTA manager created successfully");
            Some(Arc::new(Mutex::new(manager)))
        }
        Err(e) => {
            log::warn!("OTA manager creation failed: {:?}", e);
            log::warn!("OTA will be available once device is on OTA partition.");
            // Still create the wrapper so endpoints can be registered
            // The actual OTA operation will fail gracefully
            None
        }
    };

    // Start web server with OTA support if we have network
    let web_server = if network_manager.is_connected() {
        match network::web_server::WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {
            Ok(server) => {
                log::info!("Web configuration server started on port 80 with OTA support");
                Some(server)
            }
            Err(e) => {
                log::error!("Failed to start web server: {:?}", e);
                None
            }
        }
    } else {
        log::info!("Skipping web server - no network connection");
        None
    };

    // Complete boot sequence
    boot_manager.set_stage(BootStage::Complete);
    for i in 0..10 {
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
        
        // Reset watchdog
        if i % 3 == 0 {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
        
        Ets::delay_ms(50);
    }
    
    // Smooth transition to main UI
    info!("Boot sequence complete, transitioning to main UI...");
    
    // Fade out boot screen (simple implementation)
    for i in 0..5 {
        let fade_level = 255 - (i * 50);
        // We'll just clear to progressively darker grays
        let gray = colors::rgb565(fade_level as u8 / 4, fade_level as u8 / 4, fade_level as u8 / 4);
        display_manager.clear(gray)?;
        display_manager.flush()?;
        Ets::delay_ms(50);
    }
    
    // Final clear to black
    display_manager.clear(colors::BLACK)?;
    display_manager.flush()?;
    
    // Start main application loop
    info!("Starting main loop - UI should now be visible");
    
    // Ensure backlight is on before entering main loop
    display_manager.update_auto_dim()?;
    
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
        ota_manager,
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
    ota_manager: Option<Arc<Mutex<OtaManager>>>,
) -> Result<()> {
    use std::time::{Duration, Instant};

    // Note: OTA checker would run in the main loop instead of a separate thread
    // due to thread safety constraints with ESP-IDF HTTP server

    // Main UI loop with performance telemetry
    let target_frame_time = Duration::from_millis(33); // ~30 FPS
    let mut last_sensor_update = Instant::now();
    let sensor_update_interval = Duration::from_secs(10); // Reduced from 5s to 10s
    
    // Performance tracking
    let mut frame_count = 0u32;
    let mut last_fps_report = Instant::now();
    let mut total_frame_time = Duration::ZERO;
    let mut max_frame_time = Duration::ZERO;
    
    // OTA status update timer
    let mut last_ota_check = Instant::now();
    let ota_check_interval = Duration::from_secs(1);
    
    // Watchdog reset tracking
    let mut last_watchdog_reset = Instant::now();
    let watchdog_reset_interval = Duration::from_secs(2); // Reduced from 500ms to 2s
    
    log::info!("Main render loop started - entering infinite loop");

    loop {
        let frame_start = Instant::now();
        
        // Removed debug logging from hot path

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
                network_manager.get_ssid().to_string(),
                network_manager.get_signal_strength(),
                network_manager.get_gateway(),
                network_manager.get_mac()
            );
            
            last_sensor_update = Instant::now();
        }
        
        // Update OTA status periodically (if OTA is available)
        if last_ota_check.elapsed() >= ota_check_interval {
            if let Some(ref ota_mgr) = ota_manager {
                let ota_status = ota_mgr.lock().unwrap().get_status();
                ui_manager.update_ota_status(ota_status);
            }
            last_ota_check = Instant::now();
        }

        // Reset watchdog periodically
        if last_watchdog_reset.elapsed() >= watchdog_reset_interval {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
            last_watchdog_reset = Instant::now();
            // Removed debug logging
        }
        
        // Update and render UI
        ui_manager.update()?;
        ui_manager.render(&mut display_manager)?;
        
        // Removed redundant watchdog reset
        
        // Update auto-dim
        display_manager.update_auto_dim()?;
        
        display_manager.flush()?;

        // Frame timing and telemetry
        let frame_time = frame_start.elapsed();
        frame_count += 1;
        total_frame_time += frame_time;
        max_frame_time = max_frame_time.max(frame_time);
        
        // Report FPS every second
        if last_fps_report.elapsed() >= Duration::from_secs(1) {
            let avg_frame_time = total_frame_time / frame_count;
            let fps = (frame_count as f32) / last_fps_report.elapsed().as_secs_f32();
            
            // Get CPU frequency
            let cpu_freq = unsafe { 
                esp_idf_sys::esp_clk_cpu_freq() / 1_000_000
            };
            
            // Get PSRAM info if available
            let psram_free = unsafe {
                esp_idf_sys::esp_get_free_internal_heap_size() / 1024
            };
            
            log::info!("[PERF] FPS: {:.1} | Avg: {:?} | Max: {:?} | CPU: {}MHz | Heap: {}KB | IRAM: {}KB",
                fps,
                avg_frame_time,
                max_frame_time,
                cpu_freq,
                unsafe { esp_idf_sys::esp_get_free_heap_size() } / 1024,
                psram_free
            );
            
            // Reset counters
            frame_count = 0;
            total_frame_time = Duration::ZERO;
            max_frame_time = Duration::ZERO;
            last_fps_report = Instant::now();
        }
        
        // Frame rate limiting - cap at 60 FPS to prevent burning cycles
        let target_60fps = Duration::from_micros(16667); // ~60 FPS
        if frame_time < target_60fps {
            // Use busy wait for more precise timing
            let wait_time = target_60fps - frame_time;
            if wait_time > Duration::from_millis(1) {
                esp_idf_hal::delay::FreeRtos::delay_ms((wait_time.as_millis() - 1) as u32);
            }
            // Busy wait for the last millisecond for precision
            while frame_start.elapsed() < target_60fps {}
        }
    }
}