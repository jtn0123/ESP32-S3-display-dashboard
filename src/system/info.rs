use std::time::{Duration, Instant};

pub struct SystemInfo {
    boot_time: Instant,
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            boot_time: Instant::now(),
        }
    }
    
    pub fn get_uptime(&self) -> Duration {
        self.boot_time.elapsed()
    }
    
    pub fn format_uptime(&self) -> String {
        let uptime = self.get_uptime();
        let total_secs = uptime.as_secs();
        
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        
        if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes:02}:{seconds:02}")
        }
    }
    
    pub fn get_free_heap_kb(&self) -> u32 {
        unsafe { esp_idf_sys::esp_get_free_heap_size() / 1024 }
    }
    
    pub fn get_flash_info(&self) -> (u32, u32) {
        // Get flash chip size and app size
        unsafe {
            use esp_idf_sys::*;
            
            // Get flash chip size
            let mut chip_size = 0u32;
            esp_flash_get_size(std::ptr::null_mut(), &mut chip_size);
            let chip_size_mb = chip_size / (1024 * 1024);
            
            // Get app partition size
            let partition = esp_partition_find_first(
                esp_partition_type_t_ESP_PARTITION_TYPE_APP,
                esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_APP_FACTORY,
                std::ptr::null()
            );
            
            let app_size_mb = if !partition.is_null() {
                (*partition).size / (1024 * 1024)
            } else {
                0
            };
            
            (chip_size_mb, app_size_mb)
        }
    }
    
    pub fn get_cpu_freq_mhz(&self) -> u32 {
        // Return default CPU frequency for ESP32-S3
        240
    }
}