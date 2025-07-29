use anyhow::Result;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::ota::OtaManager;

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
    #[allow(dead_code)]
    pub fn new(config: Arc<Mutex<Config>>) -> Result<Self> {
        Self::new_with_ota(config, None)
    }
    
    pub fn new_with_ota(config: Arc<Mutex<Config>>, ota_manager: Option<Arc<Mutex<OtaManager>>>) -> Result<Self> {
        let mut server = EspHttpServer::new(&Configuration::default())?;
        
        let _config_clone = config.clone();
        
        // Home page
        server.fn_handler("/", esp_idf_svc::http::Method::Get, move |req| {
            let mut response = req.into_ok_response()?;
            response.write_all(HOME_PAGE.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>> as Result<(), Box<dyn std::error::Error>>
        })?;

        // Get current configuration
        let config_clone2 = config.clone();
        server.fn_handler("/api/config", esp_idf_svc::http::Method::Get, move |req| {
            let config = config_clone2.lock().unwrap();
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
                let mut config = config_clone3.lock().unwrap();
                *config = new_config;
                config.save()?;
            }
            
            let _response = req.into_ok_response()?;
            Ok(()) as Result<(), Box<dyn std::error::Error>> as Result<(), Box<dyn std::error::Error>>
        })?;

        // System info endpoint
        server.fn_handler("/api/system", esp_idf_svc::http::Method::Get, move |req| {
            let info = SystemInfo {
                version: env!("CARGO_PKG_VERSION").to_string(),
                free_heap: unsafe { esp_idf_sys::esp_get_free_heap_size() },
                uptime_ms: unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 },
            };
            
            let json = serde_json::to_string(&info)?;
            let mut response = req.into_ok_response()?;
            response.write_all(json.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Prometheus metrics endpoint
        server.fn_handler("/metrics", esp_idf_svc::http::Method::Get, move |req| {
            // Get system metrics
            let uptime_seconds = unsafe { esp_idf_sys::esp_timer_get_time() / 1_000_000 } as u64;
            let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
            let heap_total = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
            
            // Get device info for labels
            let version = crate::version::DISPLAY_VERSION;
            let board_type = "ESP32-S3";
            let chip_model = "T-Display-S3";
            
            // Get metrics data
            let metrics_data = crate::metrics::metrics().lock().unwrap().clone();
            
            // Format all metrics in Prometheus format
            let metrics = format!(
                "# HELP esp32_device_info Device information\n\
                 # TYPE esp32_device_info gauge\n\
                 esp32_device_info{{version=\"{}\",board=\"{}\",model=\"{}\"}} 1\n\
                 \n\
                 # HELP esp32_uptime_seconds Total uptime in seconds\n\
                 # TYPE esp32_uptime_seconds counter\n\
                 esp32_uptime_seconds {}\n\
                 \n\
                 # HELP esp32_heap_free_bytes Current free heap memory in bytes\n\
                 # TYPE esp32_heap_free_bytes gauge\n\
                 esp32_heap_free_bytes {}\n\
                 \n\
                 # HELP esp32_heap_total_bytes Total heap memory in bytes\n\
                 # TYPE esp32_heap_total_bytes gauge\n\
                 esp32_heap_total_bytes {}\n\
                 \n\
                 # HELP esp32_fps_actual Current actual frames per second\n\
                 # TYPE esp32_fps_actual gauge\n\
                 esp32_fps_actual {:.1}\n\
                 \n\
                 # HELP esp32_fps_target Target frames per second\n\
                 # TYPE esp32_fps_target gauge\n\
                 esp32_fps_target {:.1}\n\
                 \n\
                 # HELP esp32_cpu_usage_percent CPU usage percentage (average)\n\
                 # TYPE esp32_cpu_usage_percent gauge\n\
                 esp32_cpu_usage_percent {}\n\
                 \n\
                 # HELP esp32_cpu0_usage_percent CPU Core 0 usage percentage\n\
                 # TYPE esp32_cpu0_usage_percent gauge\n\
                 esp32_cpu0_usage_percent {}\n\
                 \n\
                 # HELP esp32_cpu1_usage_percent CPU Core 1 usage percentage\n\
                 # TYPE esp32_cpu1_usage_percent gauge\n\
                 esp32_cpu1_usage_percent {}\n\
                 \n\
                 # HELP esp32_cpu_freq_mhz CPU frequency in MHz\n\
                 # TYPE esp32_cpu_freq_mhz gauge\n\
                 esp32_cpu_freq_mhz {}\n\
                 \n\
                 # HELP esp32_temperature_celsius Internal temperature in Celsius\n\
                 # TYPE esp32_temperature_celsius gauge\n\
                 esp32_temperature_celsius {:.1}\n\
                 \n\
                 # HELP esp32_wifi_rssi_dbm WiFi signal strength in dBm\n\
                 # TYPE esp32_wifi_rssi_dbm gauge\n\
                 esp32_wifi_rssi_dbm {}\n\
                 \n\
                 # HELP esp32_wifi_connected WiFi connection status (0=disconnected, 1=connected)\n\
                 # TYPE esp32_wifi_connected gauge\n\
                 esp32_wifi_connected{}{{ssid=\"{}\"}} {}\n\
                 \n\
                 # HELP esp32_display_brightness Display brightness level (0-255)\n\
                 # TYPE esp32_display_brightness gauge\n\
                 esp32_display_brightness {}\n\
                 \n\
                 # HELP esp32_battery_voltage_mv Battery voltage in millivolts\n\
                 # TYPE esp32_battery_voltage_mv gauge\n\
                 esp32_battery_voltage_mv {}\n\
                 \n\
                 # HELP esp32_battery_percentage Battery charge percentage\n\
                 # TYPE esp32_battery_percentage gauge\n\
                 esp32_battery_percentage {}\n\
                 \n\
                 # HELP esp32_battery_charging Battery charging status (0=not charging, 1=charging)\n\
                 # TYPE esp32_battery_charging gauge\n\
                 esp32_battery_charging {}\n\
                 \n\
                 # HELP esp32_render_time_milliseconds Display render time in milliseconds\n\
                 # TYPE esp32_render_time_milliseconds gauge\n\
                 esp32_render_time_milliseconds {}\n\
                 \n\
                 # HELP esp32_flush_time_milliseconds Display flush time in milliseconds\n\
                 # TYPE esp32_flush_time_milliseconds gauge\n\
                 esp32_flush_time_milliseconds {}\n\
                 \n\
                 # HELP esp32_frame_skip_rate_percent Percentage of frames skipped\n\
                 # TYPE esp32_frame_skip_rate_percent gauge\n\
                 esp32_frame_skip_rate_percent {:.1}\n\
                 \n\
                 # HELP esp32_total_frames_count Total number of frames processed\n\
                 # TYPE esp32_total_frames_count counter\n\
                 esp32_total_frames_count {}\n\
                 \n\
                 # HELP esp32_skipped_frames_count Number of frames skipped\n\
                 # TYPE esp32_skipped_frames_count counter\n\
                 esp32_skipped_frames_count {}\n\
                 \n\
                 # HELP esp32_psram_free_bytes Free PSRAM memory in bytes\n\
                 # TYPE esp32_psram_free_bytes gauge\n\
                 esp32_psram_free_bytes {}\n\
                 \n\
                 # HELP esp32_psram_total_bytes Total PSRAM memory in bytes\n\
                 # TYPE esp32_psram_total_bytes gauge\n\
                 esp32_psram_total_bytes {}\n\
                 \n\
                 # HELP esp32_psram_used_percent PSRAM usage percentage\n\
                 # TYPE esp32_psram_used_percent gauge\n\
                 esp32_psram_used_percent {:.1}\n",
                version,
                board_type,
                chip_model,
                uptime_seconds,
                heap_free,
                heap_total,
                metrics_data.fps_actual,
                metrics_data.fps_target,
                metrics_data.cpu_usage_percent,
                metrics_data.cpu0_usage_percent,
                metrics_data.cpu1_usage_percent,
                metrics_data.cpu_freq_mhz,
                metrics_data.temperature_celsius,
                metrics_data.wifi_rssi_dbm,
                if metrics_data.wifi_connected { "" } else { "_disconnected" },
                metrics_data.wifi_ssid,
                if metrics_data.wifi_connected { 1 } else { 0 },
                metrics_data.display_brightness,
                metrics_data.battery_voltage_mv,
                metrics_data.battery_percentage,
                if metrics_data.is_charging { 1 } else { 0 },
                metrics_data.render_time_ms,
                metrics_data.flush_time_ms,
                if metrics_data.frame_count > 0 {
                    (metrics_data.skip_count as f32 / metrics_data.frame_count as f32 * 100.0)
                } else {
                    0.0
                },
                metrics_data.frame_count,
                metrics_data.skip_count,
                metrics_data.psram_free_bytes,
                metrics_data.psram_total_bytes,
                if metrics_data.psram_total_bytes > 0 {
                    ((metrics_data.psram_total_bytes - metrics_data.psram_free_bytes) as f32 / metrics_data.psram_total_bytes as f32 * 100.0)
                } else {
                    0.0
                }
            );
            
            let mut response = req.into_response(
                200,
                Some("OK"),
                &[("Content-Type", "text/plain; version=0.0.4")]
            )?;
            response.write_all(metrics.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        // Always add OTA endpoints (they'll show error if OTA not available)
        {
            log::info!("Adding OTA endpoints to web server...");
            let ota_mgr_clone = ota_manager.clone();
            
            // OTA web interface
            server.fn_handler("/ota", esp_idf_svc::http::Method::Get, move |req| {
                let mut response = req.into_ok_response()?;
                
                if ota_mgr_clone.is_some() {
                    const OTA_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 Dashboard OTA Update</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f0f2f5;
            color: #1a1a1a;
        }
        .container {
            max-width: 600px;
            margin: 0 auto;
            background-color: white;
            border-radius: 12px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
            padding: 30px;
        }
        h1 {
            color: #2563eb;
            margin-bottom: 10px;
        }
        .subtitle {
            color: #6b7280;
            margin-bottom: 30px;
        }
        .file-select {
            margin: 20px 0;
        }
        input[type="file"] {
            display: none;
        }
        .file-label {
            display: inline-block;
            padding: 12px 24px;
            background-color: #f3f4f6;
            color: #374151;
            border-radius: 8px;
            cursor: pointer;
            transition: background-color 0.2s;
        }
        .file-label:hover {
            background-color: #e5e7eb;
        }
        .file-name {
            margin-left: 15px;
            color: #6b7280;
        }
        button {
            background-color: #2563eb;
            color: white;
            border: none;
            padding: 12px 32px;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 500;
            cursor: pointer;
            transition: background-color 0.2s;
            width: 100%;
            margin-top: 20px;
        }
        button:hover:not(:disabled) {
            background-color: #1d4ed8;
        }
        button:disabled {
            background-color: #9ca3af;
            cursor: not-allowed;
        }
        .progress-container {
            display: none;
            margin-top: 30px;
        }
        .progress-bar {
            width: 100%;
            height: 24px;
            background-color: #f3f4f6;
            border-radius: 12px;
            overflow: hidden;
            position: relative;
        }
        .progress-fill {
            height: 100%;
            background: linear-gradient(90deg, #2563eb, #3b82f6);
            width: 0%;
            transition: width 0.3s ease;
            position: relative;
        }
        .progress-text {
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            font-size: 12px;
            font-weight: 500;
            color: #374151;
            z-index: 10;
        }
        .status {
            margin-top: 15px;
            text-align: center;
            color: #6b7280;
            font-size: 14px;
        }
        .error {
            color: #dc2626;
            margin-top: 15px;
            padding: 12px;
            background-color: #fee2e2;
            border-radius: 8px;
            display: none;
        }
        .back-link {
            display: inline-block;
            margin-top: 30px;
            color: #2563eb;
            text-decoration: none;
            font-size: 14px;
        }
        .back-link:hover {
            text-decoration: underline;
        }
        .warning {
            background-color: #fef3c7;
            border: 1px solid #f59e0b;
            color: #92400e;
            padding: 12px;
            border-radius: 8px;
            margin-bottom: 20px;
            font-size: 14px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>ESP32-S3 Dashboard</h1>
        <p class="subtitle">Over-The-Air Firmware Update</p>
        
        <div class="warning">
            ⚠️ Do not disconnect power during update process
        </div>
        
        <div class="file-select">
            <label for="file" class="file-label">Choose Firmware File</label>
            <span class="file-name" id="fileName">No file selected</span>
            <input type="file" id="file" accept=".bin" />
        </div>
        
        <button id="uploadBtn" disabled>Upload Firmware</button>
        
        <div class="progress-container" id="progressContainer">
            <div class="progress-bar">
                <div class="progress-fill" id="progressFill"></div>
                <div class="progress-text" id="progressText">0%</div>
            </div>
            <p class="status" id="status">Uploading...</p>
        </div>
        
        <div class="error" id="error"></div>
        
        <a href="/" class="back-link">← Back to Settings</a>
    </div>

    <script>
        const fileInput = document.getElementById('file');
        const fileName = document.getElementById('fileName');
        const uploadBtn = document.getElementById('uploadBtn');
        const progressContainer = document.getElementById('progressContainer');
        const progressFill = document.getElementById('progressFill');
        const progressText = document.getElementById('progressText');
        const status = document.getElementById('status');
        const error = document.getElementById('error');
        
        fileInput.addEventListener('change', function() {
            if (this.files && this.files[0]) {
                fileName.textContent = this.files[0].name;
                uploadBtn.disabled = false;
            }
        });
        
        uploadBtn.addEventListener('click', async function() {
            const file = fileInput.files[0];
            if (!file) return;
            
            // Disable controls
            uploadBtn.disabled = true;
            fileInput.disabled = true;
            error.style.display = 'none';
            
            // Show progress
            progressContainer.style.display = 'block';
            
            const formData = new FormData();
            formData.append('firmware', file);
            
            try {
                const xhr = new XMLHttpRequest();
                
                xhr.upload.addEventListener('progress', function(e) {
                    if (e.lengthComputable) {
                        const percentComplete = Math.round((e.loaded / e.total) * 100);
                        progressFill.style.width = percentComplete + '%';
                        progressText.textContent = percentComplete + '%';
                    }
                });
                
                xhr.onreadystatechange = function() {
                    if (xhr.readyState === XMLHttpRequest.DONE) {
                        if (xhr.status === 200) {
                            progressFill.style.width = '100%';
                            progressText.textContent = '100%';
                            status.textContent = 'Update successful! Device will restart...';
                            status.style.color = '#059669';
                            
                            setTimeout(() => {
                                status.textContent = 'Restarting... Please wait 30 seconds then refresh the page.';
                            }, 2000);
                        } else {
                            error.textContent = 'Update failed: ' + (xhr.responseText || 'Unknown error');
                            error.style.display = 'block';
                            uploadBtn.disabled = false;
                            fileInput.disabled = false;
                            progressContainer.style.display = 'none';
                        }
                    }
                };
                
                xhr.open('POST', '/ota/update');
                xhr.send(file);
                
            } catch (err) {
                error.textContent = 'Upload error: ' + err.message;
                error.style.display = 'block';
                uploadBtn.disabled = false;
                fileInput.disabled = false;
                progressContainer.style.display = 'none';
            }
        });
    </script>
</body>
</html>"#;
                    response.write_all(OTA_HTML.as_bytes())?;
                } else {
                    // Show message that OTA will be available after first USB update
                    let msg = r#"<!DOCTYPE html>
<html>
<head>
    <title>OTA Update</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .info { padding: 20px; border: 1px solid #0066cc; background: #e6f2ff; }
    </style>
</head>
<body>
    <h1>OTA Update</h1>
    <div class="info">
        <p>OTA updates will be available after the next USB firmware update.</p>
        <p>Once enabled, you can update wirelessly using:</p>
        <pre>./ota.sh 10.27.27.201</pre>
    </div>
    <p><a href="/">Back to Settings</a></p>
</body>
</html>"#;
                    response.write_all(msg.as_bytes())?;
                }
                Ok::<(), anyhow::Error>(())
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
                    let mut ota = ota_mgr.lock().unwrap();
                    
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
                        let error_msg = format!("OTA update failed: {}", e);
                        response.write_all(error_msg.as_bytes())?;
                        Ok::<(), anyhow::Error>(())
                    }
                }
            })?;
            
            // OTA status endpoint
            let ota_manager_clone3 = ota_manager.clone();
            server.fn_handler("/api/ota/status", esp_idf_svc::http::Method::Get, move |req| {
                let status_json = if let Some(ref ota_mgr) = ota_manager_clone3 {
                    let ota = ota_mgr.lock().unwrap();
                    let status = ota.get_status();
                    match status {
                        crate::ota::OtaStatus::Idle => r#"{"status":"idle"}"#.to_string(),
                        crate::ota::OtaStatus::Downloading { progress } => {
                            format!(r#"{{"status":"downloading","progress":{}}}"#, progress)
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

        Ok(Self { _server: server })
    }
}

#[derive(serde::Serialize)]
struct SystemInfo {
    version: String,
    free_heap: u32,
    uptime_ms: u64,
}

const HOME_PAGE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 Dashboard Configuration</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body {
            font-family: Arial, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f0f0f0;
        }
        .container {
            max-width: 600px;
            margin: 0 auto;
            background-color: white;
            padding: 20px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            text-align: center;
        }
        .form-group {
            margin-bottom: 15px;
        }
        label {
            display: block;
            margin-bottom: 5px;
            color: #555;
            font-weight: bold;
        }
        input[type="text"],
        input[type="password"],
        input[type="number"],
        select {
            width: 100%;
            padding: 10px;
            border: 1px solid #ddd;
            border-radius: 5px;
            box-sizing: border-box;
        }
        input[type="checkbox"] {
            margin-right: 10px;
        }
        button {
            background-color: #4CAF50;
            color: white;
            padding: 12px 20px;
            border: none;
            border-radius: 5px;
            cursor: pointer;
            width: 100%;
            font-size: 16px;
        }
        button:hover {
            background-color: #45a049;
        }
        .status {
            margin-top: 20px;
            padding: 10px;
            border-radius: 5px;
            text-align: center;
        }
        .success {
            background-color: #d4edda;
            color: #155724;
            border: 1px solid #c3e6cb;
        }
        .error {
            background-color: #f8d7da;
            color: #721c24;
            border: 1px solid #f5c6cb;
        }
        .info {
            background-color: #d1ecf1;
            color: #0c5460;
            border: 1px solid #bee5eb;
            margin-bottom: 20px;
        }
        .system-info {
            font-size: 14px;
            color: #666;
            text-align: center;
            margin-top: 20px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>ESP32-S3 Dashboard</h1>
        <div class="info">
            <strong>System Status:</strong>
            <span id="system-status">Loading...</span>
        </div>
        
        <form id="configForm">
            <h2>WiFi Configuration</h2>
            
            <div class="form-group">
                <label for="wifi_ssid">WiFi SSID:</label>
                <input type="text" id="wifi_ssid" name="wifi_ssid" required>
            </div>
            
            <div class="form-group">
                <label for="wifi_password">WiFi Password:</label>
                <input type="password" id="wifi_password" name="wifi_password">
            </div>
            
            <h2>Display Settings</h2>
            
            <div class="form-group">
                <label for="brightness">Brightness (0-255):</label>
                <input type="number" id="brightness" name="brightness" min="0" max="255" value="255">
            </div>
            
            <div class="form-group">
                <label for="auto_dim">
                    <input type="checkbox" id="auto_dim" name="auto_dim">
                    Auto-dim display
                </label>
            </div>
            
            <div class="form-group">
                <label for="update_interval">Update Interval (seconds):</label>
                <input type="number" id="update_interval" name="update_interval" min="1" max="60" value="5">
            </div>
            
            <h2>OTA Updates</h2>
            
            <div class="form-group">
                <label for="ota_url">OTA Server URL:</label>
                <input type="text" id="ota_url" name="ota_url" placeholder="http://example.com/firmware">
            </div>
            
            <div class="form-group">
                <label for="auto_update">
                    <input type="checkbox" id="auto_update" name="auto_update">
                    Enable automatic updates
                </label>
            </div>
            
            <button type="submit">Save Configuration</button>
        </form>
        
        <div id="status" class="status" style="display: none;"></div>
        
        <div class="system-info" id="systemInfo"></div>
    </div>

    <script>
        // Load current configuration
        async function loadConfig() {
            try {
                const response = await fetch('/api/config');
                const config = await response.json();
                
                // Populate form fields
                document.getElementById('wifi_ssid').value = config.wifi_ssid || '';
                document.getElementById('wifi_password').value = config.wifi_password || '';
                document.getElementById('brightness').value = config.brightness || 255;
                document.getElementById('auto_dim').checked = config.auto_dim || false;
                document.getElementById('update_interval').value = config.update_interval || 5;
                document.getElementById('ota_url').value = config.ota_url || '';
                document.getElementById('auto_update').checked = config.auto_update || false;
            } catch (error) {
                showStatus('Failed to load configuration', 'error');
            }
        }
        
        // Load system info
        async function loadSystemInfo() {
            try {
                const response = await fetch('/api/system');
                const info = await response.json();
                
                const uptimeMinutes = Math.floor(info.uptime_ms / 60000);
                const uptimeHours = Math.floor(uptimeMinutes / 60);
                const uptimeStr = uptimeHours > 0 
                    ? `${uptimeHours}h ${uptimeMinutes % 60}m`
                    : `${uptimeMinutes}m`;
                
                document.getElementById('system-status').textContent = 
                    `Version ${info.version} | Heap: ${Math.floor(info.free_heap / 1024)}KB | Uptime: ${uptimeStr}`;
                
                document.getElementById('systemInfo').textContent = 
                    `Free memory: ${info.free_heap} bytes | Uptime: ${uptimeStr}`;
            } catch (error) {
                document.getElementById('system-status').textContent = 'Error loading status';
            }
        }
        
        // Handle form submission
        document.getElementById('configForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const formData = new FormData(e.target);
            const config = {
                wifi_ssid: formData.get('wifi_ssid'),
                wifi_password: formData.get('wifi_password'),
                brightness: parseInt(formData.get('brightness')),
                auto_dim: formData.get('auto_dim') === 'on',
                update_interval: parseInt(formData.get('update_interval')),
                ota_url: formData.get('ota_url'),
                auto_update: formData.get('auto_update') === 'on'
            };
            
            try {
                const response = await fetch('/api/config', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify(config)
                });
                
                if (response.ok) {
                    showStatus('Configuration saved successfully!', 'success');
                } else {
                    showStatus('Failed to save configuration', 'error');
                }
            } catch (error) {
                showStatus('Error: ' + error.message, 'error');
            }
        });
        
        function showStatus(message, type) {
            const status = document.getElementById('status');
            status.textContent = message;
            status.className = 'status ' + type;
            status.style.display = 'block';
            
            setTimeout(() => {
                status.style.display = 'none';
            }, 5000);
        }
        
        // Load data on page load
        loadConfig();
        loadSystemInfo();
        
        // Refresh system info every 10 seconds
        setInterval(loadSystemInfo, 10000);
    </script>
</body>
</html>"#;