pub mod wifi;
pub mod ota;
pub mod web_server;

use anyhow::Result;
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    timer::EspTaskTimerService,
};
use std::sync::{Arc, Mutex};

use self::wifi::WifiManager;
use self::ota::OtaManager;
use self::web_server::WebConfigServer;
use crate::config::Config;

pub struct NetworkManager {
    wifi: WifiManager,
    ota: OtaManager,
    web_server: Option<WebConfigServer>,
}

impl NetworkManager {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        timer_service: EspTaskTimerService,
        ssid: String,
        password: String,
        config: Arc<Mutex<Config>>,
    ) -> Result<Self> {
        let wifi = WifiManager::new(modem, sys_loop, ssid, password)?;
        let ota = OtaManager::new()?;

        Ok(Self {
            wifi,
            ota,
            web_server: None,
        })
    }

    pub fn run(mut self, config: Arc<Mutex<Config>>) -> Result<()> {
        // Connect to WiFi
        self.wifi.connect()?;
        log::info!("WiFi connected, IP: {:?}", self.wifi.get_ip());

        // Start web server after WiFi is connected
        match WebConfigServer::new(config) {
            Ok(server) => {
                self.web_server = Some(server);
                log::info!("Web configuration server started on port 80");
            }
            Err(e) => {
                log::error!("Failed to start web server: {:?}", e);
            }
        }

        // Start OTA update checker
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600)); // Check hourly
            
            if let Err(e) = self.ota.check_for_updates() {
                log::error!("OTA check failed: {:?}", e);
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected()
    }

    pub fn get_ip(&self) -> Option<String> {
        self.wifi.get_ip()
    }
}