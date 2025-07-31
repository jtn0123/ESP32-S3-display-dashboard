pub mod wifi;
pub mod web_server;
pub mod telnet_server;

use anyhow::Result;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    timer::EspTaskTimerService,
};
use std::sync::{Arc, Mutex};

use self::wifi::WifiManager;
use crate::config::Config;
use esp_idf_svc::mdns::EspMdns;

pub struct NetworkManager {
    wifi: WifiManager,
    _mdns: Option<EspMdns>,
    signal_strength: i8,
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
        let wifi = WifiManager::new(modem, sys_loop, ssid, password)?;

        Ok(Self {
            wifi,
            _mdns: None,
            signal_strength: -100,
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        // Get signal strength during connection
        self.signal_strength = self.wifi.connect_and_get_signal()?;
        log::info!("WiFi connected, IP: {:?}, Signal: {} dBm", self.wifi.get_ip(), self.signal_strength);
        
        // Start mDNS for network discovery
        match self.start_mdns() {
            Ok(_) => log::info!("mDNS service started: esp32-dashboard.local"),
            Err(e) => log::warn!("Failed to start mDNS: {:?}", e),
        }
        
        Ok(())
    }
    
    fn start_mdns(&mut self) -> Result<()> {
        let mut mdns = EspMdns::take()?;
        mdns.set_hostname("esp32-dashboard")?;
        
        // Properties are set via service text records in esp-idf-svc
        
        // Add service for OTA discovery
        mdns.add_service(None, "_esp32-ota", "_tcp", 8080, &[
            ("path", "/ota"),
            ("version", "v4.13"),
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
}