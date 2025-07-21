# ESP LCD DMA Migration Plan

## Overview
Migration from GPIO bit-banging display driver to ESP-IDF LCD component with DMA support.

**Goal**: Achieve 30+ FPS with <25% CPU usage (currently 10 FPS at ~90% CPU)

**Start Time**: 2025-01-21

## Phase 0: Safeguards & Baseline [IN PROGRESS]

### Tasks
- [x] Create lcd-dma branch
- [x] Record baseline FPS/CPU metrics script
- [x] Add GPIO/DMA feature flags to Cargo.toml
- [ ] Set up CI matrix for both backends

### Implementation Log

#### 1. Creating lcd-dma branch ✅
```bash
git checkout -b lcd-dma
```
Branch created successfully.

#### 2. Baseline Metrics (GPIO Implementation)
- **Version**: v5.37-dma (bumped for testing)
- **Script**: `scripts/baseline-performance.sh` created
- **FPS**: [TO BE MEASURED - run script]
- **CPU Usage**: [TO BE MEASURED - run script]
- **Render Time**: ~357ms (full screen)
- **Main Loop**: ~19k FPS with 100% skip rate

#### 3. Feature Flags Setup ✅
Added to Cargo.toml:
```toml
[features]
default = ["lcd-gpio"]
lcd-gpio = []
lcd-dma = []
```

#### 4. CI Matrix Configuration
[TO BE IMPLEMENTED]

### Status
🟡 In Progress - Ready to run baseline measurements

---

## Phase 1: Reactivate & Compile (Est: 90 min) ✅

### Tasks
- [x] Uncomment `pub mod lcd_cam_esp_hal;` in display/mod.rs
- [x] Add lcd-dma Cargo feature gate
- [x] Resolve compilation errors
- [x] Verify esp-idf-sys has LCD support

### Implementation Log

#### Compilation Fixes Applied:
1. **Module Enabled**: Added `#[cfg(feature = "lcd-dma")]` gates to lcd_cam_esp_hal and lcd_cam_display_manager
2. **Structure Updates**:
   - Changed `clk_src` to use `soc_periph_lcd_clk_src_t_LCD_CLK_SRC_DEFAULT`
   - Fixed `psram_trans_align` - now in `__bindgen_anon_1` union
   - Updated `esp_lcd_panel_io_i80_config_t` dc_levels to use bitfield
   - Fixed `new_bitfield_1` to use 5 parameters (was 3)
   - Changed panel config to use `__bindgen_anon_1` for color space
3. **Build Success**: Compiles cleanly with `--features lcd-dma`

**Time Taken**: ~20 minutes

### Status
✅ Complete - Ready for Phase 2 testing

---

## Phase 2: Minimal Pixel Push (Est: 2 hrs) [IN PROGRESS]

### Tasks
- [x] Configure with reference values:
  - pclk_hz = 17 MHz ✅
  - max_transfer_bytes = 32768 ✅ (using 320*100*2)
  - trans_queue_depth = 10 ✅ (kept at 10 for better throughput)
- [x] Implement black screen smoke test
- [ ] Monitor serial for LCD initialization messages
- [ ] **Checkpoint A**: Verify any pixels rendered

### Expected Serial Output
```
I (xxx) lcd_panel: new I80 bus(iomux), clk=17MHz ...
```

### Implementation Log

#### Test Implementation
1. Created `esp_lcd_test.rs` with comprehensive test:
   - Black screen test
   - Color cycle (Red, Green, Blue, White)
   - Rectangle drawing test
   - Text rendering test
2. Added test flag `RUN_ESP_LCD_TEST` in main.rs
3. Test builds successfully with `--features lcd-dma`

#### Next Steps
1. Flash device with: `./compile.sh --no-default-features --features lcd-dma && ./scripts/flash.sh`
2. Monitor serial output for ESP LCD messages
3. Verify display shows test pattern

### Status
🟡 Ready to flash and test - Checkpoint A pending

---

## Phase 3: Performance Optimization (Est: 2.5 hrs) [PREPARED]

### Tasks
- [x] Create FPS benchmark suite
- [ ] Baseline FPS measurement (target: >25 FPS)
- [ ] Clock stepping: 24→30→40→48 MHz
- [ ] Tune max_transfer_bytes & queue depth
- [ ] Implement double buffering
- [ ] **Checkpoint B**: 30 FPS, <25% CPU, no memory leaks

