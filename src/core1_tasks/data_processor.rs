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
    
    // Track when we last sent data to avoid spam
    last_sent: Option<Instant>,
    has_new_sensor_data: bool,
    has_new_network_data: bool,
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
            last_sent: None,
            has_new_sensor_data: false,
            has_new_network_data: false,
        }
    }
    
    pub fn process(&mut self) {
        // Process all pending sensor updates
        loop {
            match self.sensor_rx.try_recv() {
                Ok(update) => {
                    self.update_sensor_history(&update);
                    self.last_sensor = Some(update);
                    self.has_new_sensor_data = true;
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
                    self.has_new_network_data = true;
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::error!("Network channel disconnected");
                    return;
                }
            }
        }
        
        // Only send if we have new data (not just cached data)
        let should_send = if self.has_new_sensor_data || self.has_new_network_data {
            // We have new data, check if enough time has passed since last send
            if let Some(last_sent) = self.last_sent {
                // Rate limit to once per second even with new data
                last_sent.elapsed() >= Duration::from_secs(1)
            } else {
                // Never sent before
                true
            }
        } else {
            false
        };
        
        // Generate processed data if we have both sensor and network data AND should send
        if should_send {
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
                if self.tx.send(processed).is_ok() {
                    self.last_sent = Some(Instant::now());
                    self.has_new_sensor_data = false;
                    self.has_new_network_data = false;
                    log::info!("Core 1: Sent updated sensor data to main task");
                }
            }
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