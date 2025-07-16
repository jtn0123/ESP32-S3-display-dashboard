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
        bus.cs.set_high()?; // CS inactive (will pulse per transaction)
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

    /// Write a single byte to the bus
    fn write_byte(&mut self, data: u8) -> Result<()> {
        // Set data pins
        for i in 0..8 {
            if (data >> i) & 1 == 1 {
                self.data_pins[i].set_high()?;
            } else {
                self.data_pins[i].set_low()?;
            }
        }

        // Toggle write pin with proper timing
        // For Xtensa, we can't use inline assembly, but the GPIO operations
        // themselves take enough time to meet the 40ns requirement
        
        self.wr.set_low()?;
        // The GPIO write operation takes ~50-100ns on ESP32-S3
        // which exceeds the 40ns minimum hold time
        
        self.wr.set_high()?;
        // GPIO operations provide sufficient recovery time

        Ok(())
    }

    /// Write a command byte
    pub fn write_command(&mut self, cmd: u8) -> Result<()> {
        self.cs.set_low()?; // Assert CS
        self.dc.set_low()?; // Command mode
        self.write_byte(cmd)?;
        self.cs.set_high()?; // Release CS - critical for some panels
        // Small delay after command
        unsafe { esp_idf_sys::esp_rom_delay_us(10); }
        Ok(())
    }

    /// Write a data byte
    pub fn write_data(&mut self, data: u8) -> Result<()> {
        self.cs.set_low()?; // Assert CS
        self.dc.set_high()?; // Data mode
        self.write_byte(data)?;
        self.cs.set_high()?; // Release CS
        Ok(())
    }

    /// Write multiple data bytes efficiently
    pub fn write_data_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.cs.set_low()?; // Assert CS
        self.dc.set_high()?; // Data mode
        
        for &byte in data {
            self.write_byte(byte)?;
        }
        
        self.cs.set_high()?; // Release CS
        Ok(())
    }

    /// Write a 16-bit value as two bytes
    pub fn write_data_16(&mut self, data: u16) -> Result<()> {
        self.write_data((data >> 8) as u8)?;
        self.write_data((data & 0xFF) as u8)?;
        Ok(())
    }

    /// Begin a data write sequence (caller must call end_write when done)
    pub fn begin_write(&mut self) -> Result<()> {
        self.cs.set_low()?; // Assert CS for bulk write
        self.dc.set_high()?;
        Ok(())
    }

    /// Write raw bytes during a write sequence
    pub fn write_raw(&mut self, data: &[u8]) -> Result<()> {
        for &byte in data {
            self.write_byte(byte)?;
        }
        Ok(())
    }

    /// End a data write sequence
    pub fn end_write(&mut self) -> Result<()> {
        self.cs.set_high()?; // Release CS after bulk write
        Ok(())
    }
}