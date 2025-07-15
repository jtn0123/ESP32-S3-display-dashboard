// LCD_CAM peripheral driver for ESP32-S3
// Provides DMA-accelerated parallel LCD interface

use esp_idf_sys::*;
use core::ptr;
use log::*;

const FRAMEBUFFER_SIZE: usize = 320 * 170;
const DMA_DESCRIPTOR_COUNT: usize = 32;
const DMA_MAX_SIZE: usize = 4092; // 4095 - 3 bytes for config

#[repr(C, align(4))]
struct DmaDescriptor {
    config: u32,
    buffer: *const u8,
    next: *mut DmaDescriptor,
}

pub struct LcdCam {
    framebuffer: &'static mut [u16; FRAMEBUFFER_SIZE],
    descriptors: &'static mut [DmaDescriptor; DMA_DESCRIPTOR_COUNT],
}

impl LcdCam {
    pub unsafe fn new() -> Result<Self, EspError> {
        info!("Initializing LCD_CAM peripheral");
        
        // Allocate framebuffer in DMA-capable memory
        let framebuffer = Self::alloc_framebuffer()?;
        let descriptors = Self::alloc_descriptors()?;
        
        // Enable LCD_CAM peripheral clock
        (*PCR::ptr()).lcd_cam_conf.modify(|_, w| {
            w.lcd_cam_clk_en().set_bit()
             .lcd_cam_rst_en().clear_bit()
        });
        
        // Reset LCD_CAM
        (*LCD_CAM::ptr()).lcd_user.write(|w| w.bits(0));
        
        // Configure LCD controller
        (*LCD_CAM::ptr()).lcd_user.modify(|_, w| {
            w.lcd_dout().set_bit()           // Enable data output
             .lcd_8bits_order().clear_bit()  // Normal byte order
             .lcd_bit_order().clear_bit()    // MSB first
             .lcd_byte_order().clear_bit()   // High byte first
             .lcd_2byte_mode().set_bit()     // 16-bit RGB565 mode
        });
        
        // Configure clock (20MHz from 160MHz PLL)
        (*LCD_CAM::ptr()).lcd_clock.write(|w| {
            w.lcd_clk_sel().bits(2)          // Select PLL_F160M
             .lcd_clkm_div_num().bits(8)     // Divide by 8 = 20MHz
             .lcd_clkm_div_a().bits(0)
             .lcd_clkm_div_b().bits(0)
             .lcd_ck_out_edge().clear_bit()  // Data valid on falling edge
             .lcd_ck_idle_edge().clear_bit()
             .lcd_clk_equ_sysclk().clear_bit()
        });
        
        // Configure data pins (D0-D7 on GPIO 39-48)
        Self::configure_data_pins()?;
        
        // Configure WR pin (GPIO 8)
        Self::configure_wr_pin()?;
        
        let mut lcd_cam = LcdCam {
            framebuffer,
            descriptors,
        };
        
        // Set up DMA descriptor chain
        lcd_cam.setup_descriptors();
        
        // Configure DMA
        (*LCD_CAM::ptr()).lcd_dma_conf.modify(|_, w| {
            w.lcd_dma_out_eof_mode().set_bit()  // Generate EOF after descriptor
        });
        
        info!("LCD_CAM initialized successfully");
        
        Ok(lcd_cam)
    }
    
    unsafe fn alloc_framebuffer() -> Result<&'static mut [u16; FRAMEBUFFER_SIZE], EspError> {
        // Allocate in DMA-capable memory (must be 4-byte aligned)
        let size = FRAMEBUFFER_SIZE * 2; // 2 bytes per pixel
        let ptr = heap_caps_malloc(size, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL) as *mut u16;
        
        if ptr.is_null() {
            error!("Failed to allocate framebuffer");
            return Err(EspError::from(ESP_ERR_NO_MEM).unwrap());
        }
        
        // Clear framebuffer
        ptr::write_bytes(ptr, 0, FRAMEBUFFER_SIZE);
        
        Ok(&mut *(ptr as *mut [u16; FRAMEBUFFER_SIZE]))
    }
    
    unsafe fn alloc_descriptors() -> Result<&'static mut [DmaDescriptor; DMA_DESCRIPTOR_COUNT], EspError> {
        let size = core::mem::size_of::<[DmaDescriptor; DMA_DESCRIPTOR_COUNT]>();
        let ptr = heap_caps_malloc(size, MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL) as *mut DmaDescriptor;
        
        if ptr.is_null() {
            error!("Failed to allocate DMA descriptors");
            return Err(EspError::from(ESP_ERR_NO_MEM).unwrap());
        }
        
        // Clear descriptors
        ptr::write_bytes(ptr, 0, DMA_DESCRIPTOR_COUNT);
        
        Ok(&mut *(ptr as *mut [DmaDescriptor; DMA_DESCRIPTOR_COUNT]))
    }
    
    fn setup_descriptors(&mut self) {
        let total_bytes = FRAMEBUFFER_SIZE * 2; // 2 bytes per pixel
        let fb_ptr = self.framebuffer.as_ptr() as *const u8;
        
        let mut offset = 0;
        let mut desc_idx = 0;
        
        while offset < total_bytes && desc_idx < DMA_DESCRIPTOR_COUNT {
            let remaining = total_bytes - offset;
            let chunk_size = remaining.min(DMA_MAX_SIZE);
            
            self.descriptors[desc_idx] = DmaDescriptor {
                // [31:24] Reserved
                // [23:16] Buffer length[15:8]
                // [15:12] Buffer length[19:16]
                // [11]    Reserved
                // [10]    sosf - start of sub-frame (set on first descriptor)
                // [9]     Reserved  
                // [8]     owner - 1: DMA, 0: CPU
                // [7:0]   Buffer length[7:0]
                config: (chunk_size as u32) | (1 << 8) | if desc_idx == 0 { 1 << 10 } else { 0 },
                buffer: unsafe { fb_ptr.add(offset) },
                next: if desc_idx < DMA_DESCRIPTOR_COUNT - 1 && offset + chunk_size < total_bytes {
                    &mut self.descriptors[desc_idx + 1] as *mut _
                } else {
                    ptr::null_mut() // End of chain
                },
            };
            
            offset += chunk_size;
            desc_idx += 1;
        }
        
        info!("Set up {} DMA descriptors for {} bytes", desc_idx, total_bytes);
    }
    
    unsafe fn configure_data_pins() -> Result<(), EspError> {
        // Data pins D0-D7 mapped to GPIO 39, 40, 41, 42, 45, 46, 47, 48
        const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
        
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            // Configure pin as output
            gpio_set_direction(pin as i32, GPIO_MODE_OUTPUT);
            
            // Connect to LCD_CAM peripheral via GPIO matrix
            gpio_matrix_out(pin as u32, LCD_DATA_OUT0_IDX + i as u32, false, false);
        }
        
        Ok(())
    }
    
    unsafe fn configure_wr_pin() -> Result<(), EspError> {
        const WR_PIN: u8 = 8;
        
        // Configure as output
        gpio_set_direction(WR_PIN as i32, GPIO_MODE_OUTPUT);
        
        // Connect to LCD_CAM PCLK signal
        gpio_matrix_out(WR_PIN as u32, LCD_PCLK_IDX, false, false);
        
        Ok(())
    }
    
    pub fn start_transfer(&mut self) {
        unsafe {
            // Clear interrupts
            (*LCD_CAM::ptr()).lcd_dma_int_clr.write(|w| w.bits(0xFFFFFFFF));
            
            // Reset FIFO
            (*LCD_CAM::ptr()).lcd_misc.modify(|_, w| w.lcd_afifo_reset().set_bit());
            (*LCD_CAM::ptr()).lcd_misc.modify(|_, w| w.lcd_afifo_reset().clear_bit());
            
            // Set descriptor address
            (*LCD_CAM::ptr()).lcd_dma_out_link.write(|w| {
                w.outlink_addr().bits(self.descriptors.as_ptr() as u32)
                 .outlink_start().set_bit()
            });
            
            // Start transfer
            (*LCD_CAM::ptr()).lcd_user.modify(|_, w| w.lcd_start().set_bit());
        }
    }
    
    pub fn wait_transfer_done(&self) {
        unsafe {
            // Wait for transfer complete interrupt
            while (*LCD_CAM::ptr()).lcd_dma_int_raw.read().lcd_trans_done_int_raw().bit_is_clear() {
                // Could yield here in async context
            }
            
            // Clear interrupt
            (*LCD_CAM::ptr()).lcd_dma_int_clr.write(|w| w.lcd_trans_done_int_clr().set_bit());
        }
    }
    
    pub fn get_framebuffer(&self) -> &[u16; FRAMEBUFFER_SIZE] {
        self.framebuffer
    }
    
    pub fn get_framebuffer_mut(&mut self) -> &mut [u16; FRAMEBUFFER_SIZE] {
        self.framebuffer
    }
}

// Manual Drop implementation to free DMA memory
impl Drop for LcdCam {
    fn drop(&mut self) {
        unsafe {
            // Disable LCD_CAM
            (*LCD_CAM::ptr()).lcd_user.write(|w| w.bits(0));
            
            // Free allocated memory
            heap_caps_free(self.framebuffer.as_mut_ptr() as *mut _);
            heap_caps_free(self.descriptors.as_mut_ptr() as *mut _);
        }
    }
}