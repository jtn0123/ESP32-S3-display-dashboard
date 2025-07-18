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

pub struct WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
    pub ssid: String,
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
        let cfg = Configuration::Client(ClientConfiguration {
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
        })
    }

    #[allow(dead_code)]
    pub fn connect(&mut self) -> Result<()> {
        log::info!("Starting WiFi...");
        self.wifi.start()?;

        log::info!("Scanning for networks...");
        
        // Reset watchdog before scan (scan can take 4+ seconds)
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        
        let ap_infos = self.wifi.scan()?;
        
        // Reset watchdog after scan
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        
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
        
        // Reset watchdog before potentially long DHCP wait
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        
        self.wifi.wait_netif_up()?;
        
        // Reset watchdog after DHCP complete
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }

        log::info!("WiFi connected!");
        
        // Enable WiFi power save mode
        unsafe {
            use esp_idf_sys::*;
            let result = esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_MIN_MODEM);
            if result == ESP_OK {
                log::info!("WiFi power save enabled (MIN_MODEM mode)");
            } else {
                log::warn!("Failed to enable WiFi power save: {:?}", result);
            }
        }
        
        Ok(())
    }

    // disconnect and is_connected removed - not used

    pub fn get_ip(&self) -> Option<String> {
        self.wifi.wifi().sta_netif().get_ip_info().ok()
            .map(|ip_info| format!("{}", ip_info.ip))
    }
}