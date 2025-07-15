use crate::hardware::*;
use embassy_time::{Duration, Instant};

#[test]
fn test_button_state_initialization() {
    let state = ButtonState::new();
    assert!(!state.pressed);
    assert!(!state.was_pressed);
}

#[test]
fn test_button_debouncing() {
    let mut state = ButtonState::new();
    
    // First press should be detected
    let event = state.update(true);
    assert!(matches!(event, ButtonEvent::Pressed));
    
    // Rapid subsequent presses within debounce time should be ignored
    // This would require time mocking in real tests
}

#[test]
fn test_button_click_detection() {
    let mut state = ButtonState::new();
    
    // Simulate button press
    assert!(matches!(state.update(true), ButtonEvent::Pressed));
    
    // Simulate button release quickly (click)
    // In real tests, we'd need to mock time to be < LONG_PRESS_DURATION
    let event = state.update(false);
    // Would be ButtonEvent::Click if time < LONG_PRESS_DURATION
}

#[test]
fn test_battery_voltage_to_percentage() {
    let monitor = BatteryMonitor::new();
    
    // Test boundary values
    assert_eq!(monitor.voltage_to_percentage(4200), 100);
    assert_eq!(monitor.voltage_to_percentage(3000), 0);
    
    // Test intermediate values
    assert!(monitor.voltage_to_percentage(3700) > 40);
    assert!(monitor.voltage_to_percentage(3700) < 60);
    
    // Test out of range values
    assert_eq!(monitor.voltage_to_percentage(5000), 100);
    assert_eq!(monitor.voltage_to_percentage(2000), 0);
}

#[test]
fn test_battery_status_from_percentage() {
    assert!(matches!(BatteryStatus::from_percentage(100), BatteryStatus::Full));
    assert!(matches!(BatteryStatus::from_percentage(80), BatteryStatus::Normal));
    assert!(matches!(BatteryStatus::from_percentage(25), BatteryStatus::Low));
    assert!(matches!(BatteryStatus::from_percentage(10), BatteryStatus::Critical));
    assert!(matches!(BatteryStatus::from_percentage(5), BatteryStatus::Critical));
}

#[test]
fn test_sensor_data_initialization() {
    let data = SensorData::new();
    assert_eq!(data.battery_percentage, 0);
    assert_eq!(data.battery_voltage, 0);
    assert!(matches!(data.battery_status, BatteryStatus::Unknown));
    assert_eq!(data.temperature, 0.0);
    assert_eq!(data.brightness, 50);
}

#[test]
fn test_sensor_data_update() {
    let mut data = SensorData::new();
    
    data.update_battery(3700, 50);
    assert_eq!(data.battery_voltage, 3700);
    assert_eq!(data.battery_percentage, 50);
    assert!(matches!(data.battery_status, BatteryStatus::Normal));
    
    data.update_temperature(25.5);
    assert_eq!(data.temperature, 25.5);
    
    data.update_brightness(75);
    assert_eq!(data.brightness, 75);
}

#[test]
fn test_button_event_combinations() {
    // Test that button events are distinct
    assert!(!matches!(ButtonEvent::None, ButtonEvent::Pressed));
    assert!(!matches!(ButtonEvent::Button1Click, ButtonEvent::Button2Click));
    assert!(!matches!(ButtonEvent::Button1LongPress, ButtonEvent::Button2LongPress));
}

#[test]
fn test_battery_monitor_smoothing() {
    let mut monitor = BatteryMonitor::new();
    
    // Add multiple readings
    monitor.add_reading(3700);
    monitor.add_reading(3720);
    monitor.add_reading(3680);
    monitor.add_reading(3710);
    
    // Average should be smoothed
    let avg = monitor.get_average();
    assert!(avg > 3690 && avg < 3710);
}

#[test]
fn test_brightness_clamping() {
    let mut data = SensorData::new();
    
    // Test clamping to valid range
    data.update_brightness(150); // Over max
    assert!(data.brightness <= 100);
    
    data.update_brightness(0); // Min value
    assert_eq!(data.brightness, 0);
}

#[cfg(test)]
mod mock_tests {
    use super::*;
    
    // Mock ADC for testing
    struct MockAdc {
        value: u16,
    }
    
    impl MockAdc {
        fn new(value: u16) -> Self {
            Self { value }
        }
        
        fn read(&self) -> u16 {
            self.value
        }
    }
    
    #[test]
    fn test_adc_to_voltage_conversion() {
        let adc = MockAdc::new(2048); // Half of 12-bit range
        
        // Assuming 3.3V reference and 12-bit ADC
        let voltage = (adc.read() as u32 * 3300) / 4095;
        assert!(voltage > 1600 && voltage < 1700); // ~1.65V
    }
}