use anyhow::Result;
use serde::{Deserialize, Serialize};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};

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
        // Get WiFi credentials from environment variables set by build.rs
        // These come from wifi_config.h which should NOT be committed to git
        let wifi_ssid = env!("WIFI_SSID");
        let wifi_password = env!("WIFI_PASSWORD");
        
        log::info!("Config default: SSID='{}', Password={}", 
            wifi_ssid, 
            if wifi_password.is_empty() { "<empty>" } else { "<set>" }
        );
        
        Self {
            wifi_ssid: wifi_ssid.to_string(),
            wifi_password: wifi_password.to_string(),
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
        Ok(mut config) => {
            log::info!("Loaded configuration from NVS");
            
            // If NVS has empty WiFi credentials, use the compiled-in ones
            if config.wifi_ssid.is_empty() || config.wifi_password.is_empty() {
                let default_config = Config::default();
                log::warn!("NVS WiFi credentials empty, using compiled defaults: SSID='{}'", default_config.wifi_ssid);
                config.wifi_ssid = default_config.wifi_ssid;
                config.wifi_password = default_config.wifi_password;
                
                // Save the updated config back to NVS
                if let Err(e) = config.save() {
                    log::warn!("Failed to save updated config with WiFi credentials: {:?}", e);
                }
            }
            
            Ok(config)
        }
        Err(e) => {
            log::warn!("Failed to load config from NVS: {:?}, using defaults", e);
            let config = Config::default();
            
            // Try to save default config to NVS for next time
            if let Err(save_err) = config.save() {
                log::warn!("Failed to save default config to NVS: {:?}", save_err);
            }
            
            Ok(config)
        }
    }
}

// Remove duplicate save function - already exists as method on Config

fn load_from_nvs() -> Result<Config> {
    let nvs_partition = EspDefaultNvsPartition::take()?;
    let nvs = EspNvs::new(nvs_partition, CONFIG_NAMESPACE, true)?;
    
    let mut buf = vec![0u8; 2048]; // Max config size
    let data = nvs.get_blob(CONFIG_KEY, &mut buf)?
        .ok_or_else(|| anyhow::anyhow!("Config not found in NVS"))?;
    
    let config: Config = serde_json::from_slice(data)?;
    
    Ok(config)
}

fn save_to_nvs(config: &Config) -> Result<()> {
    let nvs_partition = EspDefaultNvsPartition::take()?;
    let mut nvs = EspNvs::new(nvs_partition, CONFIG_NAMESPACE, false)?;
    
    let json = serde_json::to_vec(config)?;
    nvs.set_blob(CONFIG_KEY, &json)?;
    
    Ok(())
}

// CONFIG_HTML moved to web_server module where it's actually used