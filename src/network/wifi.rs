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
        log::info!("Initializing WiFi manager for SSID: '{}'", ssid);
        
        // Check if credentials are empty
        if ssid.is_empty() {
            log::error!("WiFi SSID is empty! Check wifi_config.h");
            bail!("WiFi SSID cannot be empty");
        }
        
        let nvs = EspDefaultNvsPartition::take()?;
        let mut esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;

        // Configure WiFi
        let cfg = Configuration::Client(ClientConfiguration {
            ssid: ssid.as_str().try_into()
                .map_err(|e| {
                    log::error!("Failed to convert SSID '{}': {:?}", ssid, e);
                    anyhow::anyhow!("Invalid SSID format: {}", ssid)
                })?,
            password: password.as_str().try_into()
                .map_err(|e| {
                    log::error!("Failed to convert password: {:?}", e);
                    anyhow::anyhow!("Invalid password format")
                })?,
            auth_method: if password.is_empty() {
                log::warn!("WiFi password is empty, using open network");
                AuthMethod::None
            } else {
                log::info!("Using WPA2 authentication");
                AuthMethod::WPA2Personal
            },
            ..Default::default()
        });

        log::info!("Setting WiFi configuration...");
        esp_wifi.set_configuration(&cfg)?;
        
        let wifi = BlockingWifi::wrap(esp_wifi, sys_loop)?;

        log::info!("WiFi manager initialized successfully");
        Ok(Self {
            wifi,
            ssid,
            last_signal_strength: -100,
        })
    }

    pub fn connect_and_get_signal(&mut self) -> Result<i8> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u32 = 5000;
        
        log::info!("Starting WiFi connection process for SSID: '{}'", self.ssid);
        
        for attempt in 1..=MAX_RETRIES {
            log::info!("WiFi connection attempt {} of {}", attempt, MAX_RETRIES);
            
            match self.try_connect() {
                Ok(signal) => {
                    log::info!("WiFi connected successfully on attempt {}", attempt);
                    log::info!("Signal strength: {} dBm", signal);
                    return Ok(signal);
                }
                Err(e) => {
                    log::warn!("WiFi connection attempt {} failed: {:?}", attempt, e);
                    
                    if attempt < MAX_RETRIES {
                        log::info!("Waiting {}ms before retry...", RETRY_DELAY_MS);
                        esp_idf_hal::delay::FreeRtos::delay_ms(RETRY_DELAY_MS);
                        
                        // Try to restart WiFi for next attempt
                        log::info!("Stopping WiFi for clean retry...");
                        let _ = self.wifi.stop();
                        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
                    }
                }
            }
        }
        
        log::error!("Failed to connect to WiFi '{}' after {} attempts", self.ssid, MAX_RETRIES);
        bail!("Failed to connect to WiFi after {} attempts", MAX_RETRIES)
    }
    
    fn try_connect(&mut self) -> Result<i8> {
        log::info!("Starting WiFi...");
        self.wifi.start()?;
        
        // Disable power save mode immediately after starting WiFi
        // This helps prevent disconnections during initial setup
        unsafe {
            use esp_idf_sys::*;
            let result = esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE);
            if result == ESP_OK {
                log::info!("WiFi power save disabled at startup");
            }
        }

        log::info!("Scanning for networks...");
        
        // Temporarily remove current task from watchdog monitoring during WiFi scan
        unsafe {
            let result = esp_idf_sys::esp_task_wdt_delete(std::ptr::null_mut());
            if result == esp_idf_sys::ESP_OK {
                log::info!("Temporarily disabled watchdog for WiFi scan");
            }
        }
        
        // Perform the scan (this can take 3-5 seconds)
        let ap_infos = match self.wifi.scan() {
            Ok(aps) => aps,
            Err(e) => {
                // Re-add task to watchdog monitoring before returning error
                unsafe {
                    esp_idf_sys::esp_task_wdt_add(std::ptr::null_mut());
                    esp_idf_sys::esp_task_wdt_reset();
                }
                return Err(anyhow::anyhow!("WiFi scan failed: {:?}", e));
            }
        };
        
        // Re-add task to watchdog monitoring
        unsafe {
            let result = esp_idf_sys::esp_task_wdt_add(std::ptr::null_mut());
            if result == esp_idf_sys::ESP_OK {
                log::info!("Re-enabled watchdog after WiFi scan");
                // Reset immediately to start fresh
                esp_idf_sys::esp_task_wdt_reset();
            }
        };
        
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
        
        // Set a timeout for connection
        match self.wifi.connect() {
            Ok(_) => {
                log::info!("Connect command sent successfully");
            }
            Err(e) => {
                log::error!("Connect command failed: {:?}", e);
                return Err(anyhow::anyhow!("Failed to initiate connection: {:?}", e));
            }
        }

        log::info!("Waiting for DHCP...");
        
        // Reset watchdog before potentially long DHCP wait
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        
        // Wait for network interface with timeout
        match self.wifi.wait_netif_up() {
            Ok(_) => {
                log::info!("Network interface is up");
            }
            Err(e) => {
                log::error!("Failed to get IP address: {:?}", e);
                return Err(anyhow::anyhow!("DHCP failed: {:?}", e));
            }
        }
        
        // Reset watchdog after DHCP complete
        unsafe { esp_idf_sys::esp_task_wdt_reset(); }

        log::info!("WiFi connected!");
        
        // Store signal strength
        self.last_signal_strength = signal_strength;
        
        // Disable WiFi power save mode to prevent disconnections
        // MIN_MODEM mode can cause disconnections during web server activity
        // The ESP32 may disconnect with error code 0x6374c0 when power save is active
        // and there's significant network traffic (web requests, telnet, etc.)
        // IMPORTANT: We must wait a bit for WiFi to stabilize before disabling power save
        esp_idf_hal::delay::FreeRtos::delay_ms(500);
        
        unsafe {
            use esp_idf_sys::*;
            let result = esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE);
            if result == ESP_OK {
                log::info!("WiFi power save disabled for stable connection");
            } else {
                log::warn!("Failed to set WiFi power save mode: {:?}", result);
            }
        }
        
        // Give WiFi more time to stabilize with power save disabled
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        log::info!("WiFi connection stabilized");
        
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
}