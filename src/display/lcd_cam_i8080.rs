// LCD_CAM I8080 driver using esp-hal
// This replaces our from-scratch implementation with esp-hal's proven driver

use esp_idf_hal::{
    dma::{Dma, DmaPriority, DmaChannel0},
    dma_descriptors,
    gpio::{OutputPin, GpioPin},
    lcd_cam::{
        lcd::{
            i8080::{Config, I8080, TxEightBits},
            ClockMode, Phase, Polarity,
        },
        LcdCam,
    },
    peripherals::LCD_CAM,
    prelude::*,
};
use log::*;

// Our pin configuration
pub struct DisplayPins {
    pub d0: GpioPin<39>,
    pub d1: GpioPin<40>,
    pub d2: GpioPin<41>, 
    pub d3: GpioPin<42>,
    pub d4: GpioPin<45>,
    pub d5: GpioPin<46>,
    pub d6: GpioPin<47>,
    pub d7: GpioPin<48>,
    pub wr: GpioPin<8>,
}

pub struct I8080Display<'d> {
    i8080: I8080<'d, LCD_CAM, DmaChannel0>,
    framebuffer: &'static mut [u16; 320 * 170],
}

impl<'d> I8080Display<'d> {
    pub fn new(
        lcd_cam: LCD_CAM,
        channel: DmaChannel0,
        pins: DisplayPins,
    ) -> Result<Self, esp_idf_hal::lcd_cam::LcdCamError> {
        info!("Initializing I8080 display with esp-hal");
        
        // Allocate framebuffer in DMA-capable memory
        let framebuffer = Self::alloc_framebuffer()?;
        
        // Create DMA descriptors
        let (_, descriptors, _, _) = dma_descriptors!(32000);
        
        // Configure I8080
        let config = Config {
            clock_mode: ClockMode {
                polarity: Polarity::IdleLow,
                phase: Phase::ShiftLow,
            },
            cs_active_high: false,
            // Adjust timing based on your display
            setup_time: 1,
            hold_time: 1,
            clock_frequency: 20.MHz(),
        };
        
        // Create 8-bit parallel pins
        let tx_pins = TxEightBits::new(
            pins.d0, pins.d1, pins.d2, pins.d3,
            pins.d4, pins.d5, pins.d6, pins.d7,
        );
        
        // Initialize LCD_CAM peripheral
        let lcd_cam = LcdCam::new(lcd_cam);
        
        // Create I8080 instance
        let i8080 = I8080::new(
            lcd_cam.lcd,
            channel,
            descriptors,
            tx_pins,
            config,
        )?;
        
        Ok(Self {
            i8080,
            framebuffer,
        })
    }
    
    fn alloc_framebuffer() -> Result<&'static mut [u16; 320 * 170], esp_idf_hal::lcd_cam::LcdCamError> {
        // In a real implementation, use proper DMA memory allocation
        // For now, we'll use a static buffer
        static mut FRAMEBUFFER: [u16; 320 * 170] = [0; 320 * 170];
        Ok(unsafe { &mut FRAMEBUFFER })
    }
    
    pub fn send_command(&mut self, cmd: u8) -> Result<(), esp_idf_hal::lcd_cam::LcdCamError> {
        // Send single command byte
        self.i8080.send(cmd, &[cmd])
            .map_err(|_| esp_idf_hal::lcd_cam::LcdCamError::Other)?;
        Ok(())
    }
    
    pub fn send_data(&mut self, data: &[u8]) -> Result<(), esp_idf_hal::lcd_cam::LcdCamError> {
        // Send data bytes
        // Note: For ST7789, we typically send data after command
        // The first byte might need to be a dummy command byte
        self.i8080.send(0x00, data)
            .map_err(|_| esp_idf_hal::lcd_cam::LcdCamError::Other)?;
        Ok(())
    }
    
    pub fn send_pixels(&mut self, pixels: &[u16]) -> Result<(), esp_idf_hal::lcd_cam::LcdCamError> {
        // Convert u16 pixels to bytes for transmission
        let bytes: Vec<u8> = pixels.iter()
            .flat_map(|&pixel| pixel.to_be_bytes())
            .collect();
            
        self.send_data(&bytes)
    }
    
    pub fn update_framebuffer(&mut self) -> Result<(), esp_idf_hal::lcd_cam::LcdCamError> {
        // Convert framebuffer to bytes
        let bytes: Vec<u8> = self.framebuffer.iter()
            .flat_map(|&pixel| pixel.to_be_bytes())
            .collect();
            
        // Send entire framebuffer via DMA
        self.i8080.send(0x2C, &bytes) // 0x2C is RAMWR command
            .map_err(|_| esp_idf_hal::lcd_cam::LcdCamError::Other)?;
            
        Ok(())
    }
    
    pub fn clear(&mut self, color: u16) {
        for pixel in self.framebuffer.iter_mut() {
            *pixel = color;
        }
    }
    
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) {
        if x < 320 && y < 170 {
            self.framebuffer[(y as usize * 320) + x as usize] = color;
        }
    }
    
    pub fn get_framebuffer(&self) -> &[u16; 320 * 170] {
        self.framebuffer
    }
    
    pub fn get_framebuffer_mut(&mut self) -> &mut [u16; 320 * 170] {
        self.framebuffer
    }
}

// Note: This is a simplified implementation. The actual esp-hal I8080
// interface might be slightly different. We'll need to check the exact
// API when we have access to the latest esp-hal version.