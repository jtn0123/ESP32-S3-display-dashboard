use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_HISTORY_POINTS: usize = 360; // 6 hours at 1 sample/minute for memory efficiency

#[derive(Debug, Clone, serde::Serialize)]
pub struct DataPoint {
    pub timestamp: u64,
    pub value: f32,
}

pub struct SensorHistory {
    temperature: Mutex<VecDeque<DataPoint>>,
    battery: Mutex<VecDeque<DataPoint>>,
    humidity: Mutex<VecDeque<DataPoint>>,
}

impl SensorHistory {
    pub fn new() -> Self {
        Self {
            temperature: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
            battery: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
            humidity: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
        }
    }

    pub fn add_temperature(&self, value: f32) {
        self.add_data_point(&self.temperature, value);
    }

    pub fn add_battery(&self, value: f32) {
        self.add_data_point(&self.battery, value);
    }

    pub fn add_humidity(&self, value: f32) {
        self.add_data_point(&self.humidity, value);
    }

    fn add_data_point(&self, queue: &Mutex<VecDeque<DataPoint>>, value: f32) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut data = queue.lock().unwrap();
        data.push_back(DataPoint { timestamp, value });

        // Remove old data points
        while data.len() > MAX_HISTORY_POINTS {
            data.pop_front();
        }
    }

    pub fn get_temperature_history(&self, hours: u32) -> Vec<DataPoint> {
        self.get_history(&self.temperature, hours)
    }

    pub fn get_battery_history(&self, hours: u32) -> Vec<DataPoint> {
        self.get_history(&self.battery, hours)
    }

    pub fn get_humidity_history(&self, hours: u32) -> Vec<DataPoint> {
        self.get_history(&self.humidity, hours)
    }

    fn get_history(&self, queue: &Mutex<VecDeque<DataPoint>>, hours: u32) -> Vec<DataPoint> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(hours as u64 * 3600);

        let data = queue.lock().unwrap();
        data.iter()
            .filter(|dp| dp.timestamp >= cutoff)
            .cloned()
            .collect()
    }

    pub fn clear_all(&self) {
        self.temperature.lock().unwrap().clear();
        self.battery.lock().unwrap().clear();
        self.humidity.lock().unwrap().clear();
    }
}