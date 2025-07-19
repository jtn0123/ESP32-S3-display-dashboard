# LCD_CAM Implementation Plan for ESP32-S3 T-Display

## Overview
This document outlines the implementation of hardware LCD_CAM peripheral for the ST7789 display, replacing GPIO bit-banging with proper hardware-controlled timing.

## Why LCD_CAM?
- **Hardware timing control**: Eliminates all timing violations
- **True DMA**: CPU completely free during transfers  
- **Theoretical max**: 240 FPS at 40MHz APB (2 pixels/cycle)
- **Proper synchronization**: Hardware-managed CS/DC/WR signals

## Hardware Configuration

### Pin Mapping (T-Display-S3)
```
LCD_CAM Data pins:
- D0: GPIO 39 → LCD_DATA_OUT[0]
- D1: GPIO 40 → LCD_DATA_OUT[1]  
- D2: GPIO 41 → LCD_DATA_OUT[2]
- D3: GPIO 42 → LCD_DATA_OUT[3]
- D4: GPIO 45 → LCD_DATA_OUT[4]
- D5: GPIO 46 → LCD_DATA_OUT[5]
- D6: GPIO 47 → LCD_DATA_OUT[6]
- D7: GPIO 48 → LCD_DATA_OUT[7]

Control pins:
- WR: GPIO 8  → LCD_PCLK
- DC: GPIO 7  → LCD_DC  
- CS: GPIO 6  → LCD_CS
- RST: GPIO 5 → (GPIO control)
```

### LCD_CAM Register Configuration
```rust
// Key registers we'll need
const LCD_CAM_BASE: u32 = 0x6004_1000;

// Clock configuration
const LCD_CAM_LCD_CLOCK_REG: u32 = LCD_CAM_BASE + 0x0;
// [31:22] LCD_CLKM_DIV_NUM - Clock divider
// [21:20] LCD_CLKM_DIV_B
// [19:14] LCD_CLKM_DIV_A  
// [12:11] LCD_CLK_SEL - Clock source select
// [10] LCD_CLK_EQU_SYSCLK
// [9] LCD_CK_IDLE_EDGE
// [8] LCD_CK_OUT_EDGE
// [6] LCD_CLKM_DIV_NUM_WE
// [5] LCD_CLKM_DIV_B_WE
// [4] LCD_CLKM_DIV_A_WE
// [3] LCD_CLK_SEL_WE
// [1] LCD_CLK_EN - Enable LCD clock

// User configuration
const LCD_CAM_LCD_USER_REG: u32 = LCD_CAM_BASE + 0x4;
// [31] LCD_RESET - Software reset
// [30] LCD_DUMMY_CYCLELEN_WE
// [29:21] LCD_DUMMY_CYCLELEN
// [20] LCD_CMD - Start command
// [19] LCD_UPDATE - Start update
// [17] LCD_START_SEL
// [16] LCD_BYTE_MODE
// [15] LCD_BIT_ORDER
// [14] LCD_VT_HEIGHT_WE  
// [13:0] LCD_VT_HEIGHT

// DMA configuration  
const LCD_CAM_LCD_DLY_MODE_REG: u32 = LCD_CAM_BASE + 0x30;
const LCD_CAM_LCD_DMA_MODE_REG: u32 = LCD_CAM_BASE + 0x34;
```

## Implementation Phases

### Phase 1: Basic LCD_CAM Setup (Days 1-3)
```rust
// 1. Create safe Rust bindings
mod lcd_cam_ll {
    use esp_idf_sys::*;
    
    pub struct LcdCam {
        // Phantom data to ensure !Send/!Sync
        _private: PhantomData<*const ()>,
    }
    
    impl LcdCam {
        pub fn new() -> Result<Self> {
            // Enable peripheral clock
            unsafe {
                (*SYSTEM::ptr()).perip_clk_en1.modify(|_, w| {
                    w.lcd_cam_clk_en().set_bit()
                });
                (*SYSTEM::ptr()).perip_rst_en1.modify(|_, w| {
                    w.lcd_cam_rst().clear_bit()  
                });
            }
            
            Ok(Self { _private: PhantomData })
        }
        
        pub fn configure_i8080_8bit(&mut self) -> Result<()> {
            // Configure for 8-bit i8080 mode
            // Set up timing parameters
            // Configure DMA mode
        }
    }
}
```

