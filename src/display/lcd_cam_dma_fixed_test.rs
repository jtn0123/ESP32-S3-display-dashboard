/// LCD_CAM with DMA implementation based on fault tree and ESP-IDF
/// This implements the full DMA chain for continuous display updates
use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_sys::*;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};
use core::mem;

// LCD_CAM registers
const DR_REG_LCD_CAM_BASE: u32 = 0x6004_1000;
const DR_REG_SYSTEM_BASE: u32 = 0x600C_0000;
const DR_REG_GDMA_BASE: u32 = 0x6003_F000;

// System registers
const SYSTEM_PERIP_CLK_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x24;
const SYSTEM_PERIP_RST_EN1_REG: u32 = DR_REG_SYSTEM_BASE + 0x28;

// LCD_CAM registers
const LCD_CAM_LCD_CLOCK_REG: u32 = DR_REG_LCD_CAM_BASE + 0x00;
const LCD_CAM_LCD_USER_REG: u32 = DR_REG_LCD_CAM_BASE + 0x04;
const LCD_CAM_LCD_MISC_REG: u32 = DR_REG_LCD_CAM_BASE + 0x08;
const LCD_CAM_LCD_CTRL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x0C;
const LCD_CAM_LCD_CTRL1_REG: u32 = DR_REG_LCD_CAM_BASE + 0x10;
const LCD_CAM_LCD_CTRL2_REG: u32 = DR_REG_LCD_CAM_BASE + 0x14;
const LCD_CAM_LCD_CMD_VAL_REG: u32 = DR_REG_LCD_CAM_BASE + 0x18;
const LCD_CAM_LCD_DSCR_ADDR_REG: u32 = DR_REG_LCD_CAM_BASE + 0x1C;
const LCD_CAM_LCD_DLY_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x30;
const LCD_CAM_LCD_DATA_DOUT_MODE_REG: u32 = DR_REG_LCD_CAM_BASE + 0x34;

// GDMA registers (channel 0)
const GDMA_CH0_IN_CONF0_REG: u32 = DR_REG_GDMA_BASE + 0x0;
const GDMA_CH0_IN_CONF1_REG: u32 = DR_REG_GDMA_BASE + 0x4;
const GDMA_CH0_IN_LINK_REG: u32 = DR_REG_GDMA_BASE + 0xC;
const GDMA_CH0_IN_PRI_REG: u32 = DR_REG_GDMA_BASE + 0x18;
const GDMA_CH0_IN_PERI_SEL_REG: u32 = DR_REG_GDMA_BASE + 0x1C;

// Clock register bits
const LCD_CLK_EN: u32 = 1 << 31;
const LCD_CLKM_DIV_NUM_SHIFT: u32 = 9;

// User register bits
const LCD_START: u32 = 1 << 27;
const LCD_CMD: u32 = 1 << 26;
const LCD_DOUT: u32 = 1 << 24;
const LCD_8BITS_ORDER: u32 = 1 << 23;
const LCD_UPDATE: u32 = 1 << 20;

// MISC register bits
const LCD_CD_CMD_SET: u32 = 1 << 30;
const LCD_CD_DUMMY_SET: u32 = 1 << 29;
const LCD_CD_DATA_SET: u32 = 1 << 28;
const LCD_AFIFO_ADDR_BRIDGE_EN: u32 = 1 << 26;
const LCD_AFIFO_RESET: u32 = 1 << 25;

// CTRL register bits
const LCD_TX_FIFO_RST: u32 = 1 << 13;
const LCD_TX_FIFO_MOD_SHIFT: u32 = 10;
const LCD_DMA_TX_EN: u32 = 1 << 0;  // Enable DMA TX

// CTRL1 register bits
const LCD_DOUT_EN: u32 = 1 << 0;

// DMA descriptor structure
#[repr(C, align(4))]
struct DmaDescriptor {
    size: u32,      // Buffer size and control bits
    length: u32,    // Buffer length
    buf: *const u8, // Buffer pointer
    next: *mut DmaDescriptor, // Next descriptor
}

impl DmaDescriptor {
    const OWNER_DMA: u32 = 1 << 31;
    const EOF: u32 = 1 << 30;
    const SIZE_MASK: u32 = 0xFFF;
    
    fn new() -> Self {
        Self {
            size: 0,
            length: 0,
            buf: core::ptr::null(),
            next: core::ptr::null_mut(),
        }
    }
    
    fn setup(&mut self, buffer: &[u8], is_last: bool) {
        self.size = (buffer.len() as u32 & Self::SIZE_MASK) | Self::OWNER_DMA;
        if is_last {
            self.size |= Self::EOF;
        }
        self.length = buffer.len() as u32;
        self.buf = buffer.as_ptr();
        self.next = if is_last { core::ptr::null_mut() } else { self as *mut _ };
    }
}

// Safe register access
#[inline(always)]
unsafe fn reg_read(addr: u32) -> u32 {
    compiler_fence(Ordering::SeqCst);
    let val = read_volatile(addr as *const u32);
    compiler_fence(Ordering::SeqCst);
    val
}

