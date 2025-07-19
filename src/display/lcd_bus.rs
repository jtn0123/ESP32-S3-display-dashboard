use anyhow::Result;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, Output};
use esp_idf_hal::delay::FreeRtos;  // For delay_ms

/// Low-level 8-bit parallel LCD bus driver for ST7789
pub struct LcdBus {
    data_pins: [PinDriver<'static, AnyIOPin, Output>; 8],
    wr: PinDriver<'static, AnyIOPin, Output>,
    dc: PinDriver<'static, AnyIOPin, Output>,
    cs: PinDriver<'static, AnyIOPin, Output>,
    rst: PinDriver<'static, AnyIOPin, Output>,
}

impl LcdBus {
    pub fn new(
        d0: impl Into<AnyIOPin> + 'static,
        d1: impl Into<AnyIOPin> + 'static,
        d2: impl Into<AnyIOPin> + 'static,
        d3: impl Into<AnyIOPin> + 'static,
        d4: impl Into<AnyIOPin> + 'static,
        d5: impl Into<AnyIOPin> + 'static,
        d6: impl Into<AnyIOPin> + 'static,
        d7: impl Into<AnyIOPin> + 'static,
        wr: impl Into<AnyIOPin> + 'static,
        dc: impl Into<AnyIOPin> + 'static,
        cs: impl Into<AnyIOPin> + 'static,
        rst: impl Into<AnyIOPin> + 'static,
    ) -> Result<Self> {
        let mut bus = Self {
            data_pins: [
                PinDriver::output(d0.into())?,
                PinDriver::output(d1.into())?,
                PinDriver::output(d2.into())?,
                PinDriver::output(d3.into())?,
                PinDriver::output(d4.into())?,
                PinDriver::output(d5.into())?,
                PinDriver::output(d6.into())?,
                PinDriver::output(d7.into())?,
            ],
            wr: PinDriver::output(wr.into())?,
            dc: PinDriver::output(dc.into())?,
            cs: PinDriver::output(cs.into())?,
            rst: PinDriver::output(rst.into())?,
        };

        // Clear all data pins to prevent static
        for pin in &mut bus.data_pins {
            pin.set_low()?;
        }
        
        // Initial control pin states
        bus.cs.set_low()?; // CS active (keep low like Arduino)
        bus.wr.set_high()?; // WR inactive
        bus.dc.set_high()?; // DC in data mode initially
        bus.rst.set_high()?; // RST inactive
        
        // Small delay to ensure stable state
        FreeRtos::delay_ms(10);
        
        Ok(bus)
    }

    /// Perform hardware reset
    pub fn reset(&mut self) -> Result<()> {
        self.rst.set_high()?;
        FreeRtos::delay_ms(10);
        self.rst.set_low()?;
        FreeRtos::delay_ms(10);
        self.rst.set_high()?;
        FreeRtos::delay_ms(120);
        Ok(())
    }

    /// Write a single byte to the bus - optimized version
    #[inline(always)]
    fn write_byte(&mut self, data: u8) -> Result<()> {
        // Set all 8 data pins as fast as possible
        // Unroll the loop for better performance
        if data & 0x01 != 0 { self.data_pins[0].set_high()?; } else { self.data_pins[0].set_low()?; }
        if data & 0x02 != 0 { self.data_pins[1].set_high()?; } else { self.data_pins[1].set_low()?; }
        if data & 0x04 != 0 { self.data_pins[2].set_high()?; } else { self.data_pins[2].set_low()?; }
        if data & 0x08 != 0 { self.data_pins[3].set_high()?; } else { self.data_pins[3].set_low()?; }
        if data & 0x10 != 0 { self.data_pins[4].set_high()?; } else { self.data_pins[4].set_low()?; }
        if data & 0x20 != 0 { self.data_pins[5].set_high()?; } else { self.data_pins[5].set_low()?; }
        if data & 0x40 != 0 { self.data_pins[6].set_high()?; } else { self.data_pins[6].set_low()?; }
        if data & 0x80 != 0 { self.data_pins[7].set_high()?; } else { self.data_pins[7].set_low()?; }

        // Toggle write pin - no delays needed for ST7789
        self.wr.set_low()?;
        self.wr.set_high()?;

        Ok(())
    }

