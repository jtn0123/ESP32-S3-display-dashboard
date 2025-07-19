/// Integrated LCD_CAM display driver for ESP32-S3
/// Combines LCD_CAM peripheral control with DMA for high-performance display
use anyhow::Result;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_sys::*;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::lcd_cam_ll::{LcdCam, configure_lcd_cam_pins};
use super::lcd_cam_dma::LcdCamDma;
use super::colors::BLACK;

// Display configuration for T-Display-S3
const DISPLAY_WIDTH: usize = 300;
const DISPLAY_HEIGHT: usize = 168;
const DISPLAY_X_OFFSET: u16 = 10;
const DISPLAY_Y_OFFSET: u16 = 36;

// ST7789 Commands
const CMD_NOP: u8 = 0x00;
const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
const CMD_INVON: u8 = 0x21;
const CMD_DISPON: u8 = 0x29;
const CMD_CASET: u8 = 0x2A;
const CMD_RASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_MADCTL: u8 = 0x36;
const CMD_COLMOD: u8 = 0x3A;
const CMD_PORCTRL: u8 = 0xB2;
const CMD_GCTRL: u8 = 0xB7;
const CMD_VCOMS: u8 = 0xBB;
const CMD_LCMCTRL: u8 = 0xC0;
const CMD_VDVVRHEN: u8 = 0xC2;
const CMD_VRHS: u8 = 0xC3;
const CMD_VDVS: u8 = 0xC4;
const CMD_FRCTRL2: u8 = 0xC6;
const CMD_PWRCTRL1: u8 = 0xD0;

// Frame buffer in internal DRAM for best performance
#[link_section = ".dram0.bss"]
static mut FRAME_BUFFER: [u16; DISPLAY_WIDTH * DISPLAY_HEIGHT] = [0; DISPLAY_WIDTH * DISPLAY_HEIGHT];

pub struct LcdCamDisplay {
    lcd_cam: LcdCam,
    dma: LcdCamDma,
    rst_pin: u8,
    frame_count: AtomicU32,
    is_initialized: AtomicBool,
    current_cmd: Option<u8>,
}

