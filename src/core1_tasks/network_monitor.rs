// Network monitoring task for Core 1
// Continuously monitors WiFi signal strength, connection state, and network health

use std::sync::mpsc::Sender;
use anyhow::Result;
use esp_idf_sys::*;

#[derive(Debug, Clone)]
pub struct NetworkUpdate {
    pub rssi: i8,
}

pub struct NetworkMonitor {
    tx: Sender<NetworkUpdate>,
    last_rssi_values: Vec<i8>,
    rssi_index: usize,
}

impl NetworkMonitor {
    
    pub fn new_with_channel(tx: Sender<NetworkUpdate>) -> Self {
        Self {
            tx,
            last_rssi_values: vec![-90; 3],  // 3-sample moving average
            rssi_index: 0,
        }
    }
    
    pub fn update(&mut self) -> Result<()> {
        // Network monitoring disabled - rssi data not currently used
        // Keeping structure in place for future network metrics
        Ok(())
    }
    
    fn get_wifi_status(&self) -> (bool, i8, Option<String>) {
        unsafe {
            let mut rssi: i32 = 0;
            let ret = esp_wifi_sta_get_rssi(&mut rssi as *mut i32);
            
            if ret == 0 {  // ESP_OK
                // For now, just return connected status without IP
                // TODO: Update to use esp-idf-svc NetIF API for IP info
                (true, rssi as i8, None)
            } else {
                (false, 0, None)
            }
        }
    }
    
    
}