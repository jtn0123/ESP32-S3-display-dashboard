use anyhow::Result;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::ota::OtaManager;
use crate::metrics_formatter::MetricsFormatter;
use crate::network::compression::write_compressed_response;
use crate::network::binary_protocol::MetricsBinaryPacket;

pub struct WebConfigServer {
    _server: EspHttpServer<'static>,
}

#[derive(serde::Deserialize)]
struct WebConfig {
    wifi_ssid: String,
    wifi_password: String,
    brightness: u8,
    auto_dim: bool,
    update_interval: u32,
    auto_update: bool,
}

impl WebConfigServer {
    pub fn new_with_ota(config: Arc<Mutex<Config>>, ota_manager: Option<Arc<Mutex<OtaManager>>>) -> Result<Self> {
        Self::new_with_ota_and_metrics(config, ota_manager, crate::metrics::metrics().clone())
    }
    
    pub fn new_with_ota_and_metrics(
        config: Arc<Mutex<Config>>, 
        ota_manager: Option<Arc<Mutex<OtaManager>>>,
        metrics: Arc<crate::metrics::MetricsWrapper>
    ) -> Result<Self> {
        Self::new_with_ota_metrics_and_sensor_history(config, ota_manager, metrics, None)
    }
    
    pub fn new_with_ota_metrics_and_sensor_history(
        config: Arc<Mutex<Config>>, 
        ota_manager: Option<Arc<Mutex<OtaManager>>>,
        metrics: Arc<crate::metrics::MetricsWrapper>,
        sensor_history: Option<Arc<Mutex<crate::sensors::history::SensorHistory>>>
    ) -> Result<Self> {
        let mut server = EspHttpServer::new(&Configuration::default())?;
        
        let config_clone = config.clone();
        
        // Home page with dynamic content
        server.fn_handler("/", esp_idf_svc::http::Method::Get, move |req| {
            // Get system info for template
            let version = crate::version::DISPLAY_VERSION;
            let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
            let uptime_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
            
            // Get WiFi SSID from config
            let ssid = match config_clone.lock() {
                Ok(cfg) => cfg.wifi_ssid.clone(),
                Err(_) => "Not connected".to_string(),
            };
            
            // Render template with dynamic content
            let html = crate::templates::render_home_page(version, &ssid, free_heap, uptime_ms);
            
            write_compressed_response(req, html.as_bytes(), "text/html; charset=utf-8")
        })?;

        // Get current configuration
        let config_clone2 = config.clone();
        server.fn_handler("/api/config", esp_idf_svc::http::Method::Get, move |req| {
            let config = match config_clone2.lock() {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::error!("Failed to lock config: {}", e);
                    let mut response = req.into_status_response(503)?;
                    response.write_all(b"Configuration lock failed")?;
                    return Ok(());
                }
            };
            let json = serde_json::to_string(&*config)?;
            
            let mut response = req.into_ok_response()?;
            response.write_all(json.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Update configuration
        let config_clone3 = config.clone();
        server.fn_handler("/api/config", esp_idf_svc::http::Method::Post, move |mut req| {
            let mut buf = vec![0; 1024];
            let len = req.read(&mut buf)?;
            buf.truncate(len);
            
            let json_str = std::str::from_utf8(&buf)?;
            
            // Parse the web config format
            let web_config: WebConfig = serde_json::from_str(json_str)?;
            
            // Convert to internal config format
            let new_config = Config {
                wifi_ssid: web_config.wifi_ssid,
                wifi_password: web_config.wifi_password,
                brightness: web_config.brightness,
                auto_brightness: web_config.auto_dim,
                dim_timeout_secs: 30, // Default
                sleep_timeout_secs: 300, // Default
                theme: crate::config::Theme::Dark, // Default
                show_animations: true, // Default
                ota_enabled: web_config.auto_update,
                ota_check_interval_hours: web_config.update_interval as u32 * 3600 / 3600, // Convert seconds to hours
            };
            
            // Update and save config
            {
                let mut config = match config_clone3.lock() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        log::error!("Failed to lock config: {}", e);
                        let mut response = req.into_status_response(503)?;
                    response.write_all(b"Configuration lock failed")?;
                    return Ok(());
                    }
                };
                *config = new_config;
                config.save()?;
            }
            
            let _response = req.into_ok_response()?;
            Ok(()) as Result<(), Box<dyn std::error::Error>> as Result<(), Box<dyn std::error::Error>>
        })?;

        // System info endpoint
        let config_clone_system = config.clone();
        server.fn_handler("/api/system", esp_idf_svc::http::Method::Get, move |req| {
            // Get SSID from config
            let ssid = match config_clone_system.lock() {
                Ok(cfg) => cfg.wifi_ssid.clone(),
                Err(_) => "Unknown".to_string(),
            };
            
            let info = SystemInfo {
                version: env!("CARGO_PKG_VERSION").to_string(),
                ssid,
                free_heap: unsafe { esp_idf_sys::esp_get_free_heap_size() },
                uptime_ms: unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 },
            };
            
            let json = serde_json::to_string(&info)?;
            let mut response = req.into_ok_response()?;
            response.write_all(json.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Prometheus metrics endpoint - optimized with formatter
        server.fn_handler("/metrics", esp_idf_svc::http::Method::Get, move |req| {
            // Get system metrics
            let uptime_seconds = unsafe { esp_idf_sys::esp_timer_get_time() / 1_000_000 } as u64;
            let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
            let heap_total = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
            
            // Get device info for labels
            let version = crate::version::DISPLAY_VERSION;
            let board_type = "ESP32-S3";
            let chip_model = "T-Display-S3";
            
            // Try to get metrics data with timeout
            let metrics_result = crate::metrics::metrics().try_lock();
            
            let formatted_metrics = match metrics_result {
                Ok(metrics_guard) => {
                    // Create formatter and format metrics
                    let mut formatter = MetricsFormatter::new();
                    formatter.format_metrics(
                        &*metrics_guard,
                        version,
                        board_type,
                        chip_model,
                        uptime_seconds,
                        heap_free,
                        heap_total,
                    )
                },
                Err(_) => {
                    // If we can't get metrics, return partial data
                    log::warn!("Metrics lock contended, returning partial data");
                    Ok(format!(
                        "# HELP esp32_device_info Device information\n\
                        # TYPE esp32_device_info gauge\n\
                        esp32_device_info{{version=\"{}\",board=\"{}\",model=\"{}\"}} 1\n\n\
                        # HELP esp32_uptime_seconds Total uptime in seconds\n\
                        # TYPE esp32_uptime_seconds counter\n\
                        esp32_uptime_seconds {}\n\n\
                        # HELP esp32_heap_free_bytes Current free heap memory in bytes\n\
                        # TYPE esp32_heap_free_bytes gauge\n\
                        esp32_heap_free_bytes {}\n\n\
                        # HELP esp32_metrics_unavailable Metrics temporarily unavailable\n\
                        # TYPE esp32_metrics_unavailable gauge\n\
                        esp32_metrics_unavailable 1\n",
                        version, board_type, chip_model, uptime_seconds, heap_free
                    ))
                }
            };
            
            match formatted_metrics {
                Ok(metrics) => {
                    let mut response = req.into_response(
                        200,
                        Some("OK"),
                        &[("Content-Type", "text/plain; version=0.0.4")]
                    )?;
                    response.write_all(metrics.as_bytes())?;
                },
                Err(e) => {
                    log::error!("Failed to format metrics: {}", e);
                    let mut response = req.into_status_response(500)?;
                    response.write_all(b"Internal Server Error")?;
                }
            }
            
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Always add OTA endpoints (they'll show error if OTA not available)
        {
            log::info!("Adding OTA endpoints to web server...");
            let ota_mgr_clone = ota_manager.clone();
            
            // OTA web interface
            server.fn_handler("/ota", esp_idf_svc::http::Method::Get, move |req| {
                let html = if ota_mgr_clone.is_some() {
                    crate::templates::OTA_PAGE
                } else {
                    // Show message that OTA will be available after first USB update
                    crate::templates::OTA_UNAVAILABLE_PAGE
                };
                
                write_compressed_response(req, html.as_bytes(), "text/html; charset=utf-8")
                    .map_err(|e| anyhow::anyhow!("Response error: {}", e))
            })?;
            
            // OTA update endpoint
            let ota_manager_clone2 = ota_manager.clone();
            server.fn_handler("/ota/update", esp_idf_svc::http::Method::Post, move |mut req| {
                // Check if OTA is available
                let Some(ota_mgr) = ota_manager_clone2.as_ref() else {
                    let mut response = req.into_status_response(503)?;
                    response.write_all(b"OTA not available - device running from factory partition")?;
                    return Ok::<(), anyhow::Error>(());
                };
                
                // Get content length first
                let content_length = req
                    .header("Content-Length")
                    .and_then(|v| v.parse::<usize>().ok())
                    .ok_or_else(|| anyhow::anyhow!("Missing Content-Length"))?;
                
                log::info!("OTA Update started, size: {} bytes", content_length);
                
                // Perform the OTA update
                let result = {
                    let mut ota = match ota_mgr.lock() {
                        Ok(mgr) => mgr,
                        Err(e) => {
                            log::error!("Failed to lock OTA manager: {}", e);
                            let mut response = req.into_status_response(503)?;
                            response.write_all(b"Internal server error")?;
                            return Ok::<(), anyhow::Error>(());
                        }
                    };
                    
                    // Begin OTA update
                    if let Err(e) = ota.begin_update(content_length) {
                        log::error!("OTA begin_update failed: {:?}", e);
                        Err(anyhow::anyhow!("Failed to begin OTA: {:?}", e))
                    } else {
                        // Read and write firmware in chunks
                        let mut buffer = vec![0u8; 4096];
                        let mut total_read = 0;
                        let mut write_error = None;
                        
                        loop {
                            match req.read(&mut buffer) {
                                Ok(0) => break, // EOF
                                Ok(bytes_read) => {
                                    if let Err(e) = ota.write_chunk(&buffer[..bytes_read]) {
                                        log::error!("OTA write_chunk failed after {} bytes: {:?}", total_read, e);
                                        write_error = Some(anyhow::anyhow!("Failed to write OTA data: {:?}", e));
                                        break;
                                    }
                                    total_read += bytes_read;
                                    
                                    // Log progress
                                    let progress = ota.get_progress();
                                    if progress % 10 == 0 && progress > 0 {
                                        log::info!("OTA Progress: {}%", progress);
                                    }
                                }
                                Err(e) => {
                                    write_error = Some(anyhow::anyhow!("Failed to read request data: {:?}", e));
                                    break;
                                }
                            }
                        }
                        
                        if let Some(e) = write_error {
                            Err(e)
                        } else {
                            // Finish update
                            if let Err(e) = ota.finish_update() {
                                log::error!("OTA finish_update failed: {:?}", e);
                                Err(anyhow::anyhow!("Failed to finish OTA: {:?}", e))
                            } else {
                                log::info!("OTA Update complete, restarting...");
                                Ok(())
                            }
                        }
                    }
                };
                
                // Handle the result and send response
                match result {
                    Ok(_) => {
                        let mut response = req.into_ok_response()?;
                        response.write_all(b"Update successful")?;
                        
                        // Schedule restart
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            log::info!("Restarting system...");
                            unsafe { esp_idf_sys::esp_restart(); }
                        });
                        
                        Ok::<(), anyhow::Error>(())
                    }
                    Err(e) => {
                        log::error!("OTA update failed: {:?}", e);
                        let mut response = req.into_status_response(500)?;
                        let error_msg = format!("OTA update failed: {e}");
                        response.write_all(error_msg.as_bytes())?;
                        Ok::<(), anyhow::Error>(())
                    }
                }
            })?;
            
            // OTA status endpoint
            let ota_manager_clone3 = ota_manager.clone();
            server.fn_handler("/api/ota/status", esp_idf_svc::http::Method::Get, move |req| {
                let status_json = if let Some(ref ota_mgr) = ota_manager_clone3 {
                    let status = match ota_mgr.lock() {
                        Ok(mgr) => mgr.get_status(),
                        Err(e) => {
                            log::error!("Failed to lock OTA manager: {}", e);
                            crate::ota::OtaStatus::Failed
                        }
                    };
                    match status {
                        crate::ota::OtaStatus::Idle => r#"{"status":"idle"}"#.to_string(),
                        crate::ota::OtaStatus::Downloading { progress } => {
                            format!(r#"{{"status":"downloading","progress":{progress}}}"#)
                        },
                        crate::ota::OtaStatus::Verifying => r#"{"status":"verifying"}"#.to_string(),
                        crate::ota::OtaStatus::Ready => r#"{"status":"ready"}"#.to_string(),
                        crate::ota::OtaStatus::Failed => r#"{"status":"failed"}"#.to_string(),
                    }
                } else {
                    r#"{"status":"unavailable","message":"OTA not available on factory partition"}"#.to_string()
                };
                
                let mut response = req.into_response(
                    200,
                    Some("OK"),
                    &[("Content-Type", "application/json")]
                )?;
                response.write_all(status_json.as_bytes())?;
                Ok::<(), anyhow::Error>(())
            })?;
            
            log::info!("OTA endpoints registered on main web server");
        }

        // Dashboard route
        server.fn_handler("/dashboard", esp_idf_svc::http::Method::Get, move |req| {
            let version = crate::version::DISPLAY_VERSION;
            let html = include_str!("../templates/dashboard.html")
                .replace("{{VERSION}}", version);
            
            write_compressed_response(req, html.as_bytes(), "text/html; charset=utf-8")
        })?;

        // Binary metrics endpoint for efficient updates
        let metrics_clone_bin = metrics.clone();
        server.fn_handler("/api/metrics/binary", esp_idf_svc::http::Method::Get, move |req| {
            if let Ok(metrics_guard) = metrics_clone_bin.try_lock() {
                let packet = MetricsBinaryPacket::from_metrics(&*metrics_guard);
                let bytes = packet.to_bytes();
                
                let mut response = req.into_response(
                    200,
                    Some("OK"),
                    &[
                        ("Content-Type", "application/octet-stream"),
                        ("Cache-Control", "no-cache"),
                    ]
                )?;
                response.write_all(&bytes)?;
            } else {
                let mut response = req.into_status_response(503)?;
                response.write_all(b"Metrics temporarily unavailable")?;
            }
            
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // JSON metrics endpoint for dashboard
        let metrics_clone = metrics.clone();
        server.fn_handler("/api/metrics", esp_idf_svc::http::Method::Get, move |req| {
            // Get basic system info
            let uptime = unsafe { esp_idf_sys::esp_timer_get_time() / 1_000_000 } as u64;
            let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
            
            // Try to get metrics data
            let metrics_json = if let Ok(metrics_guard) = metrics_clone.try_lock() {
                // Get all metrics from the guard (deref to MetricsData)
                serde_json::json!({
                    "uptime": uptime,
                    "heap_free": heap_free,
                    "temperature": (metrics_guard.temperature * 10.0).round() / 10.0,
                    "fps_actual": (metrics_guard.fps_actual * 10.0).round() / 10.0,
                    "fps_target": metrics_guard.fps_target,
                    "render_time_ms": metrics_guard.render_time_ms,
                    "flush_time_ms": metrics_guard.flush_time_ms,
                    "cpu_usage": metrics_guard.cpu_usage,
                    "cpu0_usage": metrics_guard.cpu0_usage,
                    "cpu1_usage": metrics_guard.cpu1_usage,
                    "cpu_freq_mhz": metrics_guard.cpu_freq_mhz,
                    "battery_voltage": metrics_guard.battery_voltage_mv,
                    "battery_percentage": metrics_guard.battery_percentage,
                    "battery_charging": metrics_guard.battery_charging,
                    "wifi_rssi": metrics_guard.wifi_rssi,
                    "wifi_connected": metrics_guard.wifi_connected,
                    "wifi_ssid": metrics_guard.wifi_ssid.clone(),
                    "display_brightness": metrics_guard.display_brightness,
                    "frame_count": metrics_guard.frame_count,
                    "skip_count": metrics_guard.skip_count,
                    "skip_rate": if metrics_guard.frame_count > 0 {
                        metrics_guard.skip_count as f32 / metrics_guard.frame_count as f32 * 100.0
                    } else { 0.0 }
                })
            } else {
                // Return partial data if metrics locked
                serde_json::json!({
                    "uptime": uptime,
                    "heap_free": heap_free,
                    "error": "metrics_locked"
                })
            };
            
            let json_string = serde_json::to_string(&metrics_json)?;
            let mut response = req.into_ok_response()?;
            response.write_all(json_string.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Logs page
        server.fn_handler("/logs", esp_idf_svc::http::Method::Get, move |req| {
            let html = include_str!("../templates/logs.html");
            write_compressed_response(req, html.as_bytes(), "text/html; charset=utf-8")
        })?;

        // Logs API endpoint - returns recent log entries from telnet buffer
        server.fn_handler("/api/logs", esp_idf_svc::http::Method::Get, move |req| {
            // Get logs from the telnet server if available
            let logs = if let Some(telnet_server) = crate::logging::get_telnet_server() {
                let recent_logs = telnet_server.get_recent_logs(100);
                log::info!("Web API: Retrieved {} logs from telnet server", recent_logs.len());
                
                // Clean up the log entries - remove \r\n
                recent_logs.into_iter()
                    .map(|log| log.trim_end().to_string())
                    .collect()
            } else {
                log::warn!("Web API: No telnet server available for logs");
                Vec::new()
            };
            
            // Add some test logs if empty
            let logs = if logs.is_empty() {
                log::info!("Adding test logs since buffer is empty");
                vec![
                    format!("[{}] INFO  System started", esp_idf_svc::systime::EspSystemTime.now().as_secs()),
                    format!("[{}] INFO  WiFi connected", esp_idf_svc::systime::EspSystemTime.now().as_secs()),
                    format!("[{}] INFO  Web server ready", esp_idf_svc::systime::EspSystemTime.now().as_secs()),
                ]
            } else {
                logs
            };
            
            let logs_json = serde_json::json!({
                "logs": logs
            });
            
            let json_string = serde_json::to_string(&logs_json)?;
            let mut response = req.into_ok_response()?;
            response.write_all(json_string.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Device control endpoint
        let config_clone_control = config.clone();
        server.fn_handler("/api/control", esp_idf_svc::http::Method::Post, move |mut req| {
            let mut buf = vec![0; 512];
            let len = req.read(&mut buf)?;
            buf.truncate(len);
            
            let json_str = std::str::from_utf8(&buf)?;
            let control_cmd: serde_json::Value = serde_json::from_str(json_str)?;
            
            // Handle different control commands
            if let Some(brightness) = control_cmd.get("brightness").and_then(|v| v.as_u64()) {
                let brightness_u8 = brightness.min(255) as u8;
                
                // Update brightness in config
                if let Ok(mut cfg) = config_clone_control.lock() {
                    cfg.brightness = brightness_u8;
                    let _ = cfg.save();
                }
                log::info!("Brightness set to: {} ({}%)", brightness_u8, (brightness_u8 as f32 / 255.0 * 100.0) as u8);
            }
            
            if let Some(display_on) = control_cmd.get("display").and_then(|v| v.as_bool()) {
                // Display control would require access to the display manager
                // For now, just log the request
                log::info!("Display control requested: {} (not yet implemented)", display_on);
            }
            
            if let Some(mode) = control_cmd.get("mode").and_then(|v| v.as_str()) {
                // Set CPU frequency based on performance mode
                let freq_mhz = match mode {
                    "eco" => 80,
                    "normal" => 160,
                    "turbo" => 240,
                    _ => 160, // default to normal
                };
                
                // Configure power management
                unsafe {
                    use esp_idf_sys::*;
                    
                    // Create config struct
                    let config = esp_pm_config_t {
                        max_freq_mhz: freq_mhz,
                        min_freq_mhz: if mode == "eco" { 40 } else { 80 },
                        light_sleep_enable: false,
                    };
                    
                    // Apply the configuration
                    let result = esp_pm_configure(&config as *const esp_pm_config_t as *const core::ffi::c_void);
                    if result == ESP_OK as i32 {
                        log::info!("Performance mode set to {}: CPU {}MHz", mode, freq_mhz);
                    } else {
                        log::warn!("Failed to set performance mode: error {}", result);
                    }
                }
            }
            
            let mut response = req.into_ok_response()?;
            response.write_all(br#"{"status":"ok"}"#)?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Restart endpoint
        server.fn_handler("/api/restart", esp_idf_svc::http::Method::Post, move |req| {
            log::warn!("Device restart requested via web interface");
            
            let mut response = req.into_ok_response()?;
            response.write_all(br#"{"status":"restarting"}"#)?;
            
            // Schedule restart after response is sent
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(1));
                log::info!("Restarting device...");
                unsafe { esp_idf_sys::esp_restart(); }
            });
            
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // SSE (Server-Sent Events) endpoint - register with broadcaster
        let sse_broadcaster = crate::network::sse_broadcaster::init();
        sse_broadcaster.register_endpoints(&mut server)?;

        // Register API v1 routes
        let sensor_history = sensor_history.unwrap_or_else(|| {
            crate::sensors::history::init()
        });
        crate::network::api_routes::register_api_v1_routes(&mut server, config.clone(), sensor_history)?;

        // Register file manager routes
        crate::network::file_manager::register_file_routes(&mut server)?;
        
        // Server-Sent Events endpoint for real-time updates
        let metrics_clone_sse = metrics.clone();
        server.fn_handler("/api/events", esp_idf_svc::http::Method::Get, move |req| {
            
            let mut response = req.into_response(200, None, &[
                ("Content-Type", "text/event-stream"),
                ("Cache-Control", "no-cache"),
                ("Connection", "keep-alive"),
                ("Access-Control-Allow-Origin", "*"),
            ])?;
            
            // Send initial connection message
            response.write_all(b"retry: 1000\n\n")?;
            response.flush()?;
            
            // Send periodic updates
            for _ in 0..300 { // 5 minutes max connection
                // Get current metrics
                if let Ok(metrics_guard) = metrics_clone_sse.try_lock() {
                    let data = serde_json::json!({
                        "temperature": (metrics_guard.temperature * 10.0).round() / 10.0,
                        "battery_percentage": metrics_guard.battery_percentage,
                        "fps_actual": (metrics_guard.fps_actual * 10.0).round() / 10.0,
                        "fps_target": metrics_guard.fps_target,
                        "skip_rate": if metrics_guard.frame_count > 0 {
                            metrics_guard.skip_count as f32 / metrics_guard.frame_count as f32 * 100.0
                        } else { 0.0 },
                        "heap_free": metrics_guard.heap_free,
                        "timestamp": metrics_guard.timestamp,
                        "cpu0_usage": metrics_guard.cpu0_usage,
                        "cpu1_usage": metrics_guard.cpu1_usage,
                    });
                    
                    let event = format!("data: {}\n\n", serde_json::to_string(&data)?);
                    if response.write_all(event.as_bytes()).is_err() {
                        break; // Client disconnected
                    }
                    response.flush()?;
                }
                
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Recent logs endpoint for initial load
        server.fn_handler("/api/logs/recent", esp_idf_svc::http::Method::Get, move |req| {
            let count = req.uri()
                .split('?')
                .nth(1)
                .and_then(|query| query.split('&').find(|p| p.starts_with("count=")))
                .and_then(|p| p.strip_prefix("count="))
                .and_then(|c| c.parse::<usize>().ok())
                .unwrap_or(100);
            
            // Get log streamer instance
            let log_streamer = crate::network::log_streamer::init(None);
            let recent_logs = log_streamer.get_recent_logs(count);
            
            let json = serde_json::to_string(&recent_logs)?;
            let mut response = req.into_ok_response()?;
            response.write_all(json.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Service Worker
        server.fn_handler("/sw.js", esp_idf_svc::http::Method::Get, move |req| {
            const SW_JS: &str = include_str!("../templates/sw.js");
            let mut response = req.into_response(
                200,
                Some("OK"),
                &[
                    ("Content-Type", "application/javascript"),
                    ("Cache-Control", "no-cache"),
                ]
            )?;
            response.write_all(SW_JS.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Web App Manifest
        server.fn_handler("/manifest.json", esp_idf_svc::http::Method::Get, move |req| {
            // Use escaped quotes to avoid parsing issues
            const MANIFEST_JSON: &str = "{\"name\":\"ESP32-S3 Dashboard\",\"short_name\":\"ESP32 Dash\",\"description\":\"Control and monitor your ESP32-S3 device\",\"start_url\":\"/dashboard\",\"display\":\"standalone\",\"theme_color\":\"#3b82f6\",\"background_color\":\"#0a0a0a\",\"icons\":[{\"src\":\"/icon-192.png\",\"sizes\":\"192x192\",\"type\":\"image/png\"},{\"src\":\"/icon-512.png\",\"sizes\":\"512x512\",\"type\":\"image/png\"}]}";
            let mut response = req.into_response(
                200,
                Some("OK"),
                &[("Content-Type", "application/manifest+json")]
            )?;
            response.write_all(MANIFEST_JSON.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        Ok(Self { _server: server })
    }
}

#[derive(serde::Serialize)]
struct SystemInfo {
    version: String,
    ssid: String,
    free_heap: u32,
    uptime_ms: u64,
}
