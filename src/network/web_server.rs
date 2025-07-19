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

        // Always add OTA endpoints (they'll show error if OTA not available)
        {
            log::info!("Adding OTA endpoints to web server...");
            let ota_mgr_clone = ota_manager.clone();
            
            // OTA web interface
            server.fn_handler("/ota", esp_idf_svc::http::Method::Get, move |req| {
                let mut response = req.into_ok_response()?;
                
                if ota_mgr_clone.is_some() {
                    response.write_all(crate::ota::web_server::OTA_HTML.as_bytes())?;
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