    /// Write a command byte
    pub fn write_command(&mut self, cmd: u8) -> Result<()> {
        // CS already low from init
        self.dc.set_low()?; // Command mode
        self.write_byte(cmd)?;
        // Keep CS low (matching Arduino behavior)
        // Increased delay after command for stability
        unsafe { esp_idf_sys::esp_rom_delay_us(50); }
        Ok(())
    }

    /// Write a data byte
    pub fn write_data(&mut self, data: u8) -> Result<()> {
        // CS already low from init
        self.dc.set_high()?; // Data mode
        self.write_byte(data)?;
        // Keep CS low
        Ok(())
    }

    /// Write multiple data bytes efficiently
    pub fn write_data_bytes(&mut self, data: &[u8]) -> Result<()> {
        // CS already low from init
        self.dc.set_high()?; // Data mode
        
        for &byte in data {
            self.write_byte(byte)?;
        }
        
        // Keep CS low
        Ok(())
    }

    /// Write a 16-bit value as two bytes
    pub fn write_data_16(&mut self, data: u16) -> Result<()> {
        self.write_data((data >> 8) as u8)?;
        self.write_data((data & 0xFF) as u8)?;
        Ok(())
    }
    
    /// Write multiple pixels efficiently with optimized inner loop
    pub fn write_pixels(&mut self, color: u16, count: u32) -> Result<()> {
        // Keep DC high for data
        self.dc.set_high()?;
        
        let high_byte = (color >> 8) as u8;
        let low_byte = (color & 0xFF) as u8;
        
        // For very large fills, use an even more optimized approach
        if count > 500 {
            // Pre-set pins for high byte
            self.set_data_pins_fast(high_byte)?;
            
            // Unroll the inner loop for better performance
            let mut remaining = count;
            while remaining >= 16 {
                // Write 16 pixels worth of high bytes
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                
                // Set pins for low byte
                self.set_data_pins_fast(low_byte)?;
                
                // Write 16 pixels worth of low bytes
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                self.wr.set_low()?; self.wr.set_high()?;
                
                // Back to high byte
                self.set_data_pins_fast(high_byte)?;
                
                remaining -= 16;
                
                // Feed watchdog less frequently
                if remaining % 2048 == 0 {
                    unsafe { esp_idf_sys::esp_task_wdt_reset(); }
                }
            }
            
            // Handle remaining pixels
            for _ in 0..remaining {
                self.write_byte(high_byte)?;
                self.write_byte(low_byte)?;
            }
        } else {
            // For smaller fills, use simpler approach
            for _ in 0..count {
                self.write_byte(high_byte)?;
                self.write_byte(low_byte)?;
            }
        }
        
        Ok(())
    }
    
    /// Fast method to set all data pins at once
    #[inline]
    fn set_data_pins_fast(&mut self, data: u8) -> Result<()> {
        // Unrolled for performance
        if data & 0x01 != 0 { self.data_pins[0].set_high()?; } else { self.data_pins[0].set_low()?; }
        if data & 0x02 != 0 { self.data_pins[1].set_high()?; } else { self.data_pins[1].set_low()?; }
        if data & 0x04 != 0 { self.data_pins[2].set_high()?; } else { self.data_pins[2].set_low()?; }
        if data & 0x08 != 0 { self.data_pins[3].set_high()?; } else { self.data_pins[3].set_low()?; }
        if data & 0x10 != 0 { self.data_pins[4].set_high()?; } else { self.data_pins[4].set_low()?; }
        if data & 0x20 != 0 { self.data_pins[5].set_high()?; } else { self.data_pins[5].set_low()?; }
        if data & 0x40 != 0 { self.data_pins[6].set_high()?; } else { self.data_pins[6].set_low()?; }
        if data & 0x80 != 0 { self.data_pins[7].set_high()?; } else { self.data_pins[7].set_low()?; }
        Ok(())
    }

    // /// Begin a data write sequence (caller must call end_write when done)
    // pub fn begin_write(&mut self) -> Result<()> {
    //     // CS already low from init
    //     self.dc.set_high()?;
    //     Ok(())
    // }

    // /// Write raw bytes during a write sequence
    // pub fn write_raw(&mut self, data: &[u8]) -> Result<()> {
    //     for &byte in data {
    //         self.write_byte(byte)?;
    //     }
    //     Ok(())
    // }

    // /// End a data write sequence
    // pub fn end_write(&mut self) -> Result<()> {
    //     // Keep CS low (matching Arduino behavior)
    //     Ok(())
    // }
}