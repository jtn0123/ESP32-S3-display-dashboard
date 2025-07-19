/// LCD_CAM test using esp-idf-sys PAC for proper register access
/// This approach ensures correct field access and type safety
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;

pub fn lcd_cam_pac_test(
    _d0: impl Into<AnyIOPin>,
    _d1: impl Into<AnyIOPin>,
    _d2: impl Into<AnyIOPin>,
    _d3: impl Into<AnyIOPin>,
    _d4: impl Into<AnyIOPin>,
    _d5: impl Into<AnyIOPin>,
    _d6: impl Into<AnyIOPin>,
    _d7: impl Into<AnyIOPin>,
    _wr: impl Into<AnyIOPin>,
    _dc: impl Into<AnyIOPin>,
    _cs: impl Into<AnyIOPin>,
    _rst: impl Into<AnyIOPin>,
) -> Result<()> {
    log::warn!("Starting LCD_CAM PAC test with esp-idf-sys approach...");
    
    // Power pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    unsafe {
        // Get peripheral pointers
        let pcr = &*(0x600C_0000 as *const esp_idf_sys::bindings::pcr_dev_t);
        let lcd_cam = &*(0x6004_1000 as *const esp_idf_sys::bindings::lcd_cam_dev_t);
        
        log::info!("Step 1: Enable LCD_CAM clock");
        // Enable clock - using bitfield access
        (*pcr).lcd_cam_conf.modify(|r, w| {
            w.bits(r.bits() | (1 << 31)) // lcd_cam_clk_en
        });
        
        // Clear reset
        (*pcr).lcd_cam_conf.modify(|r, w| {
            w.bits(r.bits() & !(1 << 30)) // lcd_cam_rst_en
        });
        
        esp_rom_delay_us(100);
        
        log::info!("Step 2: Configure GPIO matrix for WR and D0");
        // WR pin (GPIO 8) to LCD_PCLK
        esp_rom_gpio_pad_select_gpio(8);
        gpio_set_direction(8 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(8, 154, false, false);
        
        // D0 pin (GPIO 39) to LCD_DATA_OUT0
        esp_rom_gpio_pad_select_gpio(39);
        gpio_set_direction(39 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(39, 133, false, false);
        
        log::info!("Step 3: Configure LCD clock");
        (*lcd_cam).lcd_clock.write(|w| {
            w.bits((1 << 31) | 3) // clk_en + div=3
        });
        
        log::info!("Step 4: Reset and configure FIFO - CRITICAL!");
        // Reset FIFOs
        (*lcd_cam).lcd_ctrl.modify(|r, w| {
            w.bits(r.bits() | (1 << 30) | (1 << 25)) // afifo_rst + tx_fifo_rst
        });
        esp_rom_delay_us(10);
        
        // Clear resets and set TX FIFO mode to 1 (DWord)
        (*lcd_cam).lcd_ctrl.modify(|r, w| {
            let val = r.bits() & !((1 << 30) | (1 << 25) | (0x3 << 22));
            w.bits(val | (1 << 22)) // tx_fifo_mod = 1
        });
        
        log::info!("Step 5: Configure user register");
        (*lcd_cam).lcd_user.write(|w| {
            w.bits((1 << 24) | (1 << 23)) // lcd_dout + lcd_8bits_order
        });
        
        // Apply update
        (*lcd_cam).lcd_user.modify(|r, w| {
            w.bits(r.bits() | (1 << 20)) // lcd_update
        });
        esp_rom_delay_us(10);
        (*lcd_cam).lcd_user.modify(|r, w| {
            w.bits(r.bits() & !(1 << 20))
        });
        
        log::info!("Step 6: Test single byte transfer");
        // Clear command bit (data mode)
        (*lcd_cam).lcd_user.modify(|r, w| {
            w.bits(r.bits() & !(1 << 26)) // clear lcd_cmd
        });
        
        // Write test byte
        (*lcd_cam).lcd_cmd_val.write(|w| w.bits(0xAA));
        
        // Start transfer
        (*lcd_cam).lcd_user.modify(|r, w| {
            w.bits(r.bits() | (1 << 27)) // lcd_start
        });
        
        // Wait for completion
        let mut timeout = 10000;
        while ((*lcd_cam).lcd_user.read().bits() & (1 << 27)) != 0 {
            timeout -= 1;
            if timeout == 0 {
                log::error!("Transfer timeout!");
                break;
            }
            esp_rom_delay_us(1);
        }
        
        if timeout > 0 {
            log::info!("Transfer completed successfully");
        }
        
        log::info!("Step 7: Add remaining data pins");
        const DATA_PINS: [u8; 7] = [40, 41, 42, 45, 46, 47, 48];
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            esp_rom_gpio_connect_out_signal(pin as u32, 134 + i as u32, false, false);
        }
        
        // Also configure CS (GPIO 6) and DC (GPIO 7)
        esp_rom_gpio_pad_select_gpio(6);
        gpio_set_direction(6 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(6, 132, false, false); // LCD_CS
        
        esp_rom_gpio_pad_select_gpio(7);
        gpio_set_direction(7 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        esp_rom_gpio_connect_out_signal(7, 153, false, false); // LCD_DC
        
        log::info!("Step 8: Send ST7789 initialization sequence");
        
        // Helper to send command
        let send_cmd = |cmd: u8| {
            // Set command mode (DC=0)
            (*lcd_cam).lcd_user.modify(|r, w| {
                w.bits(r.bits() | (1 << 26)) // lcd_cmd
            });
            
            (*lcd_cam).lcd_cmd_val.write(|w| w.bits(cmd as u32));
            
            (*lcd_cam).lcd_user.modify(|r, w| {
                w.bits(r.bits() | (1 << 27)) // lcd_start
            });
            
            while ((*lcd_cam).lcd_user.read().bits() & (1 << 27)) != 0 {}
        };
        
        // Helper to send data
        let send_data = |data: u8| {
            // Clear command mode (DC=1)
            (*lcd_cam).lcd_user.modify(|r, w| {
                w.bits(r.bits() & !(1 << 26))
            });
            
            (*lcd_cam).lcd_cmd_val.write(|w| w.bits(data as u32));
            
            (*lcd_cam).lcd_user.modify(|r, w| {
                w.bits(r.bits() | (1 << 27)) // lcd_start
            });
            
            while ((*lcd_cam).lcd_user.read().bits() & (1 << 27)) != 0 {}
        };
        
        // Send reset
        send_cmd(0x01); // SWRESET
        FreeRtos::delay_ms(150);
        
        // Sleep out
        send_cmd(0x11); // SLPOUT
        FreeRtos::delay_ms(120);
        
        // Set column address
        send_cmd(0x2A); // CASET
        send_data(0x00);
        send_data(0x0A); // X start = 10
        send_data(0x01);
        send_data(0x39); // X end = 313 (10 + 300 + 3)
        
        // Set row address
        send_cmd(0x2B); // RASET
        send_data(0x00);
        send_data(0x24); // Y start = 36
        send_data(0x00);
        send_data(0xCB); // Y end = 203 (36 + 168 - 1)
        
        // Memory write
        send_cmd(0x2C); // RAMWR
        
        // Send red pixels
        log::info!("Sending red pixel data...");
        for _ in 0..100 {
            send_data(0xF8); // Red high byte
            send_data(0x00); // Red low byte
        }
        
        // Display on
        send_cmd(0x29); // DISPON
        
        log::info!("Test complete - display should show red pixels if working");
        
        esp_task_wdt_reset();
    }
    
    Ok(())
}