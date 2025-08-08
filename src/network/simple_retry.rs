use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::config::Config;
use crate::ota::OtaManager;
use super::web_server::WebConfigServer;

/// Simple retry wrapper that can be dropped into existing code
#[allow(dead_code)]
pub fn try_start_web_server_with_retries(
    config: Arc<Mutex<Config>>,
    ota_manager: Option<Arc<Mutex<OtaManager>>>,
    network_connected: bool,
    max_wait_seconds: u32,
) -> Option<WebConfigServer> {
    if !network_connected {
        log::info!("Network not connected, waiting up to {} seconds...", max_wait_seconds);
        
        // Wait for network with periodic checks
        for i in 0..max_wait_seconds {
            thread::sleep(Duration::from_secs(1));
            
            // Check if we can get an IP (simple network check)
            let has_ip = unsafe {
                let mut ip_info: esp_idf_sys::esp_netif_ip_info_t = std::mem::zeroed();
                let netif = esp_idf_sys::esp_netif_get_handle_from_ifkey(b"WIFI_STA_DEF\0".as_ptr() as *const ::core::ffi::c_char);
                if !netif.is_null() {
                    esp_idf_sys::esp_netif_get_ip_info(netif, &mut ip_info) == esp_idf_sys::ESP_OK
                        && ip_info.ip.addr != 0
                } else {
                    false
                }
            };
            
            if has_ip {
                log::info!("Network connected after {} seconds!", i + 1);
                break;
            }
            
            if (i + 1) % 5 == 0 {
                log::info!("Still waiting for network... ({}/{})", i + 1, max_wait_seconds);
            }
        }
    }
    
    // Try to start web server with retries
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 2;
    
    for attempt in 1..=MAX_RETRIES {
        log::info!("Starting web server (attempt {}/{})", attempt, MAX_RETRIES);
        
        match WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {
            Ok(server) => {
                log::info!("Web server started successfully on attempt {}", attempt);
                return Some(server);
            }
            Err(e) => {
                log::error!("Web server start failed (attempt {}): {:?}", attempt, e);
                
                if attempt < MAX_RETRIES {
                    log::info!("Retrying in {} seconds...", RETRY_DELAY_SECS);
                    thread::sleep(Duration::from_secs(RETRY_DELAY_SECS));
                }
            }
        }
    }
    
    log::error!("Failed to start web server after {} attempts", MAX_RETRIES);
    None
}

/// Start web server in background if network becomes available later
#[allow(dead_code)]
pub fn spawn_web_server_retry_task(
    config: Arc<Mutex<Config>>,
    ota_manager: Option<Arc<Mutex<OtaManager>>>,
) {
    thread::spawn(move || {
        log::info!("Web server background retry task started");
        
        // Wait a bit for system to stabilize
        thread::sleep(Duration::from_secs(10));
        
        // Try every 30 seconds for 5 minutes
        for attempt in 1..=10 {
            log::info!("Background web server start attempt {}", attempt);
            
            // Check if network is available
            let has_ip = unsafe {
                let mut ip_info: esp_idf_sys::esp_netif_ip_info_t = std::mem::zeroed();
                let netif = esp_idf_sys::esp_netif_get_handle_from_ifkey(b"WIFI_STA_DEF\0".as_ptr() as *const ::core::ffi::c_char);
                if !netif.is_null() {
                    esp_idf_sys::esp_netif_get_ip_info(netif, &mut ip_info) == esp_idf_sys::ESP_OK
                        && ip_info.ip.addr != 0
                } else {
                    false
                }
            };
            
            if has_ip {
                log::info!("Network available, attempting to start web server...");
                
                match WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {
                    Ok(server) => {
                        log::info!("Web server started successfully from background task!");
                        // Keep server alive
                        std::mem::forget(server);
                        return;
                    }
                    Err(e) => {
                        log::error!("Background web server start failed: {:?}", e);
                    }
                }
            }
            
            thread::sleep(Duration::from_secs(30));
        }
        
        log::error!("Background web server retry task gave up after 5 minutes");
    });
}