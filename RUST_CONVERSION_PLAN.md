# ESP32-S3 Dashboard: Rust Native Conversion Plan

## Project Status
- **Start Date**: 2025-01-14
- **Target Completion**: 4 weeks
- **Approach**: Native Rust with DMA (no FFI)
- **Arduino Stable Version**: v1.0-arduino-stable (backed up)

## Backup Strategy
✅ **Completed**
- Git tag: `v1.0-arduino-stable` 
- Stable branch: `arduino-stable`
- All features working in Arduino version

To restore Arduino version:
```bash
git checkout v1.0-arduino-stable  # or
git checkout arduino-stable
```

## Conversion Strategy

### Phase 1: Native Rust Display Driver with DMA (Week 1)
**Goal**: Hardware-accelerated display using ESP32-S3 LCD_CAM peripheral

#### Research Findings ✅
- ESP32-S3 has dedicated LCD_CAM peripheral for parallel displays
- Supports 8/16-bit parallel interface with DMA
- Can achieve 40+ MHz pixel clock (10x faster than GPIO)
- Double buffering possible for tear-free updates

#### Implementation Status
- ✅ LCD_CAM research completed (see `rust-dashboard/docs/LCD_CAM_RESEARCH.md`)
- ✅ Basic project structure created
- ✅ LCD_CAM driver skeleton implemented
- ✅ ST7789 controller skeleton implemented
- 🚧 Need to solve command/data byte sending issue

#### Current Challenge: Command/Data Interface
**Problem**: LCD_CAM is designed for bulk DMA transfers, but ST7789 commands need single-byte transfers with DC pin control.

**Solutions being explored**:
1. **Hybrid approach**: GPIO for commands, LCD_CAM for data
2. **Single-byte DMA**: Configure LCD_CAM for small transfers
3. **Command buffer**: Batch commands with special markers

#### Implementation Plan
1. **LCD_CAM Configuration**
   ```rust
   // Target configuration
   - Data width: 8-bit
   - Clock: 10-20 MHz
   - DMA channels: 2 (double buffer)
   - VSYNC/HSYNC: Optional (we don't need)
   ```

2. **Display Driver Architecture**
   ```rust
   pub struct DmaDisplay {
       lcd_cam: LcdCam,
       dma_channels: (DmaChannel0, DmaChannel1),
       framebuffer: [u16; 320 * 170],
       back_buffer: [u16; 320 * 170],
       current_buffer: AtomicU8,
   }
   ```

3. **Key Components**
   - ✅ LCD_CAM peripheral initialization
   - ✅ DMA descriptor chain setup
   - 🚧 ST7789 command interface
   - ✅ Framebuffer management
   - [ ] Hardware-accelerated primitives

#### Technical Details
- **Pinout Mapping**: LCD_DATA0-7 → GPIO39-48
- **Control Pins**: WR, CS, DC, RST
- **Color Format**: RGB565 (BGR byte order)
- **Buffer Size**: 108KB per buffer (320×170×2)

### Phase 2: ESP-IDF OTA Implementation (Week 2)
**Goal**: Native OTA updates matching ArduinoOTA functionality

#### Implementation Plan
1. **Core OTA Module**
   ```rust
   pub struct OtaManager {
       update_partition: Partition,
       http_server: EspHttpServer,
       mdns: EspMdns,
       update_handle: Option<OtaHandle>,
   }
   ```

2. **Features to Implement**
   - [ ] Web upload interface (like current)
   - [ ] mDNS discovery
   - [ ] Progress reporting
   - [ ] Rollback on failure
   - [ ] Firmware validation

3. **API Design**
   ```rust
   impl OtaManager {
       pub async fn start_update(&mut self, size: usize) -> Result<()>
       pub async fn write_chunk(&mut self, data: &[u8]) -> Result<()>
       pub async fn finish_update(&mut self) -> Result<()>
       pub fn get_progress(&self) -> u8
   }
   ```

### Phase 3: UI System Port (Week 3)
**Goal**: Port all screens with improved performance

#### Screens to Port
- [ ] System Info Screen
- [ ] Power Management Screen
- [ ] WiFi Status Screen
- [ ] Hardware Monitor Screen
- [ ] Settings Screen

#### UI Architecture
```rust
trait Screen {
    fn draw(&self, display: &mut DmaDisplay);
    fn handle_input(&mut self, event: ButtonEvent);
    fn update(&mut self, dt: Duration);
}

struct Dashboard {
    screens: Vec<Box<dyn Screen>>,
    current_screen: usize,
}
```

### Phase 4: Testing & Optimization (Week 4)
- [ ] Performance benchmarking
- [ ] Memory usage optimization
- [ ] Binary size reduction
- [ ] Feature parity testing
- [ ] Documentation

## Project Structure

```
esp32-s3-dashboard/
├── Cargo.toml
├── .cargo/
│   └── config.toml
├── src/
│   ├── main.rs
│   ├── display/
│   │   ├── mod.rs
│   │   ├── lcd_cam.rs      # DMA driver
│   │   ├── st7789.rs       # Display commands
│   │   └── graphics.rs     # Drawing primitives
│   ├── ota/
│   │   ├── mod.rs
│   │   ├── manager.rs      # OTA logic
│   │   └── web_server.rs   # HTTP interface
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── screens/        # Each screen module
│   │   └── widgets/        # Reusable UI components
│   └── hardware/
│       ├── battery.rs      # ADC monitoring
│       └── buttons.rs      # Input handling
└── build.rs
```

## Development Environment Setup

