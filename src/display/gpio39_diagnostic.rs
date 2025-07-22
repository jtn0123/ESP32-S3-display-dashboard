/// GPIO39 (D0) Diagnostic Module
/// Tests for stuck HIGH condition that can cause vertical striping
use anyhow::Result;
use esp_idf_hal::gpio::{Gpio39, Input, PinDriver};
use esp_idf_sys::*;
use log::{info, warn, error};

/// Test GPIO39 for stuck HIGH condition
pub fn test_gpio39_stuck_high() -> Result<()> {
    unsafe {
        info!("=== GPIO39 (D0) Diagnostic Test ===");
        
        // First, configure GPIO39 as input to read its state
        let gpio39_num = 39;
        gpio_reset_pin(gpio39_num);
        gpio_set_direction(gpio39_num, gpio_mode_t_GPIO_MODE_INPUT);
        gpio_set_pull_mode(gpio39_num, gpio_pull_mode_t_GPIO_PULLDOWN_ONLY);
        
        // Read the pin state
        let initial_state = gpio_get_level(gpio39_num);
        info!("GPIO39 initial state (with pulldown): {}", initial_state);
        
        if initial_state == 1 {
            warn!("WARNING: GPIO39 is reading HIGH with pulldown enabled!");
            warn!("This could indicate:");
            warn!("  1. Pin is shorted to VCC");
            warn!("  2. Strong external pull-up");
            warn!("  3. Hardware damage");
        }
        
        // Try different pull configurations
        gpio_set_pull_mode(gpio39_num, gpio_pull_mode_t_GPIO_PULLUP_ONLY);
        esp_idf_hal::delay::FreeRtos::delay_ms(10);
        let pullup_state = gpio_get_level(gpio39_num);
        info!("GPIO39 state with pull-up: {}", pullup_state);
        
        gpio_set_pull_mode(gpio39_num, gpio_pull_mode_t_GPIO_FLOATING);
        esp_idf_hal::delay::FreeRtos::delay_ms(10);
        let floating_state = gpio_get_level(gpio39_num);
        info!("GPIO39 state floating: {}", floating_state);
        
        // Analysis
        if initial_state == 1 && pullup_state == 1 && floating_state == 1 {
            error!("CRITICAL: GPIO39 appears to be stuck HIGH!");
            error!("This will cause vertical striping on the display.");
            error!("Possible solutions:");
            error!("  1. Check for shorts on the PCB");
            error!("  2. Verify no solder bridges");
            error!("  3. Check if LCD controller is driving the pin");
        } else if floating_state == 1 {
            warn!("GPIO39 is HIGH when floating - possible external pull-up");
        } else {
            info!("GPIO39 appears to be functioning normally");
        }
        
        // Reset to output mode for LCD operation
        gpio_set_direction(gpio39_num, gpio_mode_t_GPIO_MODE_OUTPUT);
        gpio_set_pull_mode(gpio39_num, gpio_pull_mode_t_GPIO_FLOATING);
        
        info!("=== GPIO39 Diagnostic Complete ===");
        Ok(())
    }
}

/// Test pattern to verify D0 behavior
pub fn test_d0_pattern() -> Result<()> {
    unsafe {
        info!("=== D0 Pattern Test ===");
        
        let gpio39_num = 39;
        
        // Configure as output
        gpio_reset_pin(gpio39_num);
        gpio_set_direction(gpio39_num, gpio_mode_t_GPIO_MODE_OUTPUT);
        
        // Test toggling
        info!("Testing GPIO39 toggle pattern...");
        for i in 0..10 {
            gpio_set_level(gpio39_num, 0);
            esp_idf_hal::delay::Ets::delay_us(10);
            let low_readback = gpio_get_level(gpio39_num);
            
            gpio_set_level(gpio39_num, 1);
            esp_idf_hal::delay::Ets::delay_us(10);
            let high_readback = gpio_get_level(gpio39_num);
            
            if low_readback != 0 || high_readback != 1 {
                error!("Toggle test {} failed: low={}, high={}", i, low_readback, high_readback);
            }
        }
        
        info!("D0 pattern test complete");
        Ok(())
    }
}

/// Check all data pins for stuck conditions
pub fn check_all_data_pins() -> Result<()> {
    info!("=== Checking All LCD Data Pins ===");
    
    let data_pins = [
        (39, "D0"),
        (40, "D1"),
        (41, "D2"),
        (42, "D3"),
        (45, "D4"),
        (46, "D5"),
        (47, "D6"),
        (48, "D7"),
    ];
    
    unsafe {
        for (pin, name) in data_pins.iter() {
            // Configure as input with pulldown
            gpio_reset_pin(*pin);
            gpio_set_direction(*pin, gpio_mode_t_GPIO_MODE_INPUT);
            gpio_set_pull_mode(*pin, gpio_pull_mode_t_GPIO_PULLDOWN_ONLY);
            esp_idf_hal::delay::FreeRtos::delay_ms(5);
            
            let state = gpio_get_level(*pin);
            if state == 1 {
                warn!("Pin {} (GPIO{}) is HIGH with pulldown!", name, pin);
            } else {
                info!("Pin {} (GPIO{}) is LOW with pulldown (good)", name, pin);
            }
            
            // Reset to output
            gpio_set_direction(*pin, gpio_mode_t_GPIO_MODE_OUTPUT);
            gpio_set_pull_mode(*pin, gpio_pull_mode_t_GPIO_FLOATING);
        }
    }
    
    info!("=== Data Pin Check Complete ===");
    Ok(())
}