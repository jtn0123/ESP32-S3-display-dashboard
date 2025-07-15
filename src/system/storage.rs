use anyhow::Result;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, EspNvsPartition, NvsDefault};
use serde::{Deserialize, Serialize};

pub struct Storage {
    namespace: String,
}

impl Storage {
    pub fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
        }
    }

    pub fn read<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let nvs_partition = EspDefaultNvsPartition::take()?;
        let nvs = EspNvs::new(nvs_partition, &self.namespace, true)?;
        
        let mut buf = vec![0u8; 4096]; // Max size
        match nvs.get_blob(key, &mut buf)? {
            Some(data) => {
                let value: T = serde_json::from_slice(data)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub fn write<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let nvs_partition = EspDefaultNvsPartition::take()?;
        let mut nvs = EspNvs::new(nvs_partition, &self.namespace, false)?;
        
        let data = serde_json::to_vec(value)?;
        nvs.set_blob(key, &data)?;
        
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let nvs_partition = EspDefaultNvsPartition::take()?;
        let mut nvs = EspNvs::new(nvs_partition, &self.namespace, false)?;
        
        nvs.remove(key)?;
        Ok(())
    }

    pub fn clear_namespace(&self) -> Result<()> {
        let nvs_partition = EspDefaultNvsPartition::take()?;
        let _nvs = EspNvs::new(nvs_partition, &self.namespace, false)?;
        
        // Note: ESP-IDF NVS doesn't have a direct clear_all method
        // You would need to iterate through keys or use esp_nvs_erase_all
        log::warn!("Clear namespace not fully implemented");
        
        Ok(())
    }
}

// Specific storage for calibration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationData {
    pub battery_offset: f32,
    pub temperature_offset: f32,
    pub touch_calibration: Option<TouchCalibration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchCalibration {
    pub x_min: u16,
    pub x_max: u16,
    pub y_min: u16,
    pub y_max: u16,
}

impl Default for CalibrationData {
    fn default() -> Self {
        Self {
            battery_offset: 0.0,
            temperature_offset: 0.0,
            touch_calibration: None,
        }
    }
}

// Usage statistics storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub boot_count: u32,
    pub total_runtime_secs: u64,
    pub last_boot_time: u64,
    pub crashes: u32,
    pub ota_updates: u32,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            boot_count: 0,
            total_runtime_secs: 0,
            last_boot_time: 0,
            crashes: 0,
            ota_updates: 0,
        }
    }
}

impl UsageStats {
    pub fn increment_boot(&mut self) {
        self.boot_count += 1;
        self.last_boot_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn add_runtime(&mut self, secs: u64) {
        self.total_runtime_secs += secs;
    }
}