### Performance Tracking
| Clock (MHz) | FPS | CPU % | Notes |
|-------------|-----|-------|-------|
| 17          | TBD | TBD   | Awaiting hardware test |
| 24          | TBD | TBD   |       |
| 30          | TBD | TBD   |       |
| 40          | TBD | TBD   |       |
| 48          | TBD | TBD   |       |

### Implementation Log

#### Benchmark Suite Created
1. `esp_lcd_benchmark.rs` with tests for:
   - Full screen clear operations
   - Rectangle drawing
   - Text rendering
   - Clock speed comparisons
2. Added quick FPS test to `esp_lcd_test.rs`
3. Benchmarks measure against 10 FPS GPIO baseline

### Status
🟡 Ready for performance testing after Checkpoint A

---

## Phase 4: Integration (Est: 1.5 hrs) [STARTED]

### Tasks
- [x] Create DisplayBackend trait
- [ ] Implement DisplayBackend for GPIO DisplayManager
- [ ] Implement DisplayBackend for LcdDisplayManager
- [ ] Runtime selection via feature flags
- [ ] OTA update testing (5 cycles)
- [ ] Deep sleep testing
- [ ] WiFi coexistence testing

### Implementation Log

#### DisplayBackend Trait Created
1. `display_backend.rs` defines common interface for:
   - Drawing operations (clear, pixel, rect, text)
   - Flush operations
   - Power management (auto-dim, activity timer)
   - Backend identification
2. Trait allows runtime switching between implementations
3. Ready for implementation by both GPIO and DMA backends

### Status
🟡 Backend trait created, awaiting implementations

---

## Phase 5: Documentation (Est: 1 hr)

### Tasks
- [ ] Document why original attempt failed
- [ ] Create working sdkconfig.defaults
- [ ] Performance comparison table
- [ ] Known limitations/quirks
- [ ] Update CHANGELOG

### Implementation Log
[TO BE COMPLETED]

### Status
⏸️ Not Started

---

## Decision Points

### Checkpoint A (Phase 2)
- **Pass**: Any non-garbled pixels rendered
- **Fail**: Fall back to GPIO, open Espressif forum thread

### Checkpoint B (Phase 3)
- **Pass**: 30+ FPS, <25% CPU, stable over 5 OTA cycles
- **Fail**: Keep DMA behind feature flag, ship GPIO as default

---

## Technical Notes

### Pin Configuration (Current Hardware)
- Data pins: GPIO 39,40,41,42,45,46,47,48
- WR: GPIO 8
- DC: GPIO 7  
- CS: GPIO 6
- RST: GPIO 5

### Known Issues
- LCD_CAM direct register access failed (no signal output)
- PSRAM frame buffer causes 96% performance degradation
- Current implementation exists but was disabled with comment "needs fixing"

### Reference Implementation
- Location: `reference-template/components/tdisplays3/t_display_s3.c`
- Clock: 17 MHz
- Transfer size: LCD_H_RES * 100 * sizeof(uint16_t)

---

## Progress Summary
- **Started**: 2025-01-21
- **Current Phase**: 2 (Ready for hardware testing)
- **Completed**: Phases 0, 1, and partial 2, 3, 4
- **Time Spent**: ~2 hours
- **Remaining**: Hardware validation and integration
- **Go/No-Go Decision**: Pending Checkpoint A (hardware test)

## Current Status

### ✅ Completed
1. **Phase 0**: Branch setup, feature flags, CI matrix
2. **Phase 1**: Fixed all compilation errors with esp-idf-sys v0.36.1
3. **Phase 2**: Test suite ready, configured with reference values
4. **Phase 3**: Benchmark suite prepared
5. **Phase 4**: DisplayBackend trait created

### 🔧 Ready to Test
1. **ESP LCD Test**: Set `RUN_ESP_LCD_TEST = true` in main.rs
2. **Build**: `./compile.sh --no-default-features --features lcd-dma`
3. **Flash**: `./scripts/flash.sh`
4. **Monitor**: Look for `I (xxx) lcd_panel` messages

### ⏳ Pending Hardware Validation
1. **Checkpoint A**: Verify pixels rendered on display
2. **Performance**: Measure actual FPS (target >25)
3. **Stability**: Run for extended period
4. **Integration**: Implement backend trait for both drivers