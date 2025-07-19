use anyhow::Result;
use esp_idf_hal::gpio::{AnyIOPin, PinDriver, Output, Pin};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_sys::*;

// GPIO register addresses for ESP32-S3
const GPIO_OUT_W1TS_REG: u32 = 0x60004008; // Set output high
const GPIO_OUT_W1TC_REG: u32 = 0x6000400C; // Set output low
const GPIO_OUT1_W1TS_REG: u32 = 0x60004014; // Set output high (GPIOs 32-53)
const GPIO_OUT1_W1TC_REG: u32 = 0x60004018; // Set output low (GPIOs 32-53)

/// Fast parallel LCD bus driver using direct register access
pub struct LcdBusFast {
    // Keep HAL drivers for initialization and safety
    data_pins: [PinDriver<'static, AnyIOPin, Output>; 8],
    wr: PinDriver<'static, AnyIOPin, Output>,
    dc: PinDriver<'static, AnyIOPin, Output>,
    cs: PinDriver<'static, AnyIOPin, Output>,
    rst: PinDriver<'static, AnyIOPin, Output>,
    
    // Pre-calculated pin masks for fast access
    data_pin_nums: [i32; 8],
    wr_pin_num: i32,
    dc_pin_num: i32,
    
    // Masks for fast GPIO operations
    data_masks_low: [u32; 256],  // Pre-calculated masks for GPIOs 0-31
    data_masks_high: [u32; 256], // Pre-calculated masks for GPIOs 32-53
    wr_mask_low: u32,
    wr_mask_high: u32,
    dc_mask_low: u32,
    dc_mask_high: u32,
}

impl LcdBusFast {
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
        // Convert to AnyIOPin
        let d0_pin = d0.into();
        let d1_pin = d1.into();
        let d2_pin = d2.into();
        let d3_pin = d3.into();
        let d4_pin = d4.into();
        let d5_pin = d5.into();
        let d6_pin = d6.into();
        let d7_pin = d7.into();
        let wr_pin = wr.into();
        let dc_pin = dc.into();
        
        // Extract pin numbers for direct register access
        let data_pin_nums = [
            d0_pin.pin(), d1_pin.pin(), d2_pin.pin(), d3_pin.pin(),
            d4_pin.pin(), d5_pin.pin(), d6_pin.pin(), d7_pin.pin(),
        ];
        let wr_pin_num = wr_pin.pin();
        let dc_pin_num = dc_pin.pin();
        
        // Pre-calculate all possible data byte masks
        let mut data_masks_low = [0u32; 256];
        let mut data_masks_high = [0u32; 256];
        
        for byte_val in 0..256 {
            let mut mask_low = 0u32;
            let mut mask_high = 0u32;
            
            for bit in 0..8 {
                let pin_num = data_pin_nums[bit];
                if byte_val & (1 << bit) != 0 {
                    if pin_num < 32 {
                        mask_low |= 1 << pin_num;
                    } else {
                        mask_high |= 1 << (pin_num - 32);
                    }
                }
            }
            
            data_masks_low[byte_val] = mask_low;
            data_masks_high[byte_val] = mask_high;
        }
        
        // Calculate control pin masks
        let (wr_mask_low, wr_mask_high) = if wr_pin_num < 32 {
            (1u32 << wr_pin_num, 0u32)
        } else {
            (0u32, 1u32 << (wr_pin_num - 32))
        };
        
        let (dc_mask_low, dc_mask_high) = if dc_pin_num < 32 {
            (1u32 << dc_pin_num, 0u32)
        } else {
            (0u32, 1u32 << (dc_pin_num - 32))
        };
        
        let mut bus = Self {
            data_pins: [
                PinDriver::output(d0_pin)?,
                PinDriver::output(d1_pin)?,
                PinDriver::output(d2_pin)?,
                PinDriver::output(d3_pin)?,
                PinDriver::output(d4_pin)?,
                PinDriver::output(d5_pin)?,
                PinDriver::output(d6_pin)?,
                PinDriver::output(d7_pin)?,
            ],
            wr: PinDriver::output(wr_pin)?,
            dc: PinDriver::output(dc_pin)?,
            cs: PinDriver::output(cs.into())?,
            rst: PinDriver::output(rst.into())?,
            data_pin_nums,
            wr_pin_num,
            dc_pin_num,
            data_masks_low,
            data_masks_high,
            wr_mask_low,
            wr_mask_high,
            dc_mask_low,
            dc_mask_high,
        };

        // Initialize pins
        for pin in &mut bus.data_pins {
            pin.set_low()?;
        }
        
        bus.cs.set_low()?;
        bus.wr.set_high()?;
        bus.dc.set_high()?;
        bus.rst.set_high()?;
        
        FreeRtos::delay_ms(10);
        
        Ok(bus)
    }

    pub fn reset(&mut self) -> Result<()> {
        self.rst.set_high()?;
        FreeRtos::delay_ms(10);
        self.rst.set_low()?;
        FreeRtos::delay_ms(10);
        self.rst.set_high()?;
        FreeRtos::delay_ms(120);
        Ok(())
    }

