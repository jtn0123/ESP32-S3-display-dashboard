// Data processing pipeline for Core 1
// Aggregates sensor and network data, performs filtering, and sends updates to Core 0

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use super::{SensorUpdate, NetworkUpdate};

#[derive(Debug, Clone)]
pub struct ProcessedData {
    pub temperature: f32,
    pub battery_percentage: u8,
    pub battery_voltage: u16,
    pub is_charging: bool,
    pub is_on_usb: bool,
    pub cpu_usage_core0: u8,
    pub cpu_usage_core1: u8,
}


pub struct DataProcessor {
    sensor_rx: Receiver<SensorUpdate>,
    network_rx: Receiver<NetworkUpdate>,
    tx: Sender<ProcessedData>,
    
    // Last known values
    last_sensor: Option<SensorUpdate>,
}

impl DataProcessor {
    
    pub fn new_with_channel(
        sensor_rx: Receiver<SensorUpdate>,
        network_rx: Receiver<NetworkUpdate>,
        tx: Sender<ProcessedData>,
    ) -> Self {
        Self {
            sensor_rx,
            network_rx,
            tx,
            last_sensor: None,
        }
    }
    
    pub fn process(&mut self) {
        // Process all pending sensor updates
        loop {
            match self.sensor_rx.try_recv() {
                Ok(update) => {
                    self.last_sensor = Some(update);
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::error!("Sensor channel disconnected");
                    return;
                }
            }
        }
        
        // Drain network updates (not currently used)
        while let Ok(_) = self.network_rx.try_recv() {
            // Network data not currently used in ProcessedData
        }
        
        // Generate processed data if we have sensor data
        if let Some(sensor) = &self.last_sensor {
            let processed = ProcessedData {
                temperature: sensor.temperature,
                battery_percentage: sensor.battery_percentage,
                battery_voltage: sensor.battery_voltage,
                is_charging: sensor.is_charging,
                is_on_usb: sensor.is_on_usb,
                cpu_usage_core0: sensor.cpu_usage_core0,
                cpu_usage_core1: sensor.cpu_usage_core1,
            };
            
            // Send processed data (will block if channel is full)
            let _ = self.tx.send(processed);
        }
    }
    
    
}