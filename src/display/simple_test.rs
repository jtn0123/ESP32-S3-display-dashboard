/// Simple GPIO test to verify display is working
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_sys::*;

/// Simple test that just toggles all data pins
pub fn simple_gpio_test() -> Result<()> {
    log::warn!("Starting simple GPIO test...");
    
    // Initialize LCD power pin (GPIO 15) - CRITICAL!
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        log::info!("LCD power enabled on GPIO 15");
    }
    
    // Initialize backlight pin (GPIO 38)
    unsafe {
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
        log::info!("Backlight enabled on GPIO 38");
    }
    
    // Wait for power to stabilize
    FreeRtos::delay_ms(500);
    
    // Initialize all display pins as outputs
    let pins = [39, 40, 41, 42, 45, 46, 47, 48, // Data pins
                8,  // WR
                7,  // DC
                6,  // CS
                5]; // RST
    
    for &pin in &pins {
        unsafe {
            esp_rom_gpio_pad_select_gpio(pin);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_level(pin as gpio_num_t, 0);
        }
    }
    
    // Reset display
    unsafe {
        gpio_set_level(5 as gpio_num_t, 1);
        FreeRtos::delay_ms(10);
        gpio_set_level(5 as gpio_num_t, 0);
        FreeRtos::delay_ms(10);
        gpio_set_level(5 as gpio_num_t, 1);
        FreeRtos::delay_ms(120);
    }
    
    log::info!("Display reset complete");
    
    // Toggle all pins to see if display responds
    let mut count = 0u32;
    loop {
        // Toggle all data pins
        for &pin in &[39, 40, 41, 42, 45, 46, 47, 48] {
            unsafe {
                gpio_set_level(pin as gpio_num_t, (count & 1) as u32);
            }
        }
        
        // Toggle control pins
        unsafe {
            gpio_set_level(8 as gpio_num_t, (count & 1) as u32); // WR
            gpio_set_level(6 as gpio_num_t, 0); // CS always low
        }
        
        count += 1;
        
        if count % 100000 == 0 {
            log::info!("GPIO toggle test: {} cycles", count);
            unsafe { esp_task_wdt_reset(); }
        }
        
        // Small delay - use FreeRTOS vTaskDelay
        if count % 100 == 0 {
            unsafe { vTaskDelay(1); }
        }
    }
}