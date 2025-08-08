use anyhow::{Result, bail};
use esp_idf_svc::eventloop::{EspEventLoop, System};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

/// WiFi reconnection manager that handles disconnection events
pub struct WifiReconnectManager {
    ssid: String,
    password: String,
    last_disconnect: Arc<Mutex<Option<Instant>>>,
    reconnect_attempts: Arc<Mutex<u32>>,
    is_connected: Arc<AtomicBool>,
    monitoring_active: Arc<AtomicBool>,
}

impl WifiReconnectManager {
    pub fn new(ssid: String, password: String) -> Self {
        Self {
            ssid,
            password,
            last_disconnect: Arc::new(Mutex::new(None)),
            reconnect_attempts: Arc::new(Mutex::new(0)),
            is_connected: Arc::new(AtomicBool::new(true)), // Assume connected initially
            monitoring_active: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Start monitoring WiFi connection and handle reconnections
    pub fn start_monitoring(&self) -> Result<()> {
        // Only start if not already monitoring
        if self.monitoring_active.swap(true, Ordering::SeqCst) {
            log::warn!("WiFi monitoring already active");
            return Ok(());
        }
        
        // Clone these in case we need them for future reconnection improvements
        let _ssid = self.ssid.clone();
        let _password = self.password.clone();
        let is_connected = self.is_connected.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let last_disconnect = self.last_disconnect.clone();
        let monitoring_active = self.monitoring_active.clone();
        
        // Spawn monitoring task
        std::thread::spawn(move || {
            log::info!("WiFi monitoring task started");
            
            // Initial delay to avoid race condition during boot
            log::info!("Waiting 15 seconds before starting WiFi monitoring...");
            std::thread::sleep(Duration::from_secs(15));
            
            while monitoring_active.load(Ordering::Relaxed) {
                // Check if WiFi is connected
                let connected = unsafe {
                    let mut ap_info: esp_idf_sys::wifi_ap_record_t = std::mem::zeroed();
                    esp_idf_sys::esp_wifi_sta_get_ap_info(&mut ap_info) == esp_idf_sys::ESP_OK
                };
                
                let was_connected = is_connected.load(Ordering::Relaxed);
                is_connected.store(connected, Ordering::Relaxed);
                
                if was_connected && !connected {
                    // Just disconnected
                    log::warn!("WiFi disconnected! Starting reconnection process...");
                    if let Ok(mut ld) = last_disconnect.lock() { *ld = Some(Instant::now()); }
                    if let Ok(mut ra) = reconnect_attempts.lock() { *ra = 0; }
                }
                
                if !connected {
                    // Try to reconnect
                    let attempts = {
                        if let Ok(mut guard) = reconnect_attempts.lock() {
                            *guard += 1;
                            *guard
                        } else {
                            1
                        }
                    };
                    
                    log::warn!("WiFi disconnected: attempt #{} (will not mask persistent issues)", attempts);
                    
                    // Calculate backoff delay (exponential, max 60 seconds)
                    let delay = std::cmp::min(60, 5 * (1 << std::cmp::min(attempts - 1, 4)));
                    log::info!("Backoff {}s before reconnection attempt", delay);
                    std::thread::sleep(Duration::from_secs(delay as u64));
                    
                    // Attempt reconnection
                    match Self::force_reconnect() {
                        Ok(_) => {
                            log::info!("WiFi reconnection initiated");
                            // Wait a bit for connection to establish
                            std::thread::sleep(Duration::from_secs(10));
                        }
                        Err(e) => {
                            log::error!("WiFi reconnection failed: {:?}", e);
                        }
                    }
                } else {
                    // Connected - reset attempts counter
                    let attempts = reconnect_attempts.lock().map(|g| *g).unwrap_or(0);
                    if attempts > 0 {
                        log::warn!("WiFi reconnected after {} attempts (intermittent network)", attempts);
                        if let Ok(mut ra) = reconnect_attempts.lock() { *ra = 0; }
                    }
                }
                
                // Check every 10 seconds
                std::thread::sleep(Duration::from_secs(10));
            }
            
            log::info!("WiFi monitoring task stopped");
        });
        
        log::info!("WiFi auto-reconnection monitoring started");
        Ok(())
    }
    
    /// Register WiFi event handlers for automatic reconnection
    pub fn register_event_handlers(&self, _sysloop: &EspEventLoop<System>) -> Result<()> {
        // Start the monitoring task instead
        self.start_monitoring()
    }
    
    /// Stop monitoring
    #[allow(dead_code)] // Will be used for graceful shutdown
    pub fn stop_monitoring(&self) {
        self.monitoring_active.store(false, Ordering::SeqCst);
        log::info!("WiFi monitoring stopped");
    }
    
    /// Check if currently connected
    #[allow(dead_code)] // Useful for status checks
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }
    
    /// Force a WiFi reconnection (useful after OTA)
    pub fn force_reconnect() -> Result<()> {
        log::info!("Forcing WiFi reconnection...");
        
        unsafe {
            // Check WiFi state to avoid race condition
            let mut mode: esp_idf_sys::wifi_mode_t = 0;
            esp_idf_sys::esp_wifi_get_mode(&mut mode);
            
            // If WiFi is not in STA mode, skip reconnection
            if mode != esp_idf_sys::wifi_mode_t_WIFI_MODE_STA && 
               mode != esp_idf_sys::wifi_mode_t_WIFI_MODE_APSTA {
                log::warn!("WiFi not in STA mode, skipping reconnection");
                return Ok(());
            }
            
            // Check if already connected
            let mut ap_info: esp_idf_sys::wifi_ap_record_t = std::mem::zeroed();
            if esp_idf_sys::esp_wifi_sta_get_ap_info(&mut ap_info) == esp_idf_sys::ESP_OK {
                log::info!("WiFi already connected, skipping reconnection");
                return Ok(());
            }
            
            // Disconnect first (ignore error if not connected)
            let _ = esp_idf_sys::esp_wifi_disconnect();
            
            // Wait a bit
            std::thread::sleep(Duration::from_millis(500));
            
            // Reconnect
            let result = esp_idf_sys::esp_wifi_connect();
            if result == esp_idf_sys::ESP_OK {
                log::info!("WiFi reconnection initiated");
                Ok(())
            } else if result == esp_idf_sys::ESP_ERR_WIFI_CONN as i32 {
                // Already connecting - not an error
                log::info!("WiFi already connecting");
                Ok(())
            } else {
                bail!("Failed to initiate WiFi reconnection: {} (0x{:x})", result, result)
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