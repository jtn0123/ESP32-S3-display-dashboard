use anyhow::Result;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use std::time::{Duration, Instant};

const NVS_NAMESPACE: &str = "uptime";
const NVS_KEY_TOTAL: &str = "total_secs";
const NVS_KEY_BOOTS: &str = "boot_count";
const SAVE_INTERVAL: Duration = Duration::from_secs(60); // Save every minute

pub struct UptimeTracker {
    nvs: Option<EspNvs<NvsDefault>>,
    boot_time: Instant,
    total_uptime_at_boot: u64,
    boot_count: u32,
    last_save: Instant,
}

impl UptimeTracker {
    pub fn new() -> Result<Self> {
        // Ensure NVS is initialized and recover if needed
        unsafe {
            let init_res = esp_idf_sys::nvs_flash_init();
            if init_res == esp_idf_sys::ESP_ERR_NVS_NO_FREE_PAGES
                || init_res == esp_idf_sys::ESP_ERR_NVS_NEW_VERSION_FOUND
            {
                let _ = esp_idf_sys::nvs_flash_erase();
                let _ = esp_idf_sys::nvs_flash_init();
            }
        }

        // Try to take default NVS partition first
        let nvs_result = esp_idf_svc::nvs::EspNvsPartition::<NvsDefault>::take()
            .or_else(|e| {
                log::warn!("EspNvsPartition::take failed: {:?}; trying default partition", e);
                esp_idf_svc::nvs::EspDefaultNvsPartition::take()
                    .map(|p| p.into())
            });
        
        let (nvs, total_uptime, boot_count) = match nvs_result {
            Ok(nvs_partition) => {
                match EspNvs::new(nvs_partition, NVS_NAMESPACE, true) {
                    Ok(nvs) => {
                        // Read previous values
                        let total = nvs.get_u64(NVS_KEY_TOTAL).ok().flatten().unwrap_or(0);
                        let boots = nvs.get_u32(NVS_KEY_BOOTS).ok().flatten().unwrap_or(0);
                        
                        // Increment boot count
                        let new_boots = boots + 1;
                        let _ = nvs.set_u32(NVS_KEY_BOOTS, new_boots);
                        
                        log::info!("Uptime tracker initialized - Total: {} hours, Boots: {}", 
                                  total / 3600, new_boots);
                        
                        (Some(nvs), total, new_boots)
                    }
                    Err(e) => {
                        log::warn!("Failed to open NVS namespace: {:?}", e);
                        (None, 0, 0)
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to initialize NVS partition: {:?}", e);
                (None, 0, 0)
            }
        };
        
        Ok(Self {
            nvs,
            boot_time: Instant::now(),
            total_uptime_at_boot: total_uptime,
            boot_count,
            last_save: Instant::now(),
        })
    }
    
    /// Get current session uptime
    pub fn get_session_uptime(&self) -> Duration {
        self.boot_time.elapsed()
    }
    
    /// Get total device uptime (including previous sessions)
    pub fn get_total_uptime(&self) -> Duration {
        let session = self.get_session_uptime();
        Duration::from_secs(self.total_uptime_at_boot + session.as_secs())
    }
    
    /// Get boot count
    pub fn get_boot_count(&self) -> u32 {
        self.boot_count
    }
    
    /// Save current uptime to NVS (call periodically)
    pub fn save_if_needed(&mut self) -> Result<()> {
        if self.last_save.elapsed() < SAVE_INTERVAL {
            return Ok(());
        }
        
        // Calculate total uptime before borrowing nvs
        let total_secs = self.get_total_uptime().as_secs();
        
        if let Some(ref mut nvs) = self.nvs {
            match nvs.set_u64(NVS_KEY_TOTAL, total_secs) {
                Ok(_) => {
                    self.last_save = Instant::now();
                    log::debug!("Saved uptime: {} hours", total_secs / 3600);
                }
                Err(e) => {
                    log::warn!("Failed to save uptime: {:?}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Format session uptime
    #[allow(dead_code)] // Will be used for UI display
    pub fn format_session_uptime(&self) -> String {
        format_duration(self.get_session_uptime())
    }
    
    /// Format total uptime
    pub fn format_total_uptime(&self) -> String {
        format_duration(self.get_total_uptime())
    }
    
    /// Get uptime statistics
    #[allow(dead_code)] // Used by telnet commands in the future
    pub fn get_stats(&self) -> UptimeStats {
        UptimeStats {
            session_uptime: self.get_session_uptime(),
            total_uptime: self.get_total_uptime(),
            boot_count: self.boot_count,
            average_uptime: if self.boot_count > 0 {
                Duration::from_secs(self.get_total_uptime().as_secs() / self.boot_count as u64)
            } else {
                Duration::from_secs(0)
            },
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)] // Will be used for telnet commands
pub struct UptimeStats {
    pub session_uptime: Duration,
    pub total_uptime: Duration,
    pub boot_count: u32,
    pub average_uptime: Duration,
}

/// Format a duration into a human-readable string
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1h 2m 5s");
        assert_eq!(format_duration(Duration::from_secs(90125)), "1d 1h 2m");
    }
}