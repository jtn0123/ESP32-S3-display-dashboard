use anyhow::Result;
use esp_idf_hal::prelude::*;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
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
mod network;
mod ota;
mod sensors;
mod system;
mod ui;
mod version;
mod dual_core;
mod psram;
mod performance;
mod core1_tasks;
mod logging;
mod metrics;
mod metrics_formatter;
mod metrics_rwlock;
// mod ring_buffer;  // TODO: Integrate ring buffer optimization
mod templates;
mod power;

use crate::boot::{BootManager, BootStage};
use crate::display::{DisplayManager, colors};
use crate::network::{NetworkManager, telnet_server::TelnetLogServer};
use crate::ota::OtaManager;
use crate::ui::UiManager;
use crate::dual_core::{DualCoreProcessor, CpuMonitor};
use crate::performance::PerformanceMetrics;
use crate::power::{PowerManager, PowerConfig};

// Global error storage for web server initialization
static mut WEB_SERVER_ERROR: Option<String> = None;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    
    // Initialize our logger with colors and timestamps
    logging::init_logger().expect("Failed to initialize logger");

    // Test enhanced logging with different levels
    info!("ESP32-S3 Dashboard {} - OTA on Port 80", crate::version::full_version());
    log::debug!("Debug logging is enabled with enhanced formatting");
    log::trace!("Trace logging provides the most detailed information");
    info!("Free heap: {} bytes", unsafe {
        esp_idf_sys::esp_get_free_heap_size()
    });
    
    // Initialize and log PSRAM info
    let psram_info = crate::psram::PsramAllocator::get_info();
    psram_info.log_info();
    
    // Check reset reason and log it
    let reset_reason_str = crate::system::reset::get_reset_reason();
    log::info!("Boot reason: {}", reset_reason_str);
    
    let reset_reason = unsafe { esp_idf_sys::esp_reset_reason() };
    let is_ota_restart = match reset_reason {
        esp_idf_sys::esp_reset_reason_t_ESP_RST_SW => {
            log::info!("Software reset detected - checking if from OTA");
            true
        }
        esp_idf_sys::esp_reset_reason_t_ESP_RST_POWERON => {
            log::info!("Power-on reset detected - could be RTC reset or actual power cycle");
            // RTC reset often shows as POWERON, so check if we have WiFi config
            // If we have config, it's likely an RTC reset from OTA
            true  // Treat as potential OTA restart
        }
        esp_idf_sys::esp_reset_reason_t_ESP_RST_PANIC => {
            log::warn!("Panic reset detected");
            false
        }
        _ => {
            log::info!("Other reset reason: {:?}", reset_reason);
            false
        }
    };
    
    // Add delay after OTA restart to let system stabilize
    if is_ota_restart {
        log::info!("Waiting for system to stabilize after OTA...");
        esp_idf_hal::delay::FreeRtos::delay_ms(3000);
        
        // Handle post-OTA WiFi reconnection
        log::info!("Initiating post-OTA WiFi reconnection sequence...");
        if let Err(e) = crate::network::wifi_reconnect::handle_post_ota_wifi() {
            log::warn!("Post-OTA WiFi handling failed: {:?}", e);
            // Continue anyway, as the normal WiFi connection logic will retry
        }
        
        // Additional delay to ensure WiFi is ready
        log::info!("Waiting for WiFi to settle...");
        esp_idf_hal::delay::FreeRtos::delay_ms(2000);
    }
    
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
    
    // Log WiFi credentials (safely)
    {
        let cfg = config.lock().map_err(|e| anyhow::anyhow!("Failed to lock config: {}", e))?;
        log::info!("WiFi credentials: SSID='{}', Password={}", 
            cfg.wifi_ssid,
            if cfg.wifi_password.is_empty() { "<empty>" } else { "<set>" }
        );
    }

    // Debug flag - set to true to run display tests
    // Change this to true and recompile to run baseline performance test
    const RUN_DISPLAY_DEBUG_TEST: bool = false;
    
    if RUN_DISPLAY_DEBUG_TEST {
        log::warn!("Running display debug tests - normal boot disabled");
        log::warn!("Set RUN_DISPLAY_DEBUG_TEST to false for normal operation");
        
        // CRITICAL: Wait for power to stabilize before initializing display
        use esp_idf_hal::delay::Ets;
        Ets::delay_ms(500);  // Longer delay for power stability
        
        // LCD_CAM tests disabled - hardware acceleration not currently used
        log::warn!("LCD_CAM tests have been disabled");
        log::warn!("Set RUN_DISPLAY_DEBUG_TEST to false for normal operation");
        
        log::warn!("Test complete!");
        
        // Keep running
        loop {
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        }
    }
    
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
    
    // Initialize metrics system AFTER display is working
    crate::metrics::init_metrics();
    info!("Metrics system initialized");

    // ESP_LCD: Fast initialization path
    #[cfg(feature = "esp_lcd_driver")]
    {
        info!("ESP_LCD: Fast initialization path - skipping all boot animations");
        
        // Simple startup screen
        display_manager.clear(colors::BLACK)?;
        display_manager.draw_text_centered(80, "ESP_LCD DMA", colors::GREEN, None, 2)?;
        display_manager.draw_text_centered(100, "Starting...", colors::WHITE, None, 1)?;
        display_manager.flush()?;
        
        // Quick init of all components
        let ui_manager = UiManager::new(&mut display_manager)?;
        info!("UI manager created");
        
        let battery_pin = peripherals.pins.gpio4;
        let adc1 = peripherals.adc1;
        let sensor_manager = sensors::SensorManager::new(adc1, battery_pin)?;
        info!("Sensors initialized");
        
        let button1 = peripherals.pins.gpio0;
        let button2 = peripherals.pins.gpio14;
        let button_manager = system::ButtonManager::new(button1, button2)?;
        info!("Buttons initialized");
        
        // Mount SPIFFS filesystem
        {
            use esp_idf_svc::fs::{self, StorageImpl};
            let spiffs_config = fs::SpiffsConfig {
                base_path: "/spiffs".to_owned(),
                partition_label: Some("storage".to_owned()),
                max_files: 5,
                format_if_mount_failed: true,
            };
            let _spiffs = fs::SpiffsSingleton::mount(spiffs_config)?;
            info!("SPIFFS filesystem mounted at /spiffs");
        }
        
        let network_config = config.lock().map_err(|e| anyhow::anyhow!("Failed to lock config: {}", e))?;
        let mut network_manager = NetworkManager::new(
            peripherals.modem,
            sys_loop,
            timer_service,
            network_config.wifi_ssid.clone(),
            network_config.wifi_password.clone(),
            config.clone(),
        )?;
        drop(network_config);
        info!("Network initialized");
        
        // Quick connect attempt
        let wifi_result = match network_manager.connect() {
            Ok(_) => {
                info!("WiFi connected");
                Ok(network_manager)
            }
            Err(e) => {
                log::warn!("WiFi failed: {:?}", e);
                Err(network_manager)
            }
        };
        
        let (mut network_manager, wifi_connected) = match wifi_result {
            Ok(mgr) => (mgr, true),
            Err(mgr) => (mgr, false),
        };
        
        // Wait for IP assignment if WiFi connected
        if wifi_connected {
            log::info!("Waiting for IP address assignment...");
            let mut ip_wait = 0;
            while !network_manager.is_connected() && ip_wait < 100 {
                esp_idf_hal::delay::FreeRtos::delay_ms(100);
                ip_wait += 1;
            }
            
            if network_manager.is_connected() {
                log::info!("IP address obtained: {:?}", network_manager.get_ip());
            } else {
                log::warn!("Failed to obtain IP address after 10 seconds");
            }
        }
        
        // Initialize OTA
        let ota_manager = match ota::OtaManager::new() {
            Ok(manager) => Some(Arc::new(Mutex::new(manager))),
            Err(e) => {
                log::warn!("OTA init failed: {:?}", e);
                None
            }
        };
        
        // Start telnet server first so we can capture web server errors
        let telnet_server = if network_manager.is_connected() {
            let server = Arc::new(TelnetLogServer::new(23));
            logging::set_telnet_server(Arc::clone(&server));
            match Arc::clone(&server).start() {
                Ok(_) => {
                    log::info!("Telnet server started successfully on port 23");
                    log::info!("ESP32-S3 Dashboard {} initialized", crate::version::DISPLAY_VERSION);
                    log::info!("Web interface available at http://{}/", network_manager.get_ip().unwrap_or_default());
                    
                    // Check for stored web server error
                    unsafe {
                        if let Some(ref error) = WEB_SERVER_ERROR {
                            log::error!("STORED WEB SERVER ERROR: {}", error);
                            log::error!("The web server failed to start earlier!");
                            log::error!("This prevents OTA updates from working!");
                        }
                    }
                    
                    Some(server)
                }
                Err(e) => {
                    log::error!("Failed to start telnet server: {:?}", e);
                    None
                }
            }
        } else {
            log::info!("Skipping telnet server - no network connection");
            None
        };
        
        // Small delay to ensure telnet is ready
        esp_idf_hal::delay::FreeRtos::delay_ms(100);
        
        // Log before attempting web server start
        log::info!("Attempting to start web server...");
        log::info!("Network connected: {}", network_manager.is_connected());
        log::info!("Device IP: {:?}", network_manager.get_ip());
        
        // Start web server after telnet so errors are logged
        let web_server = if network_manager.is_connected() {
            log::info!("Network is connected, starting web server...");
            match network::web_server::WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {
                Ok(server) => {
                    log::info!("Web server started successfully on port 80");
                    Some(server)
                }
                Err(e) => {
                    let error_msg = format!("Web server failed: {}", e);
                    log::error!("Failed to start web server: {:?}", e);
                    log::error!("Web server error details: {}", e);
                    log::error!("This error prevents OTA updates from working");
                    
                    // Store error globally
                    unsafe {
                        WEB_SERVER_ERROR = Some(error_msg.clone());
                    }
                    
                    // Log multiple times to ensure it's captured
                    for _ in 0..3 {
                        esp_idf_hal::delay::FreeRtos::delay_ms(100);
                        log::error!("WEB SERVER FAILED TO START: {}", e);
                    }
                    None
                }
            }
        } else {
            log::info!("Skipping web server - no network connection");
            None
        };
        
        // Start Core 1 tasks
        let (mut core1_manager, core1_channels) = core1_tasks::Core1Manager::new()?;
        core1_manager.start()?;
        info!("Core 1 tasks started");
        
        // Clear and go to main loop
        display_manager.clear(colors::BLACK)?;
        display_manager.flush()?;
        
        info!("ESP_LCD: Fast init complete, entering main loop");
        
        // Print initial memory stats
        #[cfg(feature = "esp_lcd_driver")]
        {
            use crate::display::diagnostics;
            diagnostics::print_memory_stats("ESP_LCD Init Complete");
            diagnostics::print_stack_watermark("ESP_LCD Init");
            diagnostics::check_dma_capable_memory();
        }
        
        // Jump directly to main loop
        return run_app(
            ui_manager,
            display_manager,
            sensor_manager,
            button_manager,
            network_manager,
            config,
            web_server,
            ota_manager,
            telnet_server,
            core1_channels,
        );
    }

    // Initialize boot manager for animated boot experience
    #[cfg(not(feature = "esp_lcd_driver"))]
    {
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
    }
    
    // Create boot manager for both paths
    let mut boot_manager = BootManager::new();
    
    // Memory initialization with progress
    #[cfg(not(feature = "esp_lcd_driver"))]
    {
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
    }
    
    // Initialize UI
    info!("Creating UI manager...");
    #[cfg(not(feature = "esp_lcd_driver"))]
    {
        boot_manager.set_stage(BootStage::UISetup);
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
    }
    
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
    
    let network_config = config.lock().map_err(|e| anyhow::anyhow!("Failed to lock config: {}", e))?;
    let mut network_manager = NetworkManager::new(
        peripherals.modem,
        sys_loop,
        timer_service,
        network_config.wifi_ssid.clone(),
        network_config.wifi_password.clone(),
        config.clone(),
    )?;
    drop(network_config);
    
    // Connect to WiFi
    log::info!("Connecting to WiFi...");
    match network_manager.connect() {
        Ok(_) => {
            log::info!("WiFi connected successfully!");
            
            // Wait for IP assignment (up to 10 seconds)
            log::info!("Waiting for IP address...");
            let mut ip_wait = 0;
            while !network_manager.is_connected() && ip_wait < 100 {
                esp_idf_hal::delay::FreeRtos::delay_ms(100);
                ip_wait += 1;
            }
            
            if network_manager.is_connected() {
                log::info!("IP address obtained: {:?}", network_manager.get_ip());
            } else {
                log::warn!("Failed to obtain IP address after 10 seconds");
            }
        }
        Err(e) => {
            log::warn!("WiFi connection failed: {:?}", e);
            log::info!("Continuing without WiFi - auto-reconnect will retry");
        }
    }
    
    // Keep animating during network init
    log::info!("Boot: Keeping display alive during network stage");
    for i in 0..10 {
        boot_manager.render_boot_screen(&mut display_manager)?;
        display_manager.flush()?;
        display_manager.update_auto_dim(true)?; // Keep display alive during boot
        
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
    
    // Extract network manager back and track connection status
    let (network_manager, wifi_connected) = match wifi_result {
        Ok(mgr) => (mgr, true),
        Err(mgr) => (mgr, false),
    };
    
    // Wait for IP assignment if WiFi connected
    if wifi_connected {
        log::info!("Waiting for IP address assignment...");
        let mut ip_wait = 0;
        while !network_manager.is_connected() && ip_wait < 100 {
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            ip_wait += 1;
        }
        
        if network_manager.is_connected() {
            log::info!("IP address obtained: {:?}", network_manager.get_ip());
        } else {
            log::warn!("Failed to obtain IP address after 10 seconds");
        }
    }

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

    // Log before attempting web server start
    log::info!("Normal boot path - attempting to start web server...");
    log::info!("Network connected: {}", network_manager.is_connected());
    log::info!("Device IP: {:?}", network_manager.get_ip());
    
    // Start web server with OTA support if we have network
    let web_server = if network_manager.is_connected() {
        log::info!("Network is connected, starting web server...");
        match network::web_server::WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {
            Ok(server) => {
                log::info!("Web configuration server started on port 80 with OTA support");
                Some(server)
            }
            Err(e) => {
                let error_msg = format!("Web server failed: {}", e);
                log::error!("Failed to start web server: {:?}", e);
                log::error!("Web server error details: {}", e);
                log::error!("This error prevents OTA updates from working");
                
                // Store error globally
                unsafe {
                    WEB_SERVER_ERROR = Some(error_msg.clone());
                }
                
                // Log multiple times to ensure it's captured
                for _ in 0..3 {
                    esp_idf_hal::delay::FreeRtos::delay_ms(100);
                    log::error!("WEB SERVER FAILED TO START: {}", e);
                }
                None
            }
        }
    } else {
        log::info!("Skipping web server - no network connection");
        None
    };
    
    // Start telnet log server if we have network
    let telnet_server = if network_manager.is_connected() {
        let server = Arc::new(TelnetLogServer::new(23));
        
        // Set the telnet server in our custom logger
        logging::set_telnet_server(Arc::clone(&server));
        
        match Arc::clone(&server).start() {
            Ok(_) => {
                log::info!("Telnet log server started on port 23");
                log::info!("Connect with: telnet {} 23", network_manager.get_ip().unwrap_or_default());
                
                // Log some initial messages to populate the buffer
                log::info!("ESP32-S3 Dashboard {} initialized", crate::version::DISPLAY_VERSION);
                log::info!("Web interface available at http://{}/", network_manager.get_ip().unwrap_or_default());
                log::info!("Logs can be viewed at http://{}/logs", network_manager.get_ip().unwrap_or_default());
                
                // Check for stored web server error
                unsafe {
                    if let Some(ref error) = WEB_SERVER_ERROR {
                        log::error!("STORED WEB SERVER ERROR: {}", error);
                        log::error!("The web server failed to start earlier!");
                        log::error!("This prevents OTA updates from working!");
                    }
                }
                
                Some(server)
            }
            Err(e) => {
                log::error!("Failed to start telnet server: {:?}", e);
                None
            }
        }
    } else {
        log::info!("Skipping telnet server - no network connection");
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
    
    // Test frame buffer by drawing test pattern
    info!("Testing frame buffer with color rectangles...");
    
    // Draw test rectangles using frame buffer
    display_manager.fill_rect(10, 10, 50, 50, 0xF800)?; // Red
    display_manager.fill_rect(70, 10, 50, 50, 0x07E0)?; // Green  
    display_manager.fill_rect(130, 10, 50, 50, 0x001F)?; // Blue
    display_manager.fill_rect(190, 10, 50, 50, 0xFFFF)?; // White
    display_manager.flush()?;
    
    info!("Test pattern drawn - checking for corruption...");
    Ets::delay_ms(2000);
    
    // Clear and test with different pattern
    display_manager.clear(colors::BLACK)?;
    display_manager.fill_rect(0, 0, 300, 10, 0xF800)?; // Red stripe at top
    display_manager.fill_rect(0, 158, 300, 10, 0x001F)?; // Blue stripe at bottom
    display_manager.flush()?;
    
    Ets::delay_ms(2000);
    
    // Start main application loop
    info!("Starting main loop - UI should now be visible");
    
    // Ensure backlight is on before entering main loop
    display_manager.update_auto_dim(true)?; // Keep display on during startup
    
    info!("Entering run_app function now...");
    
    // Run the main app with crash recovery
    // Initialize Core 1 tasks
    let (mut core1_manager, core1_channels) = core1_tasks::Core1Manager::new()?;
    core1_manager.start()?;
    info!("Core 1 background tasks started");
    
    match run_app(
        ui_manager,
        display_manager,
        sensor_manager,
        button_manager,
        network_manager,
        config,
        web_server,
        ota_manager,
        telnet_server,
        core1_channels,
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
    _telnet_server: Option<Arc<TelnetLogServer>>,
    core1_channels: core1_tasks::Core1Channels,
) -> Result<()> {
    use std::time::{Duration, Instant};

    // Initialize dual-core processor
    let dual_core = DualCoreProcessor::new();
    let mut cpu_monitor = CpuMonitor::new();
    
    // Initialize power manager with custom config
    let power_config = PowerConfig {
        dim_timeout: Duration::from_secs(60),      // Dim after 1 minute
        power_save_timeout: Duration::from_secs(300), // Power save after 5 minutes
        sleep_timeout: Duration::from_secs(600),   // Sleep after 10 minutes
        active_brightness: 100,
        dimmed_brightness: 30,
        power_save_brightness: 10,
        low_battery_threshold: 20,
    };
    let mut power_manager = PowerManager::new(power_config);
    
    // CRITICAL: Mark activity immediately to prevent instant sleep
    power_manager.activity_detected();
    log::info!("Power manager created and marked as active");
    
    // Initialize uptime tracker
    let uptime_tracker = match system::UptimeTracker::new() {
        Ok(tracker) => {
            log::info!("Uptime tracking initialized - Boot #{}, Total uptime: {}", 
                      tracker.get_boot_count(), 
                      tracker.format_total_uptime());
            Some(Arc::new(Mutex::new(tracker)))
        }
        Err(e) => {
            log::warn!("Failed to initialize uptime tracker: {:?}", e);
            None
        }
    };
    
    log::info!("Dual-core processing initialized");
    log::info!("Main task running on core {}", DualCoreProcessor::current_core());
    log::info!("Power management initialized with dimming support");

    // Note: OTA checker would run in the main loop instead of a separate thread
    // due to thread safety constraints with ESP-IDF HTTP server

    // Main UI loop with performance telemetry
    // Display hardware limitation: ~10 FPS max with parallel GPIO
    const DISPLAY_MAX_FPS: f32 = 10.0;
    let _target_frame_time = Duration::from_millis(100); // ~10 FPS
    let mut last_sensor_update = Instant::now();
    let sensor_update_interval = Duration::from_secs(10); // Reduced from 5s to 10s
    
    // Performance tracking
    let mut perf_metrics = PerformanceMetrics::new();
    let mut last_fps_report = Instant::now();
    
    // OTA status update timer
    let mut last_ota_check = Instant::now();
    let ota_check_interval = Duration::from_secs(1);
    
    // Watchdog reset tracking
    let mut last_watchdog_reset = Instant::now();
    let watchdog_reset_interval = Duration::from_secs(2); // Reduced from 500ms to 2s
    
    // CPU usage tracking
    let mut last_cpu_check = Instant::now();
    let cpu_check_interval = Duration::from_secs(2);
    let mut last_cpu0_usage = 0u8;
    
    // Power manager startup grace period - prevent sleep during initialization
    let startup_time = Instant::now();
    let startup_grace_period = Duration::from_secs(30); // 30 seconds grace period
    let mut last_cpu1_usage = 0u8;
    
    // Button polling optimization - only check every 20ms
    let mut last_button_check = Instant::now();
    let button_check_interval = Duration::from_millis(20);
    
    // Button test metrics
    let button_test_start = Instant::now();
    let mut button_events_count = 0u32;
    let mut max_response_time = Duration::ZERO;
    let mut total_response_time = Duration::ZERO;
    
    // Memory diagnostics tracking (ESP_LCD only)
    #[cfg(feature = "esp_lcd_driver")]
    let mut last_memory_check = Instant::now();
    #[cfg(feature = "esp_lcd_driver")]
    let memory_check_interval = Duration::from_secs(30);
    
    // Logging optimization - track previous values to reduce redundant logs
    let mut last_logged_fps: f32 = 0.0;
    let mut last_logged_cpu0: u8 = 0;
    let mut last_logged_cpu1: u8 = 0;
    const FPS_CHANGE_THRESHOLD: f32 = 5.0;  // Only log if FPS changes by more than 5
    const CPU_CHANGE_THRESHOLD: u8 = 10;    // Only log if CPU usage changes by more than 10%
    
    log::info!("Main render loop started - entering infinite loop");

    // Sensor reading stays on Core 0 but we'll minimize the work
    let mut last_sensor_reading = Instant::now();
    let sensor_reading_interval = Duration::from_secs(5); // Read sensors every 5 seconds
    let sensor_tx = core1_channels.sensor_tx.clone();
    
    loop {
        // Start frame timing
        let frame_start = Instant::now();
        
        // Send sensor data to Core 1 for processing
        if last_sensor_reading.elapsed() >= sensor_reading_interval {
            // Sample sensors quickly on Core 0
            if let Ok(sensor_result) = sensor_manager.sample() {
                let (cpu0_usage, cpu1_usage) = cpu_monitor.get_cpu_usage();
                
                // Send to Core 1 for processing
                let sensor_update = core1_tasks::SensorUpdate {
                    temperature: sensor_result._temperature,
                    battery_percentage: sensor_result._battery_percentage,
                    battery_voltage: sensor_result._battery_voltage,
                    is_charging: sensor_result._is_charging,
                    is_on_usb: sensor_result._is_on_usb,
                    cpu_usage_core0: cpu0_usage,
                    cpu_usage_core1: cpu1_usage,
                };
                
                // Non-blocking send to Core 1
                if let Err(_) = sensor_tx.send(sensor_update.clone()) {
                    log::warn!("Core 1 sensor queue full, skipping update");
                }
            }
            last_sensor_reading = Instant::now();
        }

        // Handle button input with debounce (only check every 20ms)
        if last_button_check.elapsed() >= button_check_interval {
            let poll_start = Instant::now();
            if let Some(event) = button_manager.poll() {
                let response_time = poll_start.elapsed();
                log::info!("[BUTTON_TEST] Button event detected: {:?}, Poll latency: {:.2}ms, Time since last check: {:.2}ms", 
                    event, 
                    response_time.as_secs_f32() * 1000.0,
                    last_button_check.elapsed().as_secs_f32() * 1000.0
                );
                
                let ui_start = Instant::now();
                ui_manager.handle_button_event(event)?;
                let ui_time = ui_start.elapsed();
                
                // Reset activity timer on button press
                display_manager.reset_activity_timer();
                power_manager.activity_detected();
                
                let total_time = response_time + ui_time;
                button_events_count += 1;
                total_response_time += total_time;
                if total_time > max_response_time {
                    max_response_time = total_time;
                }
                
                log::info!("[BUTTON_TEST] UI response time: {:.2}ms, Total response time: {:.2}ms", 
                    ui_time.as_secs_f32() * 1000.0,
                    total_time.as_secs_f32() * 1000.0
                );
                
                // Update metrics for Prometheus/Grafana
                let avg_response = if button_events_count > 0 {
                    total_response_time / button_events_count
                } else {
                    Duration::ZERO
                };
                let test_duration = button_test_start.elapsed();
                let events_per_sec = if test_duration.as_secs_f32() > 0.0 {
                    button_events_count as f32 / test_duration.as_secs_f32()
                } else {
                    0.0
                };
                
                // Update metrics
                {
                    let mut metrics = match crate::metrics::metrics().lock() {
                        Ok(m) => m,
                        Err(e) => {
                            log::error!("Failed to lock metrics: {}", e);
                            continue;
                        }
                    };
                    metrics.update_button_metrics(
                        avg_response.as_secs_f32() * 1000.0,
                        max_response_time.as_secs_f32() * 1000.0,
                        button_events_count as u64,
                        events_per_sec
                    );
                }
                
                // Print summary every 10 button presses
                if button_events_count % 10 == 0 {
                    log::warn!("[BUTTON_TEST_SUMMARY] After {} events in {:.1}s: Avg response: {:.2}ms, Max: {:.2}ms, Events/sec: {:.1}", 
                        button_events_count,
                        test_duration.as_secs_f32(),
                        avg_response.as_secs_f32() * 1000.0,
                        max_response_time.as_secs_f32() * 1000.0,
                        events_per_sec
                    );
                }
            }
            last_button_check = Instant::now();
        }

        // Check for updates from Core 1
        // Process data from Core 1 (non-blocking)
        if let Ok(processed_data) = core1_channels.processed_rx.try_recv() {
            // Core 1 now receives proper sensor data from Core 0, no override needed
            // Rate-limited debug logging to reduce spam
            static mut DEBUG_COUNTER: u32 = 0;
            unsafe {
                DEBUG_COUNTER = DEBUG_COUNTER.wrapping_add(1);
                if DEBUG_COUNTER % 600 == 0 {  // Log once every ~10 seconds at 60 FPS
                    log::debug!("Core 0: Received processed data from Core 1 - Temp: {:.1}°C, Battery: {}%", 
                        processed_data.temperature, processed_data.battery_percentage);
                }
            }
            
            // Update UI with processed sensor data
            ui_manager.update_sensor_data(sensors::SensorData {
                _temperature: processed_data.temperature,
                _battery_percentage: processed_data.battery_percentage,
                _battery_voltage: processed_data.battery_voltage,
                _is_charging: processed_data.is_charging,
                _is_on_usb: processed_data.is_on_usb,
                _light_level: 0,
            });
            
            // Update CPU usage display
            ui_manager.update_cpu_usage(
                processed_data.cpu_usage_core0,
                processed_data.cpu_usage_core1
            );
            
            // Update temperature and battery in metrics
            {
                let mut metrics = match crate::metrics::metrics().lock() {
                    Ok(m) => m,
                    Err(e) => {
                        log::error!("Failed to lock metrics: {}", e);
                        continue;
                    }
                };
                metrics.update_temperature(processed_data.temperature);
                metrics.update_battery(
                    processed_data.battery_voltage,
                    processed_data.battery_percentage,
                    processed_data.is_charging
                );
            }
            
            // Update sensor history
            if let Some(history) = crate::sensors::history::get() {
                if let Ok(hist) = history.lock() {
                    hist.add_temperature(processed_data.temperature);
                    hist.add_battery(processed_data.battery_percentage as f32);
                }
            }
            
            // TEMPORARILY DISABLED: Update power manager with sensor data (skip during startup grace period)
            // if startup_time.elapsed() > startup_grace_period {
            //     power_manager.update(&sensors::SensorData {
            //         _temperature: processed_data.temperature,
            //         _battery_percentage: processed_data.battery_percentage,
            //         _battery_voltage: processed_data.battery_voltage,
            //         _is_charging: processed_data.is_charging,
            //         _is_on_usb: processed_data.is_on_usb,
            //         _light_level: 0,
            //     });
            // } else {
            //     log::info!("Skipping power manager update during startup grace period ({:.1}s remaining)", 
            //               (startup_grace_period - startup_time.elapsed()).as_secs_f32());
            // }
            
            last_sensor_update = Instant::now();
        }
        
        // Update network status periodically
        if last_sensor_update.elapsed() >= sensor_update_interval {
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
                let ota_status = match ota_mgr.lock() {
                    Ok(mgr) => mgr.get_status(),
                    Err(e) => {
                        log::error!("Failed to lock OTA manager: {}", e);
                        continue;
                    }
                };
                ui_manager.update_ota_status(ota_status);
            }
            last_ota_check = Instant::now();
        }

        // Reset watchdog periodically
        if last_watchdog_reset.elapsed() >= watchdog_reset_interval {
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
            last_watchdog_reset = Instant::now();
            
            // Save uptime tracker state periodically (piggyback on watchdog timer)
            if let Some(ref tracker) = uptime_tracker {
                if let Ok(mut t) = tracker.lock() {
                    let _ = t.save_if_needed();
                }
            }
        }
        
        // Update and render UI
        ui_manager.update()?;
        
        let render_start = Instant::now();
        let rendered = ui_manager.render(&mut display_manager)?;
        let render_time = render_start.elapsed();
        
        // Track whether frame was actually rendered or skipped
        if rendered {
            perf_metrics.record_render_time(render_time);
            
            // TEMPORARILY HARDCODED: Always keep display on
            display_manager.update_auto_dim(true)?;
            
            // Flush to display
            let flush_start = Instant::now();
            display_manager.flush()?;
            let flush_time = flush_start.elapsed();
            perf_metrics.record_flush_time(flush_time);
        } else {
            // Frame was skipped by UI manager
            perf_metrics.fps_tracker.frame_skipped();
            
            // TEMPORARILY HARDCODED: Always keep display on
            display_manager.update_auto_dim(true)?;
        }
        
        // Track ALL loop iterations for accurate main loop FPS
        // Record every frame (both rendered and skipped) to show effective FPS
        let loop_time = frame_start.elapsed();
        perf_metrics.fps_tracker.frame_rendered(loop_time);

        // Frame timing and telemetry
        let frame_time = frame_start.elapsed();
        
        // Update memory stats periodically
        perf_metrics.update_memory_stats();
        
        // Report FPS every second
        if last_fps_report.elapsed() >= Duration::from_secs(1) {
            let fps_stats = perf_metrics.fps_tracker.stats();
            
            // Get CPU frequency
            let cpu_freq = unsafe { 
                esp_idf_sys::ets_get_cpu_frequency()
            };
            
            // Get PSRAM info if available
            let _psram_free = if crate::psram::PsramAllocator::is_available() {
                crate::psram::PsramAllocator::get_free_size() / 1024
            } else {
                0
            };
            
            // Get CPU usage for both cores
            let (cpu0_usage, cpu1_usage) = if last_cpu_check.elapsed() >= cpu_check_interval {
                last_cpu_check = Instant::now();
                let (new_cpu0, new_cpu1) = cpu_monitor.get_cpu_usage();
                last_cpu0_usage = new_cpu0;
                last_cpu1_usage = new_cpu1;
                (new_cpu0, new_cpu1)
            } else {
                (last_cpu0_usage, last_cpu1_usage)
            };
            
            // Get dual-core stats
            let core_stats = dual_core.get_stats();
            
            // Update UI with core stats
            ui_manager.update_core_stats(cpu0_usage, cpu1_usage, core_stats.core0_tasks, core_stats.core1_tasks);
            
            // Calculate if we're meeting the target
            let fps_status = if fps_stats.current_fps >= DISPLAY_MAX_FPS * 0.9 {
                "MAX"  // At hardware limit
            } else if fps_stats.current_fps >= DISPLAY_MAX_FPS * 0.5 {
                "OK"   // Acceptable performance
            } else {
                "LOW"  // Below target
            };
            
            // Only log if FPS or CPU usage changed significantly
            let fps_changed = (fps_stats.current_fps - last_logged_fps).abs() > FPS_CHANGE_THRESHOLD;
            let cpu0_changed = (cpu0_usage as i16 - last_logged_cpu0 as i16).abs() > CPU_CHANGE_THRESHOLD as i16;
            let cpu1_changed = (cpu1_usage as i16 - last_logged_cpu1 as i16).abs() > CPU_CHANGE_THRESHOLD as i16;
            
            if fps_changed || cpu0_changed || cpu1_changed {
                let perf_msg = format!("[PERF] FPS: {:.1}/{:.0} [{}] | Skip: {:.1}% | Render: {:.1}ms | Flush: {:.1}ms | CPU: {}MHz | Heap: {}KB",
                    fps_stats.current_fps,
                    DISPLAY_MAX_FPS,
                    fps_status,
                    fps_stats.skip_rate,
                    perf_metrics.last_render_time.as_secs_f32() * 1000.0,
                    perf_metrics.last_flush_time.as_secs_f32() * 1000.0,
                    cpu_freq,
                    perf_metrics.heap_free / 1024
                );
                log::info!("{}", perf_msg);
                
                // Format CPU usage - show "N/A" if 0 (not available)
                let cpu0_str = if cpu0_usage == 0 { "N/A".to_string() } else { format!("{}%", cpu0_usage) };
                let cpu1_str = if cpu1_usage == 0 { "N/A".to_string() } else { format!("{}%", cpu1_usage) };
                
                let cores_msg = format!("[CORES] CPU0: {} | CPU1: {} | Tasks: C0={} C1={} Total={} | Avg: {}μs",
                    cpu0_str,
                    cpu1_str,
                    core_stats.core0_tasks,
                    core_stats.core1_tasks,
                    core_stats.total_tasks,
                    core_stats.avg_task_time_us
                );
                log::info!("{}", cores_msg);
                
                // Update last logged values
                last_logged_fps = fps_stats.current_fps;
                last_logged_cpu0 = cpu0_usage;
                last_logged_cpu1 = cpu1_usage;
            }
            
            // Update UI manager with accurate FPS
            ui_manager.update_fps(fps_stats.current_fps);
            
            // Debug log if FPS is very low or zero
            if fps_stats.current_fps < 1.0 {
                log::debug!("[FPS] Low/zero FPS detected: {:.1}, frames: {}, skipped: {}", 
                    fps_stats.current_fps, fps_stats.total_frames, fps_stats.skipped_frames);
            }
            
            // Update global metrics
            {
                let mut metrics = match crate::metrics::metrics().lock() {
                    Ok(m) => m,
                    Err(e) => {
                        log::error!("Failed to lock metrics: {}", e);
                        continue;
                    }
                };
                
                // FPS and performance metrics
                metrics.update_fps(fps_stats.current_fps, DISPLAY_MAX_FPS); // realistic target based on hardware
                
                // Frame skip metrics
                metrics.update_frame_stats(fps_stats.total_frames, fps_stats.skipped_frames);
                
                // Debug log what we're storing
                if fps_stats.current_fps > 100.0 {
                    log::warn!("[METRICS] Unrealistic FPS being stored: {:.1} (frames: {}, skipped: {})", 
                        fps_stats.current_fps, fps_stats.total_frames, fps_stats.skipped_frames);
                }
                
                // CPU metrics - both individual cores and average
                metrics.update_cpu_cores(cpu0_usage, cpu1_usage);
                metrics.update_cpu((cpu0_usage + cpu1_usage) / 2, cpu_freq as u16);
                
                // Timing metrics
                let render_ms = (perf_metrics.last_render_time.as_secs_f32() * 1000.0) as u32;
                let flush_ms = (perf_metrics.last_flush_time.as_secs_f32() * 1000.0) as u32;
                metrics.update_timings(render_ms, flush_ms);
                
                // WiFi signal strength and connection status
                let rssi = network_manager.get_signal_strength();
                metrics.update_wifi_signal(rssi);
                metrics.update_wifi_status(
                    network_manager.is_connected(),
                    network_manager.get_ssid().to_string()
                );
                
                // Display brightness (static for now, but available for future use)
                metrics.update_display(255); // Max brightness
                
                // PSRAM metrics
                if crate::psram::PsramAllocator::is_available() {
                    let psram_free = crate::psram::PsramAllocator::get_free_size() as u32;
                    let psram_total = crate::psram::PsramAllocator::get_size() as u32;
                    metrics.update_psram(psram_free, psram_total);
                }
                
                // Note: Temperature and battery are updated from Core 1 data elsewhere
            }
            
            // Reset report timer
            last_fps_report = Instant::now();
        }
        
        // Periodic memory diagnostics (ESP_LCD only)
        #[cfg(feature = "esp_lcd_driver")]
        if last_memory_check.elapsed() >= memory_check_interval {
            use crate::display::diagnostics;
            diagnostics::print_memory_stats("Periodic Check");
            diagnostics::print_stack_watermark("Main Loop");
            last_memory_check = Instant::now();
        }
        
        // Frame rate limiting - toggleable for performance testing
        const ENABLE_FPS_CAP: bool = true; // Set to true for production, false for benchmarking
        
        if ENABLE_FPS_CAP {
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
}