### Phase 2: DMA Descriptor Setup (Days 3-5)
```rust
#[repr(C, align(16))]
struct DmaDescriptor {
    flags: u32,           // [31]: owner, [30]: eof, [23:12]: length
    buffer: *const u8,    // Data buffer pointer  
    next: *mut DmaDescriptor, // Next descriptor or null
}

impl DmaDescriptor {
    const OWNER_DMA: u32 = 1 << 31;
    const EOF: u32 = 1 << 30;
    
    fn set_buffer(&mut self, data: &[u8], is_last: bool) {
        self.flags = Self::OWNER_DMA 
            | ((data.len() as u32 & 0xFFF) << 12)
            | if is_last { Self::EOF } else { 0 };
        self.buffer = data.as_ptr();
    }
}
```

### Phase 3: Timing Configuration (Days 5-7)  
```rust
// ST7789 timing requirements
const WR_CYCLE_NS: u32 = 66;    // 15MHz max
const WR_HIGH_NS: u32 = 40;     // Min high time
const WR_LOW_NS: u32 = 20;      // Min low time  
const CS_SETUP_NS: u32 = 10;    // CS to WR setup
const DC_SETUP_NS: u32 = 10;    // DC to WR setup

fn calculate_lcd_cam_timing(apb_freq_hz: u32) -> TimingConfig {
    let apb_period_ns = 1_000_000_000 / apb_freq_hz;
    
    TimingConfig {
        clk_div: max(1, WR_CYCLE_NS / apb_period_ns),
        dc_setup_cycles: max(1, DC_SETUP_NS / apb_period_ns),
        cs_setup_cycles: max(1, CS_SETUP_NS / apb_period_ns),
        // ... etc
    }
}
```

### Phase 4: Integration (Days 7-10)
```rust
pub struct LcdCamDisplay {
    lcd_cam: LcdCam,
    dma_channel: gdma::Channel0,
    descriptors: Pin<Box<[DmaDescriptor; 8]>>,
    frame_buffer: Pin<Box<[u16; 300 * 168]>>,
    current_desc: usize,
}

impl LcdCamDisplay {
    pub fn new(pins: DisplayPins) -> Result<Self> {
        // 1. Configure GPIO matrix for LCD_CAM
        // 2. Set up LCD_CAM peripheral
        // 3. Configure GDMA channel
        // 4. Link descriptors
        // 5. Enable interrupts
    }
    
    pub fn write_frame(&mut self, data: &[u16]) -> Result<()> {
        // 1. Set up descriptors for data
        // 2. Start DMA transfer
        // 3. Wait for completion (or async)
    }
}
```

## Debug Infrastructure

### 1. Toggle Color Test
```rust
// First test - no UI, just color toggle
pub fn lcd_cam_color_test(display: &mut LcdCamDisplay) -> Result<()> {
    const RED: u16 = 0xF800;
    const CYAN: u16 = 0x07FF;
    
    loop {
        // Fill frame buffer with red
        display.frame_buffer.fill(RED);
        display.write_frame(&display.frame_buffer)?;
        FreeRtos::delay_ms(16);
        
        // Fill frame buffer with cyan
        display.frame_buffer.fill(CYAN);
        display.write_frame(&display.frame_buffer)?;
        FreeRtos::delay_ms(16);
        
        // If this runs stable, LCD_CAM timing is good
    }
}
```

### 2. Logic Analyzer Points
```rust
// Add test points for LA
#[cfg(feature = "la_debug")]
pub fn setup_debug_pins() {
    // TP1: Frame start signal
    // TP2: DMA complete IRQ
    // TP3: Buffer swap event
    // TP4: CS signal (mirrored)
}
```

### 3. Performance Metrics
```rust
struct LcdMetrics {
    frames_sent: AtomicU32,
    dma_errors: AtomicU32,
    max_frame_time_us: AtomicU32,
    last_fps: AtomicU32,
}
```

## Risk Mitigation

1. **Fallback Path**: Keep current GPIO implementation as fallback
2. **Incremental Testing**: Each phase has standalone validation
3. **Known Good Reference**: Compare against ESP-IDF C implementation
4. **Early Performance Validation**: Toggle test proves timing before UI integration

## Success Criteria

- [ ] Toggle color test runs 24 hours without glitch
- [ ] Logic analyzer shows WR > 40ns, proper CS/DC timing  
- [ ] Achieve stable 120+ FPS with full-screen updates
- [ ] Zero CPU usage during frame transfer (verified by FreeRTOS stats)
- [ ] Clean integration with existing display API

## References
- ESP32-S3 TRM Chapter 22: LCD_CAM Controller
- ESP-IDF: components/hal/esp32s3/include/hal/lcd_ll.h
- ST7789 Datasheet: Section 7.4 (Parallel Interface Timing)