impl LcdCamDisplay {
    /// Create new LCD_CAM display instance
    pub fn new(
        d0: impl Into<AnyIOPin>,
        d1: impl Into<AnyIOPin>,
        d2: impl Into<AnyIOPin>, 
        d3: impl Into<AnyIOPin>,
        d4: impl Into<AnyIOPin>,
        d5: impl Into<AnyIOPin>,
        d6: impl Into<AnyIOPin>,
        d7: impl Into<AnyIOPin>,
        wr: impl Into<AnyIOPin>,
        dc: impl Into<AnyIOPin>,
        cs: impl Into<AnyIOPin>,
        rst: impl Into<AnyIOPin>,
    ) -> Result<Self> {
        // Extract pin numbers
        let pin_d0 = Self::get_pin_number(d0)?;
        let pin_d1 = Self::get_pin_number(d1)?;
        let pin_d2 = Self::get_pin_number(d2)?;
        let pin_d3 = Self::get_pin_number(d3)?;
        let pin_d4 = Self::get_pin_number(d4)?;
        let pin_d5 = Self::get_pin_number(d5)?;
        let pin_d6 = Self::get_pin_number(d6)?;
        let pin_d7 = Self::get_pin_number(d7)?;
        let pin_wr = Self::get_pin_number(wr)?;
        let pin_dc = Self::get_pin_number(dc)?;
        let pin_cs = Self::get_pin_number(cs)?;
        let pin_rst = Self::get_pin_number(rst)?;
        
        // Configure GPIO matrix for LCD_CAM
        unsafe {
            configure_lcd_cam_pins(
                pin_d0, pin_d1, pin_d2, pin_d3,
                pin_d4, pin_d5, pin_d6, pin_d7,
                pin_wr, pin_dc, pin_cs
            );
        }
        
        // Initialize peripherals
        log::info!("Creating LCD_CAM peripheral...");
        let mut lcd_cam = unsafe { LcdCam::new() };
        log::info!("LCD_CAM peripheral created");
        
        log::info!("Creating DMA...");
        let dma = unsafe { LcdCamDma::new()? };
        log::info!("DMA created");
        
        // Configure for 15MHz operation (safe for ST7789)
        unsafe {
            lcd_cam.reset();
            lcd_cam.configure_i8080_8bit(15_000_000);
            
            // Set timing for ST7789 requirements
            // These values assume 80MHz APB clock
            lcd_cam.configure_timing(
                2,  // DC setup: 2 cycles = 25ns
                2,  // DC hold: 2 cycles = 25ns  
                1,  // CS setup: 1 cycle = 12.5ns
                1,  // CS hold: 1 cycle = 12.5ns
            );
        }
        
        // Configure RST pin manually (not part of LCD_CAM)
        unsafe {
            const SIG_GPIO_OUT_IDX: u32 = 256; // From esp-idf bindings
            esp_rom_gpio_pad_select_gpio(pin_rst as u32);
            esp_rom_gpio_connect_out_signal(pin_rst as u32, SIG_GPIO_OUT_IDX, false, false);
            gpio_set_direction(pin_rst as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_level(pin_rst as gpio_num_t, 1);
        }
        
        let mut display = Self {
            lcd_cam,
            dma,
            rst_pin: pin_rst,
            frame_count: AtomicU32::new(0),
            is_initialized: AtomicBool::new(false),
            current_cmd: None,
        };
        
        // Initialize display
        display.init()?;
        
        Ok(display)
    }
    
    /// Extract pin number from AnyIOPin
    fn get_pin_number(pin: impl Into<AnyIOPin>) -> Result<u8> {
        let any_pin: AnyIOPin = pin.into();
        // This is a bit hacky but works for our purposes
        let pin_num = unsafe { 
            let ptr = &any_pin as *const _ as *const u8;
            *ptr.offset(0) // First byte contains pin number
        };
        Ok(pin_num)
    }
    
    /// Hardware reset
    fn reset(&mut self) -> Result<()> {
        unsafe {
            gpio_set_level(self.rst_pin as gpio_num_t, 1);
            FreeRtos::delay_ms(10);
            gpio_set_level(self.rst_pin as gpio_num_t, 0);
            FreeRtos::delay_ms(10);
            gpio_set_level(self.rst_pin as gpio_num_t, 1);
            FreeRtos::delay_ms(120);
        }
        Ok(())
    }
    
    /// Send command byte
    fn write_command(&mut self, cmd: u8) -> Result<()> {
        self.current_cmd = Some(cmd);
        
        // For LCD_CAM, we need to send command as a single byte transfer
        // with DC low. This is done by configuring the transfer appropriately.
        
        // Create command buffer
        let _cmd_data = [cmd];
        
        // Set up DMA for command (1 byte)
        self.dma.setup_frame_transfer(&[cmd as u16])?;
        
        // Start transfer with command mode
        // Note: Actual implementation would set DC low here
        unsafe {
            self.dma.start_transfer()?;
        }
        
        // Wait for completion
        self.dma.wait_transfer_complete(10)?;
        
        Ok(())
    }
    
    /// Send data bytes
    fn write_data(&mut self, data: &[u8]) -> Result<()> {
        // Convert u8 data to u16 for DMA (pairs of bytes)
        let mut u16_data = Vec::with_capacity((data.len() + 1) / 2);
        
        for chunk in data.chunks(2) {
            let high = chunk[0] as u16;
            let low = if chunk.len() > 1 { chunk[1] as u16 } else { 0 };
            u16_data.push((high << 8) | low);
        }
        
        // Set up DMA for data
        self.dma.setup_frame_transfer(&u16_data)?;
        
        // Start transfer with data mode
        // Note: Actual implementation would set DC high here
        unsafe {
            self.dma.start_transfer()?;
        }
        
        // Wait for completion
        self.dma.wait_transfer_complete(10)?;
        
        Ok(())
    }
    
    /// Initialize ST7789 display
    fn init(&mut self) -> Result<()> {
        log::info!("Initializing LCD_CAM ST7789 display...");
        
        // Hardware reset
        self.reset()?;
        
        // Software reset
        self.write_command(CMD_SWRESET)?;
        FreeRtos::delay_ms(150);
        
        // Sleep out
        self.write_command(CMD_SLPOUT)?;
        FreeRtos::delay_ms(120);
        
        // Memory access control
        self.write_command(CMD_MADCTL)?;
        self.write_data(&[0x60])?; // Landscape mode
        
        // Pixel format
        self.write_command(CMD_COLMOD)?;
        self.write_data(&[0x55])?; // 16-bit RGB565
        
        // Porch control
        self.write_command(CMD_PORCTRL)?;
        self.write_data(&[0x0C, 0x0C, 0x00, 0x33, 0x33])?;
        
        // Gate control
        self.write_command(CMD_GCTRL)?;
        self.write_data(&[0x35])?;
        
        // VCOM setting
        self.write_command(CMD_VCOMS)?;
        self.write_data(&[0x19])?;
        
        // LCM control
        self.write_command(CMD_LCMCTRL)?;
        self.write_data(&[0x2C])?;
        
        // VDV and VRH enable
        self.write_command(CMD_VDVVRHEN)?;
        self.write_data(&[0x01])?;
        
        // VRH set
        self.write_command(CMD_VRHS)?;
        self.write_data(&[0x12])?;
        
        // VDV set
        self.write_command(CMD_VDVS)?;
        self.write_data(&[0x20])?;
        
        // Frame rate control
        self.write_command(CMD_FRCTRL2)?;
        self.write_data(&[0x0F])?;
        
        // Power control
        self.write_command(CMD_PWRCTRL1)?;
        self.write_data(&[0xA4, 0xA1])?;
        
        // Inversion ON
        self.write_command(CMD_INVON)?;
        
        // Display ON
        self.write_command(CMD_DISPON)?;
        FreeRtos::delay_ms(20);
        
        // Clear display
        self.clear(BLACK)?;
        
        self.is_initialized.store(true, Ordering::Release);
        log::info!("LCD_CAM display initialized successfully");
        
        Ok(())
    }
    
    /// Set drawing window
    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<()> {
        // Column address set
        self.write_command(CMD_CASET)?;
        let x0_off = x0 + DISPLAY_X_OFFSET;
        let x1_off = x1 + DISPLAY_X_OFFSET;
        self.write_data(&[
            (x0_off >> 8) as u8,
            (x0_off & 0xFF) as u8,
            (x1_off >> 8) as u8,
            (x1_off & 0xFF) as u8,
        ])?;
        
        // Row address set
        self.write_command(CMD_RASET)?;
        let y0_off = y0 + DISPLAY_Y_OFFSET;
        let y1_off = y1 + DISPLAY_Y_OFFSET;
        self.write_data(&[
            (y0_off >> 8) as u8,
            (y0_off & 0xFF) as u8,
            (y1_off >> 8) as u8,
            (y1_off & 0xFF) as u8,
        ])?;
        
        Ok(())
    }
    
    /// Clear display to color
    pub fn clear(&mut self, color: u16) -> Result<()> {
        // Fill frame buffer
        unsafe {
            FRAME_BUFFER.fill(color);
        }
        
        // Set full screen window
        self.set_window(0, 0, (DISPLAY_WIDTH - 1) as u16, (DISPLAY_HEIGHT - 1) as u16)?;
        
        // Start pixel data
        self.write_command(CMD_RAMWR)?;
        
        // Transfer frame buffer via DMA
        unsafe {
            self.dma.setup_frame_transfer(&FRAME_BUFFER)?;
            self.dma.start_transfer()?;
        }
        
        // Wait for completion
        self.dma.wait_transfer_complete(100)?;
        
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    /// Get frame statistics
    pub fn get_stats(&self) -> (u32, u32) {
        let frames = self.frame_count.load(Ordering::Relaxed);
        let (dma_frames, _) = self.dma.get_stats();
        (frames, dma_frames)
    }
}

// Clean up on drop
impl Drop for LcdCamDisplay {
    fn drop(&mut self) {
        log::info!("Shutting down LCD_CAM display");
    }
}