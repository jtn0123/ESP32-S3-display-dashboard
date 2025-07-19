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
    last_signal_strength: i8,
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
            last_signal_strength: -100,
        })
    }

    pub fn connect_and_get_signal(&mut self) -> Result<i8> {
        log::info!("Starting WiFi...");
        self.wifi.start()?;

        log::info!("Scanning for networks...");
        
        // Reset watchdog before scan (scan can take 4+ seconds)
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        
        let ap_infos = self.wifi.scan()?;
        
        // Reset watchdog after scan
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        
        let mut found = false;
        let mut signal_strength = -100i8;
        for ap in ap_infos.iter() {
            if ap.ssid.as_str() == self.ssid.as_str() {
                found = true;
                signal_strength = ap.signal_strength;
                log::info!("Found network: {} (signal: {} dBm)", ap.ssid, signal_strength);
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
        
        // Store signal strength
        self.last_signal_strength = signal_strength;
        
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
        
        Ok(signal_strength)
    }

    // disconnect and is_connected removed - not used

    pub fn get_ip(&self) -> Option<String> {
        self.wifi.wifi().sta_netif().get_ip_info().ok()
            .map(|ip_info| format!("{}", ip_info.ip))
    }
    
    pub fn get_gateway(&self) -> Option<String> {
        // Get the IP and assume gateway is .1 in the same subnet
        self.wifi.wifi().sta_netif().get_ip_info().ok()
            .and_then(|ip_info| {
                let ip_str = format!("{}", ip_info.ip);
                let parts: Vec<&str> = ip_str.split('.').collect();
                if parts.len() == 4 {
                    // Assume gateway is x.x.x.1
                    Some(format!("{}.{}.{}.1", parts[0], parts[1], parts[2]))
                } else {
                    None
                }
            })
    }
    
    pub fn get_mac(&self) -> String {
        self.wifi.wifi().sta_netif().get_mac().ok()
            .map(|mac| format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
                              mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]))
            .unwrap_or_else(|| "Unknown".to_string())
    }
    
    #[allow(dead_code)]
    pub fn get_signal_strength(&self) -> i8 {
        self.last_signal_strength
    }
}