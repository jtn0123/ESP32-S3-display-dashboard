# ESP32-S3 LCD_CAM Peripheral Research

## Overview
The ESP32-S3 LCD_CAM peripheral is a dedicated hardware module designed for camera and LCD interfacing. For our display driver, we'll use the LCD mode with DMA for high-performance parallel data output.

## Key Features for Our Use Case
- **8/16-bit parallel output** (we need 8-bit)
- **DMA support** with linked descriptors
- **Up to 40MHz pixel clock** (vs ~2MHz with GPIO bit-banging)
- **Hardware-controlled timing**
- **Double buffering capability**
- **Automatic data formatting**

## LCD_CAM Configuration for ST7789

### Pin Mapping
```rust
// Our current pin configuration
const LCD_DATA_PINS: [u8; 8] = [39, 40, 41, 42, 45, 46, 47, 48]; // D0-D7
const LCD_WR: u8 = 8;   // Write clock
const LCD_DC: u8 = 7;   // Data/Command
const LCD_CS: u8 = 6;   // Chip Select
const LCD_RST: u8 = 5;  // Reset
```

### LCD_CAM Mode Selection
For ST7789 8-bit parallel interface:
- **Mode**: LCD 8-bit parallel
- **Clock Source**: PLL_F160M (160MHz base)
- **Clock Divider**: 8-16 (10-20MHz output)
- **Byte Order**: Need to verify BGR vs RGB

### DMA Configuration
```rust
// DMA descriptor structure
#[repr(C, align(4))]
struct DmaLcdDescriptor {
    // Descriptor configuration
    // [31:24] Reserved
    // [23:16] Buffer length[15:8]
    // [15:12] Buffer length[19:16]
    // [11]    Reserved
    // [10]    sosf - start of sub-frame
    // [9]     Reserved  
    // [8]     owner - 1: DMA, 0: CPU
    // [7:0]   Buffer length[7:0]
    config: u32,
    
    // Buffer pointer (must be word-aligned)
    buffer: *const u8,
    
    // Next descriptor pointer (NULL to end)
    next: *mut DmaLcdDescriptor,
}

// For 320x170 display:
// - Each line: 320 pixels * 2 bytes = 640 bytes
// - Total frame: 640 * 170 = 108,800 bytes
// - Max DMA descriptor: 4095 bytes
// - Need: 27 descriptors per frame
```

## Implementation Strategy

### Phase 1: Basic LCD_CAM Setup
```rust
use esp_idf_sys::*;

pub struct LcdCam {
    framebuffer: &'static mut [u16; 320 * 170],
    descriptors: &'static mut [DmaLcdDescriptor; 32],
}

impl LcdCam {
    pub unsafe fn init() -> Result<Self, esp_idf_sys::EspError> {
        // Enable peripheral clock
        (*PCR::ptr()).lcd_cam_conf.modify(|_, w| {
            w.lcd_cam_clk_en().set_bit()
             .lcd_cam_rst_en().clear_bit()
        });
        
        // Configure LCD mode
        (*LCD_CAM::ptr()).lcd_user.modify(|_, w| {
            w.lcd_8bits_order().clear_bit()  // Normal byte order
             .lcd_bit_order().clear_bit()    // MSB first
             .lcd_byte_order().clear_bit()   // High byte first
             .lcd_2byte_mode().set_bit()     // 16-bit color mode
        });
        
        // Set up timing
        (*LCD_CAM::ptr()).lcd_clock.write(|w| {
            w.lcd_clk_sel().bits(2)           // PLL_F160M
             .lcd_clkm_div_num().bits(8)     // 160/8 = 20MHz
             .lcd_clkm_div_a().bits(0)
             .lcd_clkm_div_b().bits(0)
             .lcd_ck_out_edge().clear_bit()  // Falling edge
             .lcd_ck_idle_edge().clear_bit()
        });
        
        // Configure pins (using GPIO matrix)
        // ... pin configuration code ...
        
        Ok(LcdCam {
            framebuffer: /* allocate */,
            descriptors: /* allocate */,
        })
    }
}
```

### Phase 2: DMA Descriptor Chain
```rust
impl LcdCam {
    fn setup_descriptors(&mut self) {
        let bytes_per_desc = 4092; // Max size (4095 - 3 byte header)
        let total_bytes = 320 * 170 * 2;
        let num_descriptors = (total_bytes + bytes_per_desc - 1) / bytes_per_desc;
        
        let fb_ptr = self.framebuffer.as_ptr() as *const u8;
        
        for i in 0..num_descriptors {
            let offset = i * bytes_per_desc;
            let remaining = total_bytes.saturating_sub(offset);
            let chunk_size = remaining.min(bytes_per_desc);
            
            self.descriptors[i] = DmaLcdDescriptor {
                config: (chunk_size as u32) | (1 << 8), // Size + owner=DMA
                buffer: unsafe { fb_ptr.add(offset) },
                next: if i < num_descriptors - 1 {
                    &mut self.descriptors[i + 1] as *mut _
                } else {
                    std::ptr::null_mut()
                },
            };
        }
        
        // Mark first descriptor as start-of-frame
        self.descriptors[0].config |= 1 << 10;
    }
}
```