    /// Ultra-fast byte write using direct register access
    #[inline(always)]
    unsafe fn write_byte_fast(&self, data: u8) {
        // Calculate which pins need to be set/cleared
        let set_mask_low = self.data_masks_low[data as usize];
        let set_mask_high = self.data_masks_high[data as usize];
        let clear_mask_low = self.data_masks_low[!data as usize];
        let clear_mask_high = self.data_masks_high[!data as usize];
        
        // Set data pins
        if clear_mask_low != 0 {
            core::ptr::write_volatile(GPIO_OUT_W1TC_REG as *mut u32, clear_mask_low);
        }
        if clear_mask_high != 0 {
            core::ptr::write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, clear_mask_high);
        }
        if set_mask_low != 0 {
            core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, set_mask_low);
        }
        if set_mask_high != 0 {
            core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, set_mask_high);
        }
        
        // Toggle WR pin
        if self.wr_mask_low != 0 {
            core::ptr::write_volatile(GPIO_OUT_W1TC_REG as *mut u32, self.wr_mask_low);
            core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, self.wr_mask_low);
        } else {
            core::ptr::write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, self.wr_mask_high);
            core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, self.wr_mask_high);
        }
    }

    /// Write a single byte (fallback to HAL for safety in non-critical paths)
    #[inline(always)]
    fn write_byte(&mut self, data: u8) -> Result<()> {
        unsafe { self.write_byte_fast(data); }
        Ok(())
    }

    pub fn write_command(&mut self, cmd: u8) -> Result<()> {
        // Use HAL for DC pin (command mode)
        self.dc.set_low()?;
        self.write_byte(cmd)?;
        unsafe { esp_rom_delay_us(50); }
        Ok(())
    }

    pub fn write_data(&mut self, data: u8) -> Result<()> {
        // Use HAL for DC pin (data mode)
        self.dc.set_high()?;
        self.write_byte(data)?;
        Ok(())
    }

    pub fn write_data_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.dc.set_high()?;
        
        // Use fast path for bulk data
        unsafe {
            for &byte in data {
                self.write_byte_fast(byte);
            }
        }
        
        Ok(())
    }

    pub fn write_data_16(&mut self, data: u16) -> Result<()> {
        self.write_data((data >> 8) as u8)?;
        self.write_data((data & 0xFF) as u8)?;
        Ok(())
    }
    
    /// Ultra-optimized pixel writing using direct register access
    #[inline(never)] // Prevent inlining of large function
    pub fn write_pixels(&mut self, color: u16, count: u32) -> Result<()> {
        self.dc.set_high()?;
        
        let high_byte = (color >> 8) as u8;
        let low_byte = (color & 0xFF) as u8;
        
        unsafe {
            // Set DC high using direct register access
            if self.dc_mask_low != 0 {
                core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, self.dc_mask_low);
            } else {
                core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, self.dc_mask_high);
            }
            
            // Pre-calculate masks for both bytes
            let high_set_low = self.data_masks_low[high_byte as usize];
            let high_set_high = self.data_masks_high[high_byte as usize];
            let high_clear_low = self.data_masks_low[!high_byte as usize];
            let high_clear_high = self.data_masks_high[!high_byte as usize];
            
            let low_set_low = self.data_masks_low[low_byte as usize];
            let low_set_high = self.data_masks_high[low_byte as usize];
            let low_clear_low = self.data_masks_low[!low_byte as usize];
            let low_clear_high = self.data_masks_high[!low_byte as usize];
            
            // Super aggressive unrolling for maximum performance
            let mut remaining = count;
            
            // Handle blocks of 16 pixels at once
            while remaining >= 16 {
                // Write 16 pixels (32 bytes) with aggressive unrolling
                // This reduces loop overhead significantly
                for _ in 0..16 {
                    // High byte
                    if high_clear_low != 0 { core::ptr::write_volatile(GPIO_OUT_W1TC_REG as *mut u32, high_clear_low); }
                    if high_clear_high != 0 { core::ptr::write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, high_clear_high); }
                    if high_set_low != 0 { core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, high_set_low); }
                    if high_set_high != 0 { core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, high_set_high); }
                    
                    // WR pulse for high byte
                    if self.wr_mask_low != 0 {
                        core::ptr::write_volatile(GPIO_OUT_W1TC_REG as *mut u32, self.wr_mask_low);
                        core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, self.wr_mask_low);
                    } else {
                        core::ptr::write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, self.wr_mask_high);
                        core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, self.wr_mask_high);
                    }
                    
                    // Low byte
                    if low_clear_low != 0 { core::ptr::write_volatile(GPIO_OUT_W1TC_REG as *mut u32, low_clear_low); }
                    if low_clear_high != 0 { core::ptr::write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, low_clear_high); }
                    if low_set_low != 0 { core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, low_set_low); }
                    if low_set_high != 0 { core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, low_set_high); }
                    
                    // WR pulse for low byte
                    if self.wr_mask_low != 0 {
                        core::ptr::write_volatile(GPIO_OUT_W1TC_REG as *mut u32, self.wr_mask_low);
                        core::ptr::write_volatile(GPIO_OUT_W1TS_REG as *mut u32, self.wr_mask_low);
                    } else {
                        core::ptr::write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, self.wr_mask_high);
                        core::ptr::write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, self.wr_mask_high);
                    }
                }
                
                remaining -= 16;
                
                // Feed watchdog less frequently
                if remaining % 4096 == 0 {
                    esp_task_wdt_reset();
                }
            }
            
            // Handle remaining pixels
            for _ in 0..remaining {
                self.write_byte_fast(high_byte);
                self.write_byte_fast(low_byte);
            }
        }
        
        Ok(())
    }
}