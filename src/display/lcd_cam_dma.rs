/// DMA integration for LCD_CAM peripheral
use anyhow::{Result, anyhow};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use esp_idf_sys::*;

// GDMA peripheral base
const GDMA_BASE: u32 = 0x6003_F000;

// DMA channel 0 registers (we'll use channel 0 for LCD)
const GDMA_OUT_CONF0_CH0_REG: u32 = GDMA_BASE + 0x00;
const GDMA_OUT_INT_ENA_CH0_REG: u32 = GDMA_BASE + 0x08;
const GDMA_OUT_INT_CLR_CH0_REG: u32 = GDMA_BASE + 0x0C;
const GDMA_OUT_LINK_CH0_REG: u32 = GDMA_BASE + 0x20;
const GDMA_OUT_STATE_CH0_REG: u32 = GDMA_BASE + 0x24;
const GDMA_OUT_PUSH_CH0_REG: u32 = GDMA_BASE + 0x28;
const GDMA_OUT_PERI_SEL_CH0_REG: u32 = GDMA_BASE + 0x40;

// DMA descriptor must be 16-byte aligned
#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct DmaDescriptor {
    pub size: u16,              // Buffer size (12 bits used)
    pub length: u16,            // Number of valid bytes in buffer  
    pub flags: u32,             // Control flags
    pub buffer: *const u8,      // Pointer to data
    pub next: *mut DmaDescriptor, // Next descriptor or null
}

// DMA descriptor flags
const DMA_DESC_OWNER_DMA: u32 = 1 << 31;    // 1 = DMA owns, 0 = CPU owns
const DMA_DESC_EOF: u32 = 1 << 30;          // End of frame
const DMA_DESC_SOF: u32 = 1 << 29;          // Start of frame  
const DMA_DESC_BURST_EN: u32 = 1 << 24;     // Enable burst mode

impl DmaDescriptor {
    pub const fn new() -> Self {
        Self {
            size: 0,
            length: 0,
            flags: 0,
            buffer: ptr::null(),
            next: ptr::null_mut(),
        }
    }
    
    /// Configure descriptor for a data buffer
    pub fn set_buffer(&mut self, data: &[u8], is_first: bool, is_last: bool) {
        let len = data.len().min(4095); // Max 4095 bytes per descriptor
        
        self.size = len as u16;
        self.length = len as u16;
        self.buffer = data.as_ptr();
        
        self.flags = DMA_DESC_OWNER_DMA | DMA_DESC_BURST_EN;
        
        if is_first {
            self.flags |= DMA_DESC_SOF;
        }
        if is_last {
            self.flags |= DMA_DESC_EOF;
            self.next = ptr::null_mut();
        }
    }
    
    /// Link to next descriptor
    pub fn set_next(&mut self, next: &mut DmaDescriptor) {
        self.next = next as *mut _;
    }
}

pub struct LcdCamDma {
    descriptors: Box<[DmaDescriptor; 8]>,
    current_desc: usize,
    transfer_active: AtomicBool,
    frames_completed: AtomicU32,
}

impl LcdCamDma {
    pub unsafe fn new() -> Result<Self> {
        // Allocate descriptors (Box ensures heap allocation)
        let descriptors = Box::new([DmaDescriptor::new(); 8]);
        
        // Enable GDMA clock
        const SYSTEM_PERIP_CLK_EN0_REG: u32 = 0x600C_0020;
        const GDMA_CLK_EN: u32 = 1 << 4;
        
        let perip_clk_en0 = SYSTEM_PERIP_CLK_EN0_REG as *mut u32;
        let current = perip_clk_en0.read_volatile();
        perip_clk_en0.write_volatile(current | GDMA_CLK_EN);
        
        // Configure DMA channel 0 for LCD peripheral
        const LCD_PERI_SEL: u32 = 5; // LCD peripheral selection value
        let peri_sel_reg = GDMA_OUT_PERI_SEL_CH0_REG as *mut u32;
        peri_sel_reg.write_volatile(LCD_PERI_SEL);
        
        // Enable DMA channel
        let conf0_reg = GDMA_OUT_CONF0_CH0_REG as *mut u32;
        conf0_reg.write_volatile(
            (1 << 4) |  // OUT_DATA_BURST_EN
            (1 << 3) |  // OUT_AUTO_WRBACK  
            (1 << 0)    // OUT_RST
        );
        
        // Clear reset
        esp_rom_delay_us(10);
        let current = conf0_reg.read_volatile();
        conf0_reg.write_volatile(current & !(1 << 0));
        
        Ok(Self {
            descriptors,
            current_desc: 0,
            transfer_active: AtomicBool::new(false),
            frames_completed: AtomicU32::new(0),
        })
    }
    
