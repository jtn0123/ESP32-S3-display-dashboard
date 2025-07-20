// Data processing pipeline for Core 1
// Aggregates sensor and network data, performs filtering, and sends updates to Core 0

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
use anyhow::Result;
use super::{SensorUpdate, NetworkUpdate};

#[derive(Debug, Clone)]
pub struct ProcessedData {
    pub temperature: f32,
    pub temperature_trend: TrendDirection,
    pub battery_percentage: u8,
    pub battery_voltage: u16,
    pub battery_trend: TrendDirection,
    pub is_charging: bool,
    pub is_on_usb: bool,
    pub rssi: i8,
    pub network_quality: NetworkQuality,
    pub cpu_usage_core0: u8,
    pub cpu_usage_core1: u8,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    Rising,
    Stable,
    Falling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkQuality {
    Excellent,  // RSSI > -50
    Good,       // RSSI > -60
    Fair,       // RSSI > -70
    Poor,       // RSSI <= -70
    Disconnected,
}

pub struct DataProcessor {
    sensor_rx: Receiver<SensorUpdate>,
    network_rx: Receiver<NetworkUpdate>,
    tx: Sender<ProcessedData>,
    
    // Historical data for trend analysis
    temp_history: Vec<(Instant, f32)>,
    battery_history: Vec<(Instant, u16)>,
    
    // Last known values
    last_sensor: Option<SensorUpdate>,
    last_network: Option<NetworkUpdate>,
}

impl DataProcessor {
    pub fn new(
        sensor_rx: Receiver<SensorUpdate>,
        network_rx: Receiver<NetworkUpdate>,
    ) -> Result<Self> {
        Ok(Self::new_with_channel(sensor_rx, network_rx, std::sync::mpsc::channel().0))
    }
    
    pub fn new_with_channel(
        sensor_rx: Receiver<SensorUpdate>,
        network_rx: Receiver<NetworkUpdate>,
        tx: Sender<ProcessedData>,
    ) -> Self {
        Self {
            sensor_rx,
            network_rx,
            tx,
            temp_history: Vec::with_capacity(30),
            battery_history: Vec::with_capacity(30),
            last_sensor: None,
            last_network: None,
        }
    }
    
    pub fn process(&mut self) {
        // Process all pending sensor updates
        loop {
            match self.sensor_rx.try_recv() {
                Ok(update) => {
                    self.update_sensor_history(&update);
                    self.last_sensor = Some(update);
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::error!("Sensor channel disconnected");
                    return;
                }
            }
        }
        
        // Process all pending network updates
        loop {
            match self.network_rx.try_recv() {
                Ok(update) => {
                    self.last_network = Some(update);
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::error!("Network channel disconnected");
                    return;
                }
            }
        }
        
        // Generate processed data if we have both sensor and network data
        if let (Some(sensor), Some(network)) = (&self.last_sensor, &self.last_network) {
            let processed = ProcessedData {
                temperature: sensor.temperature,
                temperature_trend: self.calculate_temp_trend(),
                battery_percentage: sensor.battery_percentage,
                battery_voltage: sensor.battery_voltage,
                battery_trend: self.calculate_battery_trend(),
                is_charging: sensor.is_charging,
                is_on_usb: sensor.is_on_usb,
                rssi: network.rssi,
                network_quality: Self::rssi_to_quality(network.rssi),
                cpu_usage_core0: sensor.cpu_usage_core0,
                cpu_usage_core1: sensor.cpu_usage_core1,
                timestamp: Instant::now(),
            };
            
            // Send processed data (will block if channel is full)
            let _ = self.tx.send(processed);
        }
    }
    
    fn update_sensor_history(&mut self, update: &SensorUpdate) {
        let now = Instant::now();
        
        // Add to history
        self.temp_history.push((now, update.temperature));
        self.battery_history.push((now, update.battery_voltage));
        
        // Keep only last 5 minutes of data
        let cutoff = now - Duration::from_secs(300);
        self.temp_history.retain(|(time, _)| *time > cutoff);
        self.battery_history.retain(|(time, _)| *time > cutoff);
    }
    
    fn calculate_temp_trend(&self) -> TrendDirection {
        if self.temp_history.len() < 3 {
            return TrendDirection::Stable;
        }
        
        // Compare average of first third vs last third
        let third = self.temp_history.len() / 3;
        let early_avg: f32 = self.temp_history[..third].iter()
            .map(|(_, temp)| temp)
            .sum::<f32>() / third as f32;
        let late_avg: f32 = self.temp_history[self.temp_history.len() - third..]
            .iter()
            .map(|(_, temp)| temp)
            .sum::<f32>() / third as f32;
        
        let diff = late_avg - early_avg;
        if diff > 0.5 {
            TrendDirection::Rising
        } else if diff < -0.5 {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        }
    }
    
    fn calculate_battery_trend(&self) -> TrendDirection {
        if self.battery_history.len() < 3 {
            return TrendDirection::Stable;
        }
        
        // Similar logic for battery
        let third = self.battery_history.len() / 3;
        let early_avg: f32 = self.battery_history[..third].iter()
            .map(|(_, volt)| *volt as f32)
            .sum::<f32>() / third as f32;
        let late_avg: f32 = self.battery_history[self.battery_history.len() - third..]
            .iter()
            .map(|(_, volt)| *volt as f32)
            .sum::<f32>() / third as f32;
        
        let diff = late_avg - early_avg;
        if diff > 20.0 {
            TrendDirection::Rising
        } else if diff < -20.0 {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        }
    }
    
    fn rssi_to_quality(rssi: i8) -> NetworkQuality {
        match rssi {
            r if r >= -50 => NetworkQuality::Excellent,
            r if r >= -60 => NetworkQuality::Good,
            r if r >= -70 => NetworkQuality::Fair,
            r if r > -90 => NetworkQuality::Poor,
            _ => NetworkQuality::Disconnected,
        }
    }
}