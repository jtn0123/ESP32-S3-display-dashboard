use anyhow::{Result, bail};
use esp_idf_svc::ota::{EspOta, EspOtaUpdate};
use embedded_svc::http::client::{Client, Request, Method};
use esp_idf_svc::http::client::{EspHttpConnection, Configuration as HttpConfig};
use serde::Deserialize;

const OTA_URL: &str = "http://your-ota-server.com/firmware";
const VERSION_URL: &str = "http://your-ota-server.com/version.json";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct VersionInfo {
    version: String,
    url: String,
    size: usize,
    checksum: String,
}

pub struct OtaManager {
    ota: EspOta,
}

impl OtaManager {
    pub fn new() -> Result<Self> {
        let ota = EspOta::new()?;
        Ok(Self { ota })
    }

    pub fn check_for_updates(&self) -> Result<()> {
        log::info!("Checking for OTA updates...");
        
        // Check version
        let version_info = self.fetch_version_info()?;
        
        if version_info.version == CURRENT_VERSION {
            log::info!("Already running latest version: {}", CURRENT_VERSION);
            return Ok(());
        }

        log::info!("New version available: {} (current: {})", 
            version_info.version, CURRENT_VERSION);
        
        // Perform update
        self.perform_update(&version_info)?;
        
        Ok(())
    }

    fn fetch_version_info(&self) -> Result<VersionInfo> {
        let config = HttpConfig {
            buffer_size: Some(4096),
            timeout: Some(std::time::Duration::from_secs(30)),
            ..Default::default()
        };
        
        let mut client = Client::wrap(EspHttpConnection::new(&config)?);
        let request = client.request(Method::Get, VERSION_URL, &[])?;
        let mut response = request.submit()?;
        
        if response.status() != 200 {
            bail!("Failed to fetch version info: HTTP {}", response.status());
        }
        
        let mut body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let bytes_read = response.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            body.extend_from_slice(&buf[..bytes_read]);
        }
        
        let version_info: VersionInfo = serde_json::from_slice(&body)?;
        Ok(version_info)
    }

    fn perform_update(&self, version_info: &VersionInfo) -> Result<()> {
        log::info!("Starting OTA update...");
        
        // Start OTA update
        let mut ota_update = self.ota.initiate_update()?;
        
        // Download and write firmware
        let config = HttpConfig {
            buffer_size: Some(4096),
            timeout: Some(std::time::Duration::from_secs(60)),
            ..Default::default()
        };
        
        let mut client = Client::wrap(EspHttpConnection::new(&config)?);
        let request = client.request(Method::Get, &version_info.url, &[])?;
        let mut response = request.submit()?;
        
        if response.status() != 200 {
            bail!("Failed to download firmware: HTTP {}", response.status());
        }
        
        let mut total_bytes = 0;
        let mut buf = [0u8; 4096];
        
        loop {
            let bytes_read = response.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            
            ota_update.write(&buf[..bytes_read])?;
            total_bytes += bytes_read;
            
            let progress = (total_bytes * 100) / version_info.size;
            log::info!("OTA progress: {}% ({}/{})", progress, total_bytes, version_info.size);
        }
        
        // Complete update
        ota_update.complete()?;
        
        log::info!("OTA update complete! Restarting...");
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // Restart
        unsafe {
            esp_idf_sys::esp_restart();
        }
    }

    pub fn get_running_partition(&self) -> Result<String> {
        let partition = self.ota.get_running_partition()?;
        Ok(format!("{:?}", partition.label()))
    }

    pub fn rollback(&mut self) -> Result<()> {
        log::warn!("Rolling back to previous firmware...");
        self.ota.rollback()?;
        
        // Restart
        unsafe {
            esp_idf_sys::esp_restart();
        }
    }
}