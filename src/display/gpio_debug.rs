/// GPIO debugging utilities
use esp_idf_sys::*;
use log::info;

pub unsafe fn verify_gpio_states() {
    info!("=== GPIO State Verification ===");
    
    // Data pins
    let data_pins = [39, 40, 41, 42, 45, 46, 47, 48];
    info!("Data pins (D0-D7):");
    for (i, pin) in data_pins.iter().enumerate() {
        let level = gpio_get_level(*pin as gpio_num_t);
        info!("  D{} (GPIO{}): {}", i, pin, if level != 0 { "HIGH" } else { "LOW" });
    }
    
    // Control pins
    info!("Control pins:");
    let control_pins = [
        (8, "WR"),
        (7, "DC"),
        (6, "CS"),
        (5, "RST"),
        (9, "RD"),
    ];
    
    for (pin, name) in control_pins.iter() {
        let level = gpio_get_level(*pin as gpio_num_t);
        info!("  {} (GPIO{}): {}", name, pin, if level != 0 { "HIGH" } else { "LOW" });
    }
    
    // Power pins
    info!("Power pins:");
    let power_pins = [
        (15, "LCD_PWR"),
        (38, "BACKLIGHT"),
    ];
    
    for (pin, name) in power_pins.iter() {
        let level = gpio_get_level(*pin as gpio_num_t);
        info!("  {} (GPIO{}): {}", name, pin, if level != 0 { "HIGH" } else { "LOW" });
    }
    
    info!("=== End GPIO Verification ===");
}

pub unsafe fn pulse_pin(gpio_num: gpio_num_t, duration_ms: u32) {
    info!("Pulsing GPIO{} for {}ms", gpio_num, duration_ms);
    
    // Save current state
    let original = gpio_get_level(gpio_num);
    
    // Toggle
    gpio_set_level(gpio_num, if original != 0 { 0 } else { 1 });
    esp_idf_hal::delay::Ets::delay_ms(duration_ms);
    
    // Restore
    gpio_set_level(gpio_num, original as u32);
    
    info!("GPIO{} restored to {}", gpio_num, if original != 0 { "HIGH" } else { "LOW" });
}