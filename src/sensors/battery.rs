// Battery voltage sensor implementation

use super::{Sensor, SensorError, SensorReading, ReadingQuality};
use embassy_time::Duration;
use heapless::Vec;

pub struct BatteryVoltageSensor {
    voltage_divider_ratio: f32,
    min_voltage: u16,  // mV
    max_voltage: u16,  // mV
    history: Vec<u16, 10>,
    initialized: bool,
}

impl BatteryVoltageSensor {
    pub fn new(voltage_divider_ratio: f32) -> Self {
        Self {
            voltage_divider_ratio,
            min_voltage: 3000,  // 3.0V
            max_voltage: 4200,  // 4.2V
            history: Vec::new(),
            initialized: false,
        }
    }
    
    pub fn set_voltage_range(&mut self, min_mv: u16, max_mv: u16) {
        self.min_voltage = min_mv;
        self.max_voltage = max_mv;
    }
    
    fn adc_to_voltage(&self, adc_value: u16) -> u16 {
        // Convert ADC reading to actual battery voltage
        // Account for voltage divider
        let adc_voltage = (adc_value as f32 / 4095.0) * 3300.0; // mV
        (adc_voltage * self.voltage_divider_ratio) as u16
    }
    
    pub fn voltage_to_percentage(&self, voltage_mv: u16) -> u8 {
        if voltage_mv >= self.max_voltage {
            100
        } else if voltage_mv <= self.min_voltage {
            0
        } else {
            let range = self.max_voltage - self.min_voltage;
            let offset = voltage_mv - self.min_voltage;
            ((offset as u32 * 100) / range as u32) as u8
        }
    }
    
    pub fn get_battery_status(&self, voltage_mv: u16) -> BatteryStatus {
        let percentage = self.voltage_to_percentage(voltage_mv);
        BatteryStatus::from_percentage(percentage)
    }
    
    fn add_to_history(&mut self, voltage: u16) {
        if self.history.push(voltage).is_err() {
            // Remove oldest value if full
            self.history.remove(0);
            self.history.push(voltage).ok();
        }
    }
    
    fn get_averaged_voltage(&self) -> Option<u16> {
        if self.history.is_empty() {
            None
        } else {
            let sum: u32 = self.history.iter().map(|&v| v as u32).sum();
            Some((sum / self.history.len() as u32) as u16)
        }
    }
}

impl Sensor for BatteryVoltageSensor {
    type Reading = BatteryReading;
    
    fn init(&mut self) -> Result<(), SensorError> {
        // Initialize ADC for battery monitoring
        self.initialized = true;
        Ok(())
    }
    
    fn read(&mut self) -> Result<Self::Reading, SensorError> {
        if !self.initialized {
            return Err(SensorError::NotAvailable);
        }
        
        // Simulate ADC reading
        let adc_value = 3400; // Simulated value
        let voltage_mv = self.adc_to_voltage(adc_value);
        
        // Add to history for averaging
        self.add_to_history(voltage_mv);
        
        // Use averaged value for stability
        let avg_voltage = self.get_averaged_voltage().unwrap_or(voltage_mv);
        let percentage = self.voltage_to_percentage(avg_voltage);
        let status = self.get_battery_status(avg_voltage);
        
        // Determine reading quality based on history
        let quality = if self.history.len() >= 5 {
            ReadingQuality::Excellent
        } else if self.history.len() >= 3 {
            ReadingQuality::Good
        } else {
            ReadingQuality::Fair
        };
        
        Ok(BatteryReading {
            voltage_mv: avg_voltage,
            percentage,
            status,
            quality,
            instantaneous_mv: voltage_mv,
        })
    }
    
    fn is_ready(&self) -> bool {
        self.initialized
    }
    
    fn min_read_interval(&self) -> Duration {
        Duration::from_millis(500) // Battery voltage is relatively stable
    }
    
    fn name(&self) -> &'static str {
        "Battery Voltage"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryReading {
    pub voltage_mv: u16,
    pub percentage: u8,
    pub status: BatteryStatus,
    pub quality: ReadingQuality,
    pub instantaneous_mv: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BatteryStatus {
    Full,
    Normal,
    Low,
    Critical,
    Charging,
    Unknown,
}

impl BatteryStatus {
    pub fn from_percentage(percentage: u8) -> Self {
        match percentage {
            90..=100 => Self::Full,
            20..=89 => Self::Normal,
            10..=19 => Self::Low,
            0..=9 => Self::Critical,
            _ => Self::Unknown,
        }
    }
    
    pub fn get_color(&self) -> u16 {
        use crate::display::colors::*;
        match self {
            Self::Full => GREEN,
            Self::Normal => GREEN,
            Self::Low => YELLOW,
            Self::Critical => RED,
            Self::Charging => CYAN,
            Self::Unknown => 0x4208, // Gray
        }
    }
}

// Battery health monitoring
pub struct BatteryHealthMonitor {
    charge_cycles: u32,
    total_discharge_mah: u32,
    peak_voltage: u16,
    min_voltage: u16,
}

impl BatteryHealthMonitor {
    pub fn new() -> Self {
        Self {
            charge_cycles: 0,
            total_discharge_mah: 0,
            peak_voltage: 0,
            min_voltage: 0xFFFF,
        }
    }
    
    pub fn update(&mut self, voltage_mv: u16, current_ma: i16) {
        // Track peak and minimum voltages
        if voltage_mv > self.peak_voltage {
            self.peak_voltage = voltage_mv;
        }
        if voltage_mv < self.min_voltage {
            self.min_voltage = voltage_mv;
        }
        
        // Track discharge
        if current_ma < 0 {
            // Negative current means discharging
            self.total_discharge_mah += (-current_ma as u32) / 3600; // mAh
        }
    }
    
    pub fn get_health_percentage(&self) -> u8 {
        // Simple health estimation based on voltage range
        // Real implementation would be more sophisticated
        let voltage_range = self.peak_voltage - self.min_voltage;
        if voltage_range > 1000 {
            // Large voltage range indicates worn battery
            80
        } else if voltage_range > 500 {
            90
        } else {
            100
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_voltage_to_percentage() {
        let sensor = BatteryVoltageSensor::new(2.0);
        
        assert_eq!(sensor.voltage_to_percentage(4200), 100);
        assert_eq!(sensor.voltage_to_percentage(3000), 0);
        assert_eq!(sensor.voltage_to_percentage(3600), 50);
        
        // Test clamping
        assert_eq!(sensor.voltage_to_percentage(5000), 100);
        assert_eq!(sensor.voltage_to_percentage(2000), 0);
    }
    
    #[test]
    fn test_battery_status() {
        assert!(matches!(BatteryStatus::from_percentage(95), BatteryStatus::Full));
        assert!(matches!(BatteryStatus::from_percentage(50), BatteryStatus::Normal));
        assert!(matches!(BatteryStatus::from_percentage(15), BatteryStatus::Low));
        assert!(matches!(BatteryStatus::from_percentage(5), BatteryStatus::Critical));
    }
    
    #[test]
    fn test_voltage_averaging() {
        let mut sensor = BatteryVoltageSensor::new(2.0);
        
        // Add several readings
        sensor.add_to_history(3700);
        sensor.add_to_history(3720);
        sensor.add_to_history(3680);
        sensor.add_to_history(3710);
        
        let avg = sensor.get_averaged_voltage().unwrap();
        assert!(avg >= 3690 && avg <= 3710);
    }
}