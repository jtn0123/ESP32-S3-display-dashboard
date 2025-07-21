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

## Phase 2: Minimal Pixel Push (Est: 2 hrs)

### Tasks
- [ ] Configure with reference values:
  - pclk_hz = 17 MHz
  - max_transfer_bytes = 32768
  - trans_queue_depth = 5
- [ ] Implement black screen smoke test
- [ ] Monitor serial for LCD initialization messages
- [ ] **Checkpoint A**: Verify any pixels rendered

### Expected Serial Output
```
I (xxx) lcd_panel: new I80 bus(iomux), clk=17MHz ...
```

### Implementation Log
[TO BE COMPLETED]

### Status
⏸️ Not Started

---

## Phase 3: Performance Optimization (Est: 2.5 hrs)

### Tasks
- [ ] Baseline FPS measurement (target: >25 FPS)
- [ ] Clock stepping: 24→30→40→48 MHz
- [ ] Tune max_transfer_bytes & queue depth
- [ ] Implement double buffering
- [ ] **Checkpoint B**: 30 FPS, <25% CPU, no memory leaks

### Performance Tracking
| Clock (MHz) | FPS | CPU % | Notes |
|-------------|-----|-------|-------|
| 17          | TBD | TBD   |       |
| 24          | TBD | TBD   |       |
| 30          | TBD | TBD   |       |
| 40          | TBD | TBD   |       |
| 48          | TBD | TBD   |       |

### Implementation Log
[TO BE COMPLETED]

### Status
⏸️ Not Started

---

## Phase 4: Integration (Est: 1.5 hrs)

### Tasks
- [ ] Implement DisplayBackend trait for DMA
- [ ] Runtime selection via feature flags
- [ ] OTA update testing (5 cycles)
- [ ] Deep sleep testing
- [ ] WiFi coexistence testing

### Implementation Log
[TO BE COMPLETED]

### Status
⏸️ Not Started

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
- **Current Phase**: 0 (Safeguards & Baseline)
- **Estimated Completion**: 9 hours total
- **Go/No-Go Decision**: Pending Checkpoint A