```bash
# Install Rust ESP toolchain
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-installer.sh | sh
espup install

# Clone and setup
cd esp32-s3-dashboard
cargo generate esp-rs/esp-idf-template

# Configure for ESP32-S3
# Select: mcu = "esp32s3", std support
```

## Dependencies

```toml
[dependencies]
esp-idf-sys = { version = "0.34", features = ["binstart"] }
esp-idf-hal = { version = "0.43", features = ["critical-section"] }
esp-idf-svc = { version = "0.48", features = ["std", "alloc"] }
embedded-graphics = "0.8"
embedded-graphics-core = "0.4"
heapless = "0.8"
log = "0.4"
anyhow = "1.0"

[profile.release]
opt-level = "z"  # Optimize for size
lto = true
strip = true
codegen-units = 1
```

## Technical Challenges & Solutions

### 1. LCD_CAM Configuration
**Challenge**: Limited Rust examples for LCD_CAM
**Solution**: Study ESP-IDF C examples, create safe Rust wrapper
**Status**: ✅ Basic wrapper created

### 2. DMA Descriptor Management
**Challenge**: Complex linked list of DMA descriptors
**Solution**: Use static allocation with known buffer sizes
**Status**: ✅ Implemented

### 3. Real-time Display Updates
**Challenge**: Maintaining 60 FPS with Rust safety
**Solution**: Double buffering with atomic buffer swaps
**Status**: 🚧 In progress

### 4. Binary Size
**Challenge**: Rust binaries larger than Arduino
**Solution**: Aggressive optimization flags, no_std where possible
**Status**: ⏳ Will measure after first build

### 5. Command/Data Multiplexing
**Challenge**: LCD_CAM designed for bulk transfers, not commands
**Solution**: Investigating hybrid GPIO/DMA approach
**Status**: 🚧 Active problem

## Success Metrics

1. **Display Performance**
   - [ ] 60+ FPS capability
   - [ ] Tear-free updates
   - [ ] < 10ms full screen redraw

2. **OTA Reliability**
   - [ ] 100% success rate
   - [ ] Automatic rollback on failure
   - [ ] Progress indication

3. **Resource Usage**
   - [ ] Binary < 2MB
   - [ ] RAM usage < 100KB
   - [ ] CPU usage < 50% idle

4. **Feature Parity**
   - [ ] All 5 screens working
   - [ ] Button navigation
   - [ ] Battery monitoring
   - [ ] WiFi connectivity
   - [ ] OTA updates

## Daily Progress Log

### Day 1 (2025-01-14)
- ✅ Created backup of Arduino version
- ✅ Researched LCD_CAM peripheral extensively
- ✅ Planned DMA architecture
- ✅ Created project structure
- ✅ Implemented basic LCD_CAM driver
- ✅ Created ST7789 controller skeleton
- 🚧 Identified command/data interface challenge

### Day 2
- [ ] Solve command/data interface issue
- [ ] Implement GPIO fallback for commands
- [ ] Test basic display initialization
- [ ] Verify DMA data transfers

### Day 3
- [ ] Implement drawing primitives
- [ ] Test full-screen updates
- [ ] Benchmark performance vs Arduino
- [ ] Start OTA research

[Progress updates continue here...]

## Immediate Next Steps

1. **Command/Data Interface Solution**
   ```rust
   // Option 1: GPIO bit-banging for commands
   fn write_command_gpio(&mut self, cmd: u8) {
       // Manually toggle 8 data pins
   }
   
   // Option 2: Special DMA mode
   fn write_command_dma(&mut self, cmd: u8) {
       // Configure LCD_CAM for single transfer
   }
   ```

2. **Test Harness**
   - Create simple test that fills screen with colors
   - Measure actual transfer speed
   - Verify pin outputs with logic analyzer

3. **Integration Test**
   - Initialize display
   - Show solid color
   - Draw test pattern
   - Measure FPS

## Resources & References

1. **ESP32-S3 Technical Reference**: LCD_CAM chapter
2. **esp-idf-hal examples**: SPI DMA (adapt for parallel)
3. **ST7789 Datasheet**: Command set and timing
4. **Embassy HAL**: Async patterns for embedded

## Code Snippets & Experiments

### LCD_CAM Initialization (WIP)
```rust
// Research notes on LCD_CAM configuration
use esp_idf_sys::*;

unsafe fn init_lcd_cam() {
    // Enable LCD_CAM peripheral clock
    (*SYSTEM::ptr()).perip_clk_en1.modify(|_, w| w.lcd_cam_clk_en().set_bit());
    
    // Configure for 8-bit parallel mode
    (*LCD_CAM::ptr()).lcd_clock.write(|w| {
        w.lcd_clk_sel().bits(2)  // PLL_F160M
         .lcd_clkm_div_num().bits(8)  // 160MHz / 8 = 20MHz
    });
    
    // More configuration needed...
}
```

### DMA Descriptor Structure
```rust
#[repr(C, align(4))]
struct DmaDescriptor {
    config: u32,
    buffer: *const u8,
    next: *mut DmaDescriptor,
}
```

## Questions to Resolve

1. ~~How to handle PSRAM for larger framebuffers?~~ Using internal DMA memory
2. Best way to implement async display updates?
3. Should we use Embassy or stick with esp-idf?
4. ~~How to minimize binary size further?~~ Optimization flags configured

## Next Steps

1. **Immediate**: Solve command/data interface
2. **Tomorrow**: First LCD_CAM test with real display
3. **This Week**: Display "Hello Rust" via DMA
4. **Next Week**: Full display driver working

---

*This document is actively updated as the conversion progresses.*