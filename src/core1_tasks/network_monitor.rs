// Network monitoring task for Core 1
// Continuously monitors WiFi signal strength, connection state, and network health

use std::sync::mpsc::Sender;
use anyhow::Result;
use esp_idf_sys::*;

#[derive(Debug, Clone)]
pub struct NetworkUpdate {
    pub rssi: i8,
    pub is_connected: bool,
    pub ip_address: Option<String>,
    pub reconnect_count: u32,
    pub packets_sent: u64,
    pub packets_received: u64,
}

pub struct NetworkMonitor {
    tx: Sender<NetworkUpdate>,
    reconnect_count: u32,
    last_rssi_values: Vec<i8>,
    rssi_index: usize,
}

impl NetworkMonitor {
    pub fn new() -> Result<Self> {
        Ok(Self::new_with_channel(std::sync::mpsc::channel().0))
    }
    
    pub fn new_with_channel(tx: Sender<NetworkUpdate>) -> Self {
        Self {
            tx,
            reconnect_count: 0,
            last_rssi_values: vec![-90; 3],  // 3-sample moving average
            rssi_index: 0,
        }
    }
    
    pub fn update(&mut self) -> Result<()> {
        // Get WiFi status
        let (is_connected, rssi, ip_address) = self.get_wifi_status();
        
        // Apply moving average to RSSI
        if is_connected && rssi != 0 {
            self.last_rssi_values[self.rssi_index] = rssi;
            self.rssi_index = (self.rssi_index + 1) % self.last_rssi_values.len();
        }
        let avg_rssi = self.last_rssi_values.iter().sum::<i8>() / self.last_rssi_values.len() as i8;
        
        // Get network statistics
        let (packets_sent, packets_received) = self.get_network_stats();
        
        // Send update
        let update = NetworkUpdate {
            rssi: avg_rssi,
            is_connected,
            ip_address,
            reconnect_count: self.reconnect_count,
            packets_sent,
            packets_received,
        };
        
        // Send update (will block if channel is full)
        if let Err(e) = self.tx.send(update) {
            log::error!("Failed to send network update: {}", e);
        }
        
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
    
    fn get_network_stats(&self) -> (u64, u64) {
        // TODO: Implement real network statistics
        // For now, return placeholder values
        (0, 0)
    }
    
    pub fn notify_reconnect(&mut self) {
        self.reconnect_count += 1;
    }
}