use anyhow::{Result, bail};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    wifi::{
        ClientConfiguration, Configuration, EspWifi,
        AuthMethod, BlockingWifi,
    },
    nvs::EspDefaultNvsPartition,
};
use embedded_svc::wifi::{AccessPointInfo, Wifi};
use std::time::Duration;

pub struct WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
    ssid: String,
    password: String,
}

impl WifiManager {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        ssid: String,
        password: String,
    ) -> Result<Self> {
        let nvs = EspDefaultNvsPartition::take()?;
        let mut esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;

        // Configure WiFi
        let mut cfg = Configuration::Client(ClientConfiguration {
            ssid: ssid.as_str().try_into()
                .map_err(|_| anyhow::anyhow!("Invalid SSID format"))?,
            password: password.as_str().try_into()
                .map_err(|_| anyhow::anyhow!("Invalid password format"))?,
            auth_method: if password.is_empty() {
                AuthMethod::None
            } else {
                AuthMethod::WPA2Personal
            },
            ..Default::default()
        });

        esp_wifi.set_configuration(&cfg)?;
        
        let wifi = BlockingWifi::wrap(esp_wifi, sys_loop)?;

        Ok(Self {
            wifi,
            ssid,
            password,
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        log::info!("Starting WiFi...");
        self.wifi.start()?;

        log::info!("Scanning for networks...");
        let ap_infos = self.wifi.scan()?;
        
        let mut found = false;
        for ap in ap_infos.iter() {
            if ap.ssid.as_str() == self.ssid.as_str() {
                found = true;
                log::info!("Found network: {} (signal: {})", ap.ssid, ap.signal_strength);
                break;
            }
        }

        if !found {
            bail!("Network {} not found", self.ssid);
        }

        log::info!("Connecting to {}...", self.ssid);
        self.wifi.connect()?;

        log::info!("Waiting for DHCP...");
        self.wifi.wait_netif_up()?;

        log::info!("WiFi connected!");
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        self.wifi.disconnect()?;
        self.wifi.stop()?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }

    pub fn get_ip(&self) -> Option<String> {
        self.wifi.wifi().sta_netif().get_ip_info().ok()
            .map(|ip_info| format!("{}", ip_info.ip))
    }

    pub fn reconnect(&mut self) -> Result<()> {
        log::info!("Attempting to reconnect WiFi...");
        
        if self.is_connected() {
            self.disconnect()?;
        }

        std::thread::sleep(Duration::from_secs(2));
        self.connect()
    }
}