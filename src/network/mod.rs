pub mod wifi;
pub mod web_server;

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
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        self.wifi.connect()?;
        log::info!("WiFi connected, IP: {:?}", self.wifi.get_ip());
        
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
        &self.wifi.ssid
    }
}