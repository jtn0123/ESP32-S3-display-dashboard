use anyhow::{Result, bail};
use esp_idf_svc::eventloop::{EspEventLoop, System};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// WiFi reconnection manager that handles disconnection events
pub struct WifiReconnectManager {
    ssid: String,
    password: String,
    last_disconnect: Arc<Mutex<Option<Instant>>>,
    reconnect_attempts: Arc<Mutex<u32>>,
}

impl WifiReconnectManager {
    pub fn new(ssid: String, password: String) -> Self {
        Self {
            ssid,
            password,
            last_disconnect: Arc::new(Mutex::new(None)),
            reconnect_attempts: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Register WiFi event handlers for automatic reconnection
    pub fn register_event_handlers(&self, _sysloop: &EspEventLoop<System>) -> Result<()> {
        // Note: In newer esp-idf-svc versions, WiFi events are handled differently
        // For now, we'll rely on the retry logic in WifiManager::connect_and_get_signal
        log::info!("WiFi reconnection logic configured in connection manager");
        Ok(())
    }
    
    /// Force a WiFi reconnection (useful after OTA)
    pub fn force_reconnect() -> Result<()> {
        log::info!("Forcing WiFi reconnection...");
        
        unsafe {
            // Disconnect first
            let _ = esp_idf_sys::esp_wifi_disconnect();
            
            // Wait a bit
            std::thread::sleep(Duration::from_millis(500));
            
            // Reconnect
            let result = esp_idf_sys::esp_wifi_connect();
            if result == esp_idf_sys::ESP_OK {
                log::info!("WiFi reconnection initiated");
                Ok(())
            } else {
                bail!("Failed to initiate WiFi reconnection: {:?}", result)
            }
        }
    }
}

/// Check if we just completed an OTA update and force reconnection
pub fn handle_post_ota_wifi() -> Result<()> {
    let reset_reason = unsafe { esp_idf_sys::esp_reset_reason() };
    
    if reset_reason == esp_idf_sys::esp_reset_reason_t_ESP_RST_SW {
        log::info!("Detected software reset (likely OTA), handling WiFi reconnection");
        
        // Give WiFi subsystem more time to fully initialize after OTA
        log::info!("Waiting for WiFi subsystem to stabilize...");
        std::thread::sleep(Duration::from_secs(3));
        
        // Stop WiFi first to ensure clean state
        unsafe {
            let stop_result = esp_idf_sys::esp_wifi_stop();
            if stop_result == esp_idf_sys::ESP_OK {
                log::info!("WiFi stopped successfully");
            } else {
                log::warn!("WiFi stop returned: {:?}", stop_result);
            }
        }
        
        // Small delay between stop and start
        std::thread::sleep(Duration::from_millis(500));
        
        // Start WiFi
        unsafe {
            let start_result = esp_idf_sys::esp_wifi_start();
            if start_result == esp_idf_sys::ESP_OK {
                log::info!("WiFi started successfully");
            } else {
                log::error!("WiFi start failed: {:?}", start_result);
                return Err(anyhow::anyhow!("Failed to start WiFi"));
            }
        }
        
        // Wait a bit more before attempting connection
        std::thread::sleep(Duration::from_secs(1));
        
        // Force reconnection with retries
        for attempt in 1..=3 {
            log::info!("WiFi reconnection attempt {} of 3", attempt);
            match WifiReconnectManager::force_reconnect() {
                Ok(_) => {
                    log::info!("WiFi reconnection initiated successfully");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Reconnection attempt {} failed: {:?}", attempt, e);
                    if attempt < 3 {
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            }
        }
        
        log::error!("Failed to reconnect WiFi after 3 attempts");
    }
    
    Ok(())
}