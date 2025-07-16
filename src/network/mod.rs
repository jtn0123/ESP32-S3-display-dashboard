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

pub struct NetworkManager {
    wifi: WifiManager,
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
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        self.wifi.connect()?;
        log::info!("WiFi connected, IP: {:?}", self.wifi.get_ip());
        Ok(())
    }

    // run_ota_checker, is_connected, get_ip removed - not used
}