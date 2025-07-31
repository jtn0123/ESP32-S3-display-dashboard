// Network monitoring task for Core 1
// Continuously monitors WiFi signal strength, connection state, and network health

use std::sync::mpsc::Sender;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct NetworkUpdate {
    // Currently no fields used - placeholder for future network metrics
}

pub struct NetworkMonitor {
    _tx: Sender<NetworkUpdate>,
}

impl NetworkMonitor {
    
    pub fn new_with_channel(tx: Sender<NetworkUpdate>) -> Self {
        Self {
            _tx: tx,
        }
    }
    
    pub fn update(&mut self) -> Result<()> {
        // Network monitoring disabled - rssi data not currently used
        // Keeping structure in place for future network metrics
        Ok(())
    }
    
}