#[inline(always)]
unsafe fn reg_write(addr: u32, val: u32) {
    compiler_fence(Ordering::SeqCst);
    write_volatile(addr as *mut u32, val);
    compiler_fence(Ordering::SeqCst);
}

#[inline(always)]
unsafe fn reg_set_bits(addr: u32, bits: u32) {
    let val = reg_read(addr);
    reg_write(addr, val | bits);
}

#[inline(always)]
unsafe fn reg_clear_bits(addr: u32, bits: u32) {
    let val = reg_read(addr);
    reg_write(addr, val & !bits);
}

pub fn lcd_cam_dma_fixed_test(
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
    log::warn!("=== LCD_CAM DMA Fixed Implementation Test ===");
    
    // Create test framebuffer (small for testing)
    const FB_WIDTH: usize = 320;
    const FB_HEIGHT: usize = 10;  // Just 10 lines for testing
    let mut framebuffer: Vec<u8> = vec![0; FB_WIDTH * FB_HEIGHT * 2];  // RGB565
    
    // Fill with test pattern
    for y in 0..FB_HEIGHT {
        for x in 0..FB_WIDTH {
            let idx = (y * FB_WIDTH + x) * 2;
            // Gradient pattern
            let color = ((x as u16 & 0x1F) << 11) | ((y as u16 & 0x3F) << 5) | ((x as u16 >> 3) & 0x1F);
            framebuffer[idx] = (color >> 8) as u8;
            framebuffer[idx + 1] = color as u8;
        }
    }
    
    // Create DMA descriptor
    let mut descriptor = Box::new(DmaDescriptor::new());
    descriptor.setup(&framebuffer, true);
    
    // Power and backlight pins
    unsafe {
        esp_rom_gpio_pad_select_gpio(15);
        gpio_set_direction(15 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(15 as gpio_num_t, 1);
        
        esp_rom_gpio_pad_select_gpio(38);
        gpio_set_direction(38 as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(38 as gpio_num_t, 1);
    }
    
    // Step 1: Enable peripherals
    unsafe {
        log::info!("Step 1: Enable LCD_CAM and GDMA");
        // LCD_CAM
        reg_set_bits(SYSTEM_PERIP_CLK_EN1_REG, 1 << 31);
        reg_clear_bits(SYSTEM_PERIP_RST_EN1_REG, 1 << 31);
        // GDMA
        reg_set_bits(SYSTEM_PERIP_CLK_EN1_REG, 1 << 0);
        reg_clear_bits(SYSTEM_PERIP_RST_EN1_REG, 1 << 0);
        esp_rom_delay_us(100);
    }
    
    // Step 2: Configure GPIO Matrix
    unsafe {
        log::info!("Step 2: Configure GPIO Matrix");
        
        const DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48];
        const WR_PIN: u8 = 8;
        const DC_PIN: u8 = 7;
        const CS_PIN: u8 = 6;
        const RST_PIN: u8 = 5;
        
        // Configure all pins
        for (i, &pin) in DATA_PINS.iter().enumerate() {
            esp_rom_gpio_pad_select_gpio(pin as u32);
            gpio_set_direction(pin as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
            gpio_set_drive_capability(pin as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
            esp_rom_gpio_connect_out_signal(pin as u32, 133 + i as u32, false, false);
        }
        
        esp_rom_gpio_pad_select_gpio(WR_PIN as u32);
        gpio_set_direction(WR_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_drive_capability(WR_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(WR_PIN as u32, 154, false, false);
        
        esp_rom_gpio_pad_select_gpio(DC_PIN as u32);
        gpio_set_direction(DC_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_drive_capability(DC_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(DC_PIN as u32, 153, false, false);
        
        esp_rom_gpio_pad_select_gpio(CS_PIN as u32);
        gpio_set_direction(CS_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_drive_capability(CS_PIN as gpio_num_t, gpio_drive_cap_t_GPIO_DRIVE_CAP_3);
        esp_rom_gpio_connect_out_signal(CS_PIN as u32, 132, false, false);
        
        esp_rom_gpio_pad_select_gpio(RST_PIN as u32);
        gpio_set_direction(RST_PIN as gpio_num_t, GPIO_MODE_DEF_OUTPUT);
        gpio_set_level(RST_PIN as gpio_num_t, 1);
    }
    
    // Step 3: Configure LCD_CAM
    unsafe {
        log::info!("Step 3: Configure LCD_CAM peripheral");
        
        // Clock: 10MHz
        reg_write(LCD_CAM_LCD_CLOCK_REG, LCD_CLK_EN | (15 << LCD_CLKM_DIV_NUM_SHIFT));
        
        // MISC: Enable all output modes and AFIFO bridge
        reg_write(LCD_CAM_LCD_MISC_REG, 
                  LCD_CD_CMD_SET | LCD_CD_DATA_SET | LCD_CD_DUMMY_SET | LCD_AFIFO_ADDR_BRIDGE_EN);
        
        // USER: 8-bit output mode
        reg_write(LCD_CAM_LCD_USER_REG, LCD_DOUT | LCD_8BITS_ORDER);
        
        // Reset FIFOs
        reg_set_bits(LCD_CAM_LCD_CTRL_REG, LCD_TX_FIFO_RST | LCD_AFIFO_RESET);
        esp_rom_delay_us(10);
        reg_clear_bits(LCD_CAM_LCD_CTRL_REG, LCD_TX_FIFO_RST | LCD_AFIFO_RESET);
        
        // Set FIFO mode and enable DMA
        let ctrl_val = (1 << LCD_TX_FIFO_MOD_SHIFT) | LCD_DMA_TX_EN;
        reg_write(LCD_CAM_LCD_CTRL_REG, ctrl_val);
        
        // Enable output
        reg_write(LCD_CAM_LCD_CTRL1_REG, LCD_DOUT_EN);
        
        // Data output mode
        reg_write(LCD_CAM_LCD_DATA_DOUT_MODE_REG, 0xFF);
        
        // Apply update
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
        esp_rom_delay_us(10);
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_UPDATE);
    }
    
    // Step 4: Configure GDMA
    unsafe {
        log::info!("Step 4: Configure GDMA channel 0");
        
        // Reset GDMA channel
        reg_write(GDMA_CH0_IN_CONF0_REG, 1 << 3);  // IN_RST
        esp_rom_delay_us(10);
        reg_write(GDMA_CH0_IN_CONF0_REG, 0);
        
        // Configure GDMA
        reg_write(GDMA_CH0_IN_CONF1_REG, 0);  // No burst, no check owner
        reg_write(GDMA_CH0_IN_PRI_REG, 0);    // Priority 0
        reg_write(GDMA_CH0_IN_PERI_SEL_REG, 5);  // LCD_CAM peripheral (value from ESP-IDF)
        
        // Set descriptor address
        let desc_addr = descriptor.as_ref() as *const _ as u32;
        reg_write(LCD_CAM_LCD_DSCR_ADDR_REG, desc_addr);
        log::info!("DMA descriptor at 0x{:08x}", desc_addr);
        
        // Start GDMA
        reg_write(GDMA_CH0_IN_LINK_REG, (desc_addr & 0xFFFFF) | (1 << 20));  // INLINK_START
    }
    
    // Reset watchdog
    unsafe { esp_task_wdt_reset(); }
    
    // Step 5: Start DMA transfer
    log::info!("Step 5: Starting DMA transfer");
    unsafe {
        // Clear data mode (for testing)
        reg_clear_bits(LCD_CAM_LCD_USER_REG, LCD_CMD);
        
        // Start transfer
        reg_set_bits(LCD_CAM_LCD_USER_REG, LCD_START);
        
        // Poll descriptor ownership
        let mut timeout = 100000;
        let desc_ptr = descriptor.as_ref() as *const DmaDescriptor;
        while timeout > 0 {
            compiler_fence(Ordering::SeqCst);
            let size = read_volatile(&(*desc_ptr).size);
            compiler_fence(Ordering::SeqCst);
            
            if (size & DmaDescriptor::OWNER_DMA) == 0 {
                log::info!("DMA transfer complete! Descriptor ownership returned to CPU");
                break;
            }
            
            timeout -= 1;
            esp_rom_delay_us(10);
            
            if timeout % 1000 == 0 {
                log::debug!("Waiting for DMA... timeout={}, desc.size=0x{:08x}", timeout, size);
            }
        }
        
        if timeout == 0 {
            log::error!("DMA transfer timeout!");
        } else {
            log::info!("Transfer took approximately {} us", (100000 - timeout) * 10);
        }
        
        // Check if START bit cleared
        let user_val = reg_read(LCD_CAM_LCD_USER_REG);
        log::info!("LCD_USER after transfer: 0x{:08x}, START bit: {}", 
                  user_val, if (user_val & LCD_START) != 0 { "SET" } else { "CLEARED" });
    }
    
    // Dump final register state
    unsafe {
        log::info!("Final register state:");
        log::info!("  CLOCK:     0x{:08x}", reg_read(LCD_CAM_LCD_CLOCK_REG));
        log::info!("  USER:      0x{:08x}", reg_read(LCD_CAM_LCD_USER_REG));
        log::info!("  MISC:      0x{:08x}", reg_read(LCD_CAM_LCD_MISC_REG));
        log::info!("  CTRL:      0x{:08x}", reg_read(LCD_CAM_LCD_CTRL_REG));
        log::info!("  CTRL1:     0x{:08x}", reg_read(LCD_CAM_LCD_CTRL1_REG));
        log::info!("  DSCR_ADDR: 0x{:08x}", reg_read(LCD_CAM_LCD_DSCR_ADDR_REG));
    }
    
    log::warn!("DMA test complete!");
    log::warn!("Expected behavior:");
    log::warn!("  - WR should pulse {} times", FB_WIDTH * FB_HEIGHT);
    log::warn!("  - Data should show gradient pattern");
    log::warn!("  - Transfer should complete in ~3ms at 10MHz");
    
    Ok(())
}