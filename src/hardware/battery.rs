// Battery voltage monitoring via ADC

use anyhow::Result;
use esp_idf_hal::{
    adc::{AdcDriver, AdcChannelDriver, Attenuation, config::Config as AdcConfig},
    gpio::Gpio4,
    peripherals::ADC1,
};

// Battery constants (matching Arduino implementation)
const USB_DETECT_THRESHOLD: u16 = 4400;  // mV threshold for USB power
const CHARGING_THRESHOLD: u16 = 4250;    // mV threshold to detect charging
const NO_BATTERY_ADC_MIN: u16 = 100;     // ADC values below this indicate no battery
const NO_BATTERY_ADC_MAX: u16 = 3900;    // ADC values above this indicate floating pin
const MAX_BATTERY_VOLTAGE: u16 = 4300;   // mV - maximum reasonable battery voltage

// Voltage divider constants
const VREF: f32 = 1100.0;  // ESP32 reference voltage in mV
const ADC_MAX: f32 = 4095.0;
const ATTENUATION_FACTOR: f32 = 3.9;  // For 11dB attenuation

pub struct BatteryMonitor {
    adc: AdcDriver<'static, ADC1>,
    pin: AdcChannelDriver<'static, Gpio4, Attenuation::Attenuation11dB>,
    history: [u16; 10],
    history_index: usize,
}

impl BatteryMonitor {
    pub fn new(adc1: ADC1, pin: Gpio4) -> Result<Self> {
        let config = AdcConfig::default();
        let adc = AdcDriver::new(adc1, &config)?;
        
        // Configure pin for battery monitoring with 11dB attenuation
        // This gives us a range of 0-3.9V which is perfect for battery monitoring
        let adc_pin = AdcChannelDriver::new(pin)?;
        
        Ok(Self {
            adc,
            pin: adc_pin,
            history: [0; 10],
            history_index: 0,
        })
    }
    
    pub fn read_raw(&mut self) -> u16 {
        // Read ADC value
        let raw = match self.adc.read(&mut self.pin) {
            Ok(val) => val,
            Err(e) => {
                log::warn!("ADC read error: {:?}", e);
                0
            }
        };
        
        // Add to history for averaging
        self.history[self.history_index] = raw;
        self.history_index = (self.history_index + 1) % self.history.len();
        
        raw
    }
    
    pub fn read_averaged(&self) -> u16 {
        // Calculate average of history
        let sum: u32 = self.history.iter().map(|&x| x as u32).sum();
        (sum / self.history.len() as u32) as u16
    }
    
    pub fn get_voltage_mv(&mut self) -> u16 {
        let adc_reading = self.read_averaged();
        
        // Convert ADC reading to voltage
        // With 11dB attenuation, full scale is ~3.9V
        let voltage = (adc_reading as f32 / ADC_MAX) * VREF * ATTENUATION_FACTOR;
        
        // Clamp to reasonable range
        voltage.clamp(0.0, MAX_BATTERY_VOLTAGE as f32) as u16
    }
    
    pub fn get_battery_percentage(&mut self) -> u8 {
        let voltage = self.get_voltage_mv();
        
        // Enhanced Li-ion discharge curve (matching Arduino)
        let percentage = if voltage >= 4150 {
            95 + ((voltage - 4150) * 5) / 50  // 95-100%: 4.15V to 4.20V
        } else if voltage >= 4050 {
            90 + ((voltage - 4050) * 5) / 100 // 90-95%: 4.05V to 4.15V
        } else if voltage >= 3950 {
            80 + ((voltage - 3950) * 10) / 100 // 80-90%: 3.95V to 4.05V
        } else if voltage >= 3850 {
            70 + ((voltage - 3850) * 10) / 100 // 70-80%: 3.85V to 3.95V
        } else if voltage >= 3750 {
            55 + ((voltage - 3750) * 15) / 100 // 55-70%: 3.75V to 3.85V
        } else if voltage >= 3650 {
            40 + ((voltage - 3650) * 15) / 100 // 40-55%: 3.65V to 3.75V
        } else if voltage >= 3550 {
            25 + ((voltage - 3550) * 15) / 100 // 25-40%: 3.55V to 3.65V
        } else if voltage >= 3400 {
            10 + ((voltage - 3400) * 15) / 150 // 10-25%: 3.40V to 3.55V
        } else if voltage >= 3200 {
            5 + ((voltage - 3200) * 5) / 200  // 5-10%: 3.20V to 3.40V
        } else if voltage >= 3000 {
            ((voltage - 3000) * 5) / 200      // 0-5%: 3.00V to 3.20V
        } else {
            0
        };
        
        percentage.clamp(0, 100) as u8
    }
    
    pub fn is_battery_connected(&mut self) -> bool {
        let adc_raw = self.read_raw();
        let voltage = self.get_voltage_mv();
        
        // Check if battery is connected (not floating)
        !(adc_raw < NO_BATTERY_ADC_MIN || 
          adc_raw > NO_BATTERY_ADC_MAX ||
          voltage < 2500)
    }
    
    pub fn is_on_usb_power(&mut self) -> bool {
        let voltage = self.get_voltage_mv();
        voltage > USB_DETECT_THRESHOLD || !self.is_battery_connected()
    }
    
    pub fn is_charging(&mut self) -> bool {
        let voltage = self.get_voltage_mv();
        self.is_battery_connected() && voltage > CHARGING_THRESHOLD
    }
    
    pub fn get_voltage_trend(&mut self) -> i16 {
        // Simple trend detection based on history
        if self.history_index == 0 {
            return 0;
        }
        
        let oldest = self.history[(self.history_index + 1) % self.history.len()];
        let newest = self.history[self.history_index.saturating_sub(1)];
        
        (newest as i16) - (oldest as i16)
    }
}