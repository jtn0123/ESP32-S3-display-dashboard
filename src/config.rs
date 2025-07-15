use anyhow::Result;
use serde::{Deserialize, Serialize};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, EspNvsPartition, NvsDefault};

const CONFIG_NAMESPACE: &str = "dashboard";
const CONFIG_KEY: &str = "config";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // WiFi settings
    pub wifi_ssid: String,
    pub wifi_password: String,
    
    // Display settings
    pub brightness: u8,
    pub auto_brightness: bool,
    
    // Power management
    pub dim_timeout_secs: u32,
    pub sleep_timeout_secs: u32,
    
    // UI preferences
    pub theme: Theme,
    pub show_animations: bool,
    
    // OTA settings
    pub ota_enabled: bool,
    pub ota_check_interval_hours: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    Auto,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            brightness: 80,
            auto_brightness: true,
            dim_timeout_secs: 30,
            sleep_timeout_secs: 300,
            theme: Theme::Dark,
            show_animations: true,
            ota_enabled: true,
            ota_check_interval_hours: 24,
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        save_to_nvs(self)?;
        log::info!("Configuration saved to NVS");
        Ok(())
    }
}

pub fn load_or_default() -> Result<Config> {
    match load_from_nvs() {
        Ok(config) => {
            log::info!("Loaded configuration from NVS");
            Ok(config)
        }
        Err(e) => {
            log::warn!("Failed to load config from NVS: {:?}, using defaults", e);
            Ok(Config::default())
        }
    }
}

pub fn save(config: &Config) -> Result<()> {
    save_to_nvs(config)?;
    log::info!("Configuration saved to NVS");
    Ok(())
}

fn load_from_nvs() -> Result<Config> {
    let nvs_partition = EspDefaultNvsPartition::take()?;
    let nvs = EspNvs::new(nvs_partition, CONFIG_NAMESPACE, true)?;
    
    let mut buf = vec![0u8; 2048]; // Max config size
    let size = nvs.get_blob(CONFIG_KEY, &mut buf)?
        .ok_or_else(|| anyhow::anyhow!("Config not found in NVS"))?;
    
    buf.truncate(size);
    let config: Config = serde_json::from_slice(&buf)?;
    
    Ok(config)
}

fn save_to_nvs(config: &Config) -> Result<()> {
    let nvs_partition = EspDefaultNvsPartition::take()?;
    let mut nvs = EspNvs::new(nvs_partition, CONFIG_NAMESPACE, false)?;
    
    let json = serde_json::to_vec(config)?;
    nvs.set_blob(CONFIG_KEY, &json)?;
    
    Ok(())
}

// Configuration web page HTML
pub const CONFIG_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 Dashboard Config</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #1a1a1a; color: #fff; }
        .container { max-width: 600px; margin: 0 auto; }
        h1 { color: #4CAF50; }
        .form-group { margin-bottom: 15px; }
        label { display: block; margin-bottom: 5px; }
        input, select { width: 100%; padding: 8px; border: 1px solid #444; background: #2a2a2a; color: #fff; }
        button { background: #4CAF50; color: white; padding: 10px 20px; border: none; cursor: pointer; }
        button:hover { background: #45a049; }
        .status { padding: 10px; margin: 10px 0; border-radius: 4px; }
        .success { background: #4CAF50; }
        .error { background: #f44336; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Dashboard Configuration</h1>
        <form id="configForm">
            <h2>WiFi Settings</h2>
            <div class="form-group">
                <label for="ssid">SSID:</label>
                <input type="text" id="ssid" name="wifi_ssid" required>
            </div>
            <div class="form-group">
                <label for="password">Password:</label>
                <input type="password" id="password" name="wifi_password">
            </div>
            
            <h2>Display Settings</h2>
            <div class="form-group">
                <label for="brightness">Brightness (0-100):</label>
                <input type="number" id="brightness" name="brightness" min="0" max="100" value="80">
            </div>
            <div class="form-group">
                <label for="auto_brightness">
                    <input type="checkbox" id="auto_brightness" name="auto_brightness" checked>
                    Auto Brightness
                </label>
            </div>
            
            <h2>Power Management</h2>
            <div class="form-group">
                <label for="dim_timeout">Dim Timeout (seconds):</label>
                <input type="number" id="dim_timeout" name="dim_timeout_secs" value="30">
            </div>
            <div class="form-group">
                <label for="sleep_timeout">Sleep Timeout (seconds):</label>
                <input type="number" id="sleep_timeout" name="sleep_timeout_secs" value="300">
            </div>
            
            <button type="submit">Save Configuration</button>
        </form>
        <div id="status"></div>
    </div>
    
    <script>
        document.getElementById('configForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const formData = new FormData(e.target);
            const config = Object.fromEntries(formData);
            
            try {
                const response = await fetch('/api/config', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(config)
                });
                
                const status = document.getElementById('status');
                if (response.ok) {
                    status.className = 'status success';
                    status.textContent = 'Configuration saved! Restarting...';
                    setTimeout(() => window.location.reload(), 3000);
                } else {
                    status.className = 'status error';
                    status.textContent = 'Failed to save configuration';
                }
            } catch (error) {
                document.getElementById('status').className = 'status error';
                document.getElementById('status').textContent = 'Error: ' + error.message;
            }
        });
        
        // Load current config
        fetch('/api/config')
            .then(r => r.json())
            .then(config => {
                for (const [key, value] of Object.entries(config)) {
                    const input = document.querySelector(`[name="${key}"]`);
                    if (input) {
                        if (input.type === 'checkbox') {
                            input.checked = value;
                        } else {
                            input.value = value;
                        }
                    }
                }
            });
    </script>
</body>
</html>
"#;