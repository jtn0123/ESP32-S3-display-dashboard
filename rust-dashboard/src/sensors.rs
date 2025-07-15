use anyhow::Result;
use esp_idf_hal::adc::{AdcChannelDriver, AdcDriver, Atten6dB, ADC1};
use esp_idf_hal::gpio::{ADCPin, Gpio4};
use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct SensorData {
    pub battery_voltage: f32,
    pub battery_percentage: u8,
    pub temperature: f32,
    pub light_level: u32,
    pub is_charging: bool,
}

pub struct SensorManager<'a> {
    battery_adc: AdcChannelDriver<'a, Gpio4, Atten6dB<ADC1>>,
    adc: AdcDriver<'a, ADC1>,
    last_reading: SensorData,
    last_update: Instant,
}

impl<'a> SensorManager<'a> {
    pub fn new(battery_pin: Gpio4) -> Result<Self> {
        let adc = AdcDriver::new(unsafe { ADC1::new() }, &esp_idf_hal::adc::config::Config::new())?;
        let battery_adc = AdcChannelDriver::new(battery_pin)?;
        
        Ok(Self {
            battery_adc,
            adc,
            last_reading: SensorData::default(),
            last_update: Instant::now(),
        })
    }

    pub fn read_all(&mut self) -> Result<SensorData> {
        // Read battery voltage
        let raw_adc = self.adc.read(&mut self.battery_adc)?;
        let battery_voltage = self.calculate_battery_voltage(raw_adc);
        let battery_percentage = self.voltage_to_percentage(battery_voltage);
        let is_charging = self.detect_charging(raw_adc, battery_voltage);

        // Simulated sensors for now
        let temperature = 25.0 + (Instant::now().elapsed().as_secs() % 10) as f32 * 0.5;
        let light_level = 500 + (Instant::now().elapsed().as_secs() % 100) as u32 * 10;

        let data = SensorData {
            battery_voltage,
            battery_percentage,
            temperature,
            light_level,
            is_charging,
        };

        self.last_reading = data.clone();
        self.last_update = Instant::now();

        Ok(data)
    }

    fn calculate_battery_voltage(&self, raw_adc: u16) -> f32 {
        // ESP32-S3 ADC is 12-bit (0-4095)
        // With 6dB attenuation: 0-2.2V range
        // Battery connected through voltage divider (2:1)
        const ADC_MAX: f32 = 4095.0;
        const VREF: f32 = 2.2; // 6dB attenuation reference
        const DIVIDER_RATIO: f32 = 2.0;
        
        let adc_voltage = (raw_adc as f32 / ADC_MAX) * VREF;
        adc_voltage * DIVIDER_RATIO * 1000.0 // Convert to millivolts
    }

    fn voltage_to_percentage(&self, voltage_mv: f32) -> u8 {
        // LiPo battery voltage curve
        const BATTERY_MIN_MV: f32 = 3300.0;
        const BATTERY_MAX_MV: f32 = 4200.0;
        
        if voltage_mv < 100.0 {
            // No battery connected
            return 0;
        }
        
        let percentage = ((voltage_mv - BATTERY_MIN_MV) / (BATTERY_MAX_MV - BATTERY_MIN_MV) * 100.0)
            .clamp(0.0, 100.0) as u8;
        
        percentage
    }

    fn detect_charging(&self, raw_adc: u16, voltage_mv: f32) -> bool {
        // Detection logic:
        // - ADC reading very low or very high = no battery
        // - Voltage > 4250mV = charging
        // - Voltage stable at ~4200mV = fully charged
        
        const NO_BATTERY_ADC_MIN: u16 = 100;
        const NO_BATTERY_ADC_MAX: u16 = 3900;
        const CHARGING_THRESHOLD_MV: f32 = 4250.0;
        
        if raw_adc < NO_BATTERY_ADC_MIN || raw_adc > NO_BATTERY_ADC_MAX {
            return false; // No battery
        }
        
        voltage_mv > CHARGING_THRESHOLD_MV
    }

    pub fn get_last_reading(&self) -> &SensorData {
        &self.last_reading
    }
}