### Phase 3: Display Commands via GPIO
Since LCD_CAM handles parallel data, we still need GPIO for control signals:

```rust
pub struct Display {
    lcd_cam: LcdCam,
    dc_pin: PinDriver<'static, Gpio7, Output>,
    cs_pin: PinDriver<'static, Gpio6, Output>,
    rst_pin: PinDriver<'static, Gpio5, Output>,
}

impl Display {
    fn write_command(&mut self, cmd: u8) {
        self.cs_pin.set_low().unwrap();
        self.dc_pin.set_low().unwrap(); // Command mode
        
        // Write command byte via LCD_CAM
        // This needs special handling - single byte transfer
        
        self.cs_pin.set_high().unwrap();
    }
    
    fn write_data(&mut self, data: &[u8]) {
        self.cs_pin.set_low().unwrap();
        self.dc_pin.set_high().unwrap(); // Data mode
        
        // DMA transfer via LCD_CAM
        unsafe {
            (*LCD_CAM::ptr()).lcd_dma_int_clr.write(|w| w.bits(0xFFFFFFFF));
            (*LCD_CAM::ptr()).lcd_misc.modify(|_, w| w.lcd_afifo_reset().set_bit());
            (*LCD_CAM::ptr()).lcd_user.modify(|_, w| w.lcd_dout().set_bit());
            (*LCD_CAM::ptr()).lcd_dma_req.write(|w| w.bits(1));
        }
        
        // Wait for completion
        while unsafe { (*LCD_CAM::ptr()).lcd_dma_int_raw.read().lcd_trans_done_int_raw().bit_is_clear() } {}
        
        self.cs_pin.set_high().unwrap();
    }
}
```

## Performance Calculations

### Current GPIO Method
- 8 GPIO writes per pixel
- ~125ns per GPIO write
- Total: ~1µs per pixel
- Full screen: 54.4ms (18 FPS max)

### LCD_CAM with DMA
- 20MHz pixel clock
- 50ns per pixel
- Full screen: 2.72ms (367 FPS theoretical)
- **20x performance improvement!**

## Critical Implementation Details

### 1. Memory Alignment
- DMA buffers must be 4-byte aligned
- Use `#[repr(C, align(4))]` for structures
- Consider PSRAM for larger buffers

### 2. Cache Coherency
```rust
// Before DMA write
esp_idf_sys::esp_cache_msync(
    buffer.as_ptr() as *const c_void,
    buffer.len(),
    ESP_CACHE_MSYNC_FLAG_DIR_C2M
);
```

### 3. Interrupt Handling
```rust
// Set up LCD_CAM interrupt
unsafe {
    esp_idf_sys::esp_intr_alloc(
        ETS_LCD_CAM_INTR_SOURCE,
        0,
        Some(lcd_cam_isr),
        std::ptr::null_mut(),
        &mut handle
    );
}
```

### 4. Double Buffering
```rust
pub struct DoubleBuffer {
    front: &'static mut [u16; 320 * 170],
    back: &'static mut [u16; 320 * 170],
    current: AtomicBool,
}

impl DoubleBuffer {
    pub fn swap(&self) {
        self.current.store(!self.current.load(Ordering::Acquire), Ordering::Release);
        // Update DMA descriptors to point to new buffer
    }
}
```

## Testing Plan

### Step 1: Basic LCD_CAM Validation
1. Configure LCD_CAM without display
2. Use logic analyzer on data pins
3. Verify 8-bit parallel output
4. Check timing (20MHz clock)

### Step 2: Simple Patterns
1. Fill framebuffer with solid color
2. Verify DMA transfer completes
3. Check display shows color

### Step 3: Performance Testing
1. Measure frame rate
2. Test tearing effects
3. Optimize descriptor chain

### Step 4: Integration
1. Port ST7789 initialization
2. Implement all drawing primitives
3. Full dashboard UI

## Potential Issues & Solutions

### Issue 1: Byte Order
**Problem**: LCD_CAM might swap bytes
**Solution**: Configure byte_order bits or swap in software

### Issue 2: Command/Data Timing
**Problem**: DC pin must be stable before WR
**Solution**: Use LCD_CAM delays or manual synchronization

### Issue 3: DMA Descriptor Limits
**Problem**: 4095 byte limit per descriptor
**Solution**: Chain multiple descriptors (already planned)

### Issue 4: Power Consumption
**Problem**: DMA might increase power usage
**Solution**: Use LCD_CAM power-down between frames

## References
1. ESP32-S3 Technical Reference Manual - Chapter 35 (LCD_CAM)
2. ESP-IDF LCD examples (adapt from SPI to parallel)
3. ST7789 Datasheet - 8-bit parallel interface timing

## Next Steps
1. [ ] Set up minimal LCD_CAM test
2. [ ] Verify parallel output with scope
3. [ ] Implement basic framebuffer DMA
4. [ ] Port ST7789 initialization
5. [ ] Benchmark vs current implementation