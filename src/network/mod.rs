pub mod wifi;
pub mod wifi_reconnect;
pub mod web_server;
pub mod simple_retry;
pub mod telnet_server;
pub mod sse_broadcaster;
pub mod api_routes;
pub mod error_handler;
pub mod error_wrapper;
pub mod validators;
pub mod log_streamer;
pub mod file_manager;
pub mod compression;
pub mod binary_protocol;
pub mod http_config;
pub mod streaming_home;
pub mod streaming_ota;
pub mod streaming_dashboard;
pub mod template_engine;
pub mod templated_home;

use anyhow::Result;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    timer::EspTaskTimerService,
};
use std::sync::{Arc, Mutex};

use self::wifi::WifiManager;
use self::wifi_reconnect::WifiReconnectManager;
use crate::config::Config;
use esp_idf_svc::mdns::EspMdns;

pub struct NetworkManager {
    wifi: WifiManager,
    _mdns: Option<EspMdns>,
    signal_strength: i8,
    _reconnect_manager: Option<Arc<WifiReconnectManager>>,
    disconnect_count: Arc<Mutex<u32>>,
    reconnect_count: Arc<Mutex<u32>>,
}

impl NetworkManager {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        _timer_service: EspTaskTimerService,
        ssid: String,
        password: String,
        _config: Arc<Mutex<Config>>,
    ) -> Result<Self> {
        let wifi = WifiManager::new(modem, sys_loop.clone(), ssid.clone(), password.clone())?;
        
        // Create reconnection manager
        let reconnect_manager = Arc::new(WifiReconnectManager::new(ssid, password));
        reconnect_manager.register_event_handlers(&sys_loop)?;

        Ok(Self {
            wifi,
            _mdns: None,
            signal_strength: -100,
            _reconnect_manager: Some(reconnect_manager),
            disconnect_count: Arc::new(Mutex::new(0)),
            reconnect_count: Arc::new(Mutex::new(0)),
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        // Get signal strength during connection
        self.signal_strength = self.wifi.connect_and_get_signal()?;
        log::info!("WiFi connected, IP: {:?}, Signal: {} dBm", self.wifi.get_ip(), self.signal_strength);
        
        // Start mDNS for network discovery
        match self.start_mdns() {
            Ok(_) => log::info!("mDNS service started: esp32.local"),
            Err(e) => log::warn!("Failed to start mDNS: {:?}", e),
        }
        
        Ok(())
    }
    
    fn start_mdns(&mut self) -> Result<()> {
        // Try to take mDNS, but it might already be taken
        let mdns_result = EspMdns::take();
        let mut mdns = match mdns_result {
            Ok(m) => m,
            Err(_) => {
                log::warn!("mDNS already initialized, skipping");
                return Ok(());
            }
        };
        mdns.set_hostname("esp32")?;
        
        // Properties are set via service text records in esp-idf-svc
        
        // Add service for OTA discovery
        mdns.add_service(None, "_esp32-ota", "_tcp", 80, &[
            ("path", "/ota"),
            ("version", crate::version::DISPLAY_VERSION),
        ])?;
        
        // Add service for web config
        mdns.add_service(None, "_http", "_tcp", 80, &[
            ("path", "/"),
        ])?;
        
        // Add service for telnet logging
        mdns.add_service(None, "_telnet", "_tcp", 23, &[
            ("type", "log-streaming"),
        ])?;
        
        self._mdns = Some(mdns);
        Ok(())
    }
    
    
    pub fn is_connected(&self) -> bool {
        self.wifi.get_ip().is_some()
    }
    
    pub fn get_ip(&self) -> Option<String> {
        self.wifi.get_ip()
    }
    
    pub fn get_ssid(&self) -> &str {
        if self.wifi.ssid.is_empty() {
            "Not configured"
        } else {
            &self.wifi.ssid
        }
    }
    
    pub fn get_signal_strength(&self) -> i8 {
        if self.is_connected() {
            self.signal_strength
        } else {
            -100
        }
    }
    
    pub fn get_gateway(&self) -> Option<String> {
        self.wifi.get_gateway()
    }
    
    pub fn get_mac(&self) -> String {
        self.wifi.get_mac()
    }
    
    
    /// Get connection stats
    pub fn get_connection_stats(&self) -> (u32, u32) {
        let disconnects = self.disconnect_count.lock().map(|c| *c).unwrap_or(0);
        let reconnects = self.reconnect_count.lock().map(|c| *c).unwrap_or(0);
        (disconnects, reconnects)
    }
}