    /// Set up DMA descriptors for a frame buffer
    pub fn setup_frame_transfer(&mut self, frame_data: &[u16]) -> Result<()> {
        if self.transfer_active.load(Ordering::Acquire) {
            return Err(anyhow!("DMA transfer already active"));
        }
        
        // Convert u16 slice to u8 for DMA
        let byte_data = unsafe {
            core::slice::from_raw_parts(
                frame_data.as_ptr() as *const u8,
                frame_data.len() * 2
            )
        };
        
        // Calculate number of descriptors needed
        const MAX_BYTES_PER_DESC: usize = 4092; // Leave some margin
        let num_descriptors = (byte_data.len() + MAX_BYTES_PER_DESC - 1) / MAX_BYTES_PER_DESC;
        
        if num_descriptors > self.descriptors.len() {
            return Err(anyhow!("Frame too large for descriptor chain"));
        }
        
        // Set up descriptor chain
        let mut offset = 0;
        for i in 0..num_descriptors {
            let chunk_size = (byte_data.len() - offset).min(MAX_BYTES_PER_DESC);
            let chunk = &byte_data[offset..offset + chunk_size];
            
            let is_first = i == 0;
            let is_last = i == num_descriptors - 1;
            
            self.descriptors[i].set_buffer(chunk, is_first, is_last);
            
            if !is_last {
                let next_ptr = &mut self.descriptors[i + 1] as *mut DmaDescriptor;
                self.descriptors[i].next = next_ptr;
            }
            
            offset += chunk_size;
        }
        
        self.current_desc = 0;
        Ok(())
    }
    
    /// Start DMA transfer
    pub unsafe fn start_transfer(&self) -> Result<()> {
        if self.transfer_active.swap(true, Ordering::AcqRel) {
            return Err(anyhow!("Transfer already in progress"));
        }
        
        // Set descriptor link address
        let link_reg = GDMA_OUT_LINK_CH0_REG as *mut u32;
        let desc_addr = &self.descriptors[0] as *const _ as u32;
        
        // Bit 0 = start, bits[31:1] = address >> 1
        link_reg.write_volatile((desc_addr & !0x1) | 1);
        
        // Enable completion interrupt
        let int_ena_reg = GDMA_OUT_INT_ENA_CH0_REG as *mut u32;
        int_ena_reg.write_volatile(1 << 0); // OUT_DONE interrupt
        
        // Start DMA
        let push_reg = GDMA_OUT_PUSH_CH0_REG as *mut u32;
        push_reg.write_volatile(1 << 0);
        
        Ok(())
    }
    
    /// Check if transfer is complete
    pub fn is_transfer_complete(&self) -> bool {
        unsafe {
            let state_reg = GDMA_OUT_STATE_CH0_REG as *mut u32;
            let state = state_reg.read_volatile();
            
            // Check if DMA is idle (bits [22:20] == 0)
            let dma_state = (state >> 20) & 0x7;
            dma_state == 0
        }
    }
    
    /// Wait for transfer completion
    pub fn wait_transfer_complete(&self, timeout_ms: u32) -> Result<()> {
        let start = unsafe { esp_timer_get_time() };
        let timeout_us = timeout_ms as i64 * 1000;
        
        while self.transfer_active.load(Ordering::Acquire) {
            if self.is_transfer_complete() {
                self.transfer_active.store(false, Ordering::Release);
                self.frames_completed.fetch_add(1, Ordering::Relaxed);
                
                // Clear interrupt
                unsafe {
                    let int_clr_reg = GDMA_OUT_INT_CLR_CH0_REG as *mut u32;
                    int_clr_reg.write_volatile(1 << 0);
                }
                
                return Ok(());
            }
            
            let elapsed = unsafe { esp_timer_get_time() } - start;
            if elapsed > timeout_us {
                return Err(anyhow!("DMA transfer timeout"));
            }
            
            // Yield to other tasks
            unsafe { vTaskDelay(1); }
        }
        
        Ok(())
    }
    
    /// Get transfer statistics
    pub fn get_stats(&self) -> (u32, bool) {
        (
            self.frames_completed.load(Ordering::Relaxed),
            self.transfer_active.load(Ordering::Acquire)
        )
    }
}