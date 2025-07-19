# DMA Display Implementation Plan

## Overview
This document tracks the systematic implementation of DMA-accelerated display rendering for the ESP32-S3 T-Display dashboard. Each phase includes validation steps and performance metrics.

## Current Status
- **Version**: v4.90-rust-direct
- **Baseline Performance**: 35-50 FPS
- **Display Method**: Direct GPIO bit-banging (no frame buffer)
- **PSRAM**: Available (8MB) but unused
- **DMA**: Disabled due to stability issues
- **Frame Buffer**: Disabled due to boot flashing/corruption

## Implementation Phases

### Phase 1: Frame Buffer Integration ❌ **FAILED - REVERTED**
**Goal**: Route all display operations through PSRAM frame buffer
**Expected Improvement**: 2-3x performance (70-100 FPS)

#### Results:
- ✅ Achieved ~60 FPS initially
- ❌ Screen switching became slower
- ❌ Boot screen flashing issues  
- ❌ Display corruption when combined with other features
- ❌ PSRAM latency issues identified

#### Root Causes (from analysis):
1. Frame buffer in PSRAM caused cache misses during DFS/WiFi
2. No CS pulse between chunks causing sync issues
3. Dirty rectangle merging too aggressive
4. Frame buffer dimension mismatches

### Phase 2: Direct GPIO Register Access ❌ **REVERTED**
**Goal**: Replace GPIO API with direct register manipulation
**Expected Improvement**: +30-40% speed

#### Tasks:
- [x] 2.1 Map GPIO registers for data pins (39,40,41,42,45,46,47,48)
- [x] 2.2 Map GPIO register for WR pin (8)
- [x] 2.3 Implement fast write_byte using register access
- [x] 2.4 Update lcd_bus module with optimized methods
- [x] 2.5 Test - **FAILED: Display corruption and timing issues**

#### Issues Found:
1. Display shows corruption/pixelation when fast GPIO is enabled
2. Boot screen flashing between black and white
3. Timing issues with ST7789 controller
4. **Decision: Reverted to stable GPIO HAL implementation**

#### Validation:
- [ ] Compare timing of old vs new write methods
- [ ] Verify signal integrity with oscilloscope (if available)
- [ ] Test all display functions
- [ ] Measure power consumption change

### Phase 3: Hardware DMA Implementation 📅 **PLANNED**
**Goal**: Use LCD_CAM peripheral for true DMA transfers
**Expected Improvement**: 10-50x speed (200-500+ FPS)

#### Tasks:
- [ ] 3.1 Study ESP32-S3 LCD_CAM peripheral documentation
- [ ] 3.2 Configure LCD_CAM for 8-bit parallel mode
- [ ] 3.3 Set up DMA descriptors and channels
- [ ] 3.4 Implement interrupt handlers for transfer completion
- [ ] 3.5 Add double buffering support
- [ ] 3.6 Optimize descriptor chaining for large transfers

#### Validation:
- [ ] Verify DMA transfers complete without errors
- [ ] Check for tearing or artifacts
- [ ] Measure CPU usage during transfers
- [ ] Stress test with rapid updates

### Phase 4: Advanced Optimizations 📅 **FUTURE**
**Goal**: Squeeze maximum performance
**Expected Improvement**: Additional 20-30%

#### Tasks:
- [ ] 4.1 Implement partial update regions
- [ ] 4.2 Add sprite/bitmap blitting
- [ ] 4.3 Optimize color format conversions
- [ ] 4.4 Add hardware scrolling support
- [ ] 4.5 Implement tear-free vsync

## Performance Tracking

| Version | Phase | FPS | CPU Usage | Method | Notes |
|---------|-------|-----|-----------|--------|-------|
| v4.79 | Baseline | 35-50 | High | GPIO bit-bang | Initial DMA framework |
| v4.80 | Phase 1 | Failed | Testing | Frame buffer | Corruption - dimension mismatch |
| v4.82 | Phase 1 Fix | ~60 | Medium | Frame buffer | Working but slow switching |
| v4.83 | Phase 1 Opt | ~60 | Medium | Frame buffer | Optimized flush method |
| v4.85 | Phase 1 Fix2 | ~60 | Medium | Frame buffer | Fixed boot screen flashing |
| v4.86 | Phase 2 | Failed | Testing | Direct GPIO | Display corruption |
| v4.87 | Phase 2 Fix | Failed | Testing | Direct GPIO | Still has issues |
| v4.88 | Reverted | ~60 | Medium | Frame buffer | Back to stable state |
| **v4.95** | LCD_CAM Test | **10** | High | Raw GPIO baseline | Confirmed baseline performance |
| **v5.00** | LCD_CAM v1 | **7** | High | LCD_CAM byte-by-byte | Inefficient implementation |
| **v5.01** | LCD_CAM v2 | **167** | Medium | LCD_CAM bulk transfer | 16.7x improvement! |
| TBD | Phase 3 | 300+ | Low | True DMA | Target with real DMA |

## Validation Checklist

### After Each Phase:
- [ ] All screens render correctly
- [ ] No visual artifacts or glitches
- [ ] Boot animation runs smoothly
- [ ] OTA updates still work
- [ ] System remains stable under load
- [ ] Power consumption acceptable
- [ ] Temperature within limits

## Known Issues & Risks

1. **LCD_CAM Complexity**: ESP-IDF bindings may be incomplete
2. **Timing Sensitivity**: ST7789 requires specific timing
3. **Memory Bandwidth**: PSRAM is slower than internal RAM
4. **Power Consumption**: Higher performance may increase power draw

## Implementation Log

### 2024-01-19 - Project Started
- Created implementation plan
- Set up tracking structure
- Identified three main phases
- Current version: v4.79-rust-dma (baseline)

### 2024-01-19 - Phase 1 Implementation
- Implemented frame buffer support in DisplayManager
- Modified draw_pixel, fill_rect, and clear to use frame buffer
- Implemented flush() method with dirty rectangle support
- Frame buffer automatically enabled when DMA display is available
- Fixed dimension mismatch (320x170 -> 300x168)
- Fixed boot screen flashing issue
- Phase 1 complete with ~60 FPS performance

### 2024-01-19 - Phase 2 Implementation Attempted
- Created gpio_fast module for direct register access
- Mapped GPIO registers for ESP32-S3 (data pins on bank 1, control on bank 0)
- Implemented fast write methods using volatile pointer writes
- Integrated fast GPIO into lcd_bus module
- **Testing revealed critical issues:**
  - Display corruption and pixelation
  - Boot screen flashing
  - Timing violations with ST7789
- **Reverted to stable GPIO HAL in v4.88**

### NEW: LCD_CAM Hardware Implementation 🚧 **IN PROGRESS**
**Goal**: Use ESP32-S3's LCD_CAM peripheral for hardware-controlled display interface
**Expected Improvement**: 200+ FPS with near-zero CPU usage

#### Why LCD_CAM is the solution:
- Hardware-controlled timing (meets ST7789 40ns requirements)
- True DMA with hardware flow control
- Designed specifically for parallel LCD interfaces
- No software timing violations possible

#### Implementation Status:
- [x] Created LCD_CAM_IMPLEMENTATION.md plan
- [x] Implemented lcd_cam_ll.rs - Low-level register access (had crash issues)
- [x] Implemented lcd_cam_dma.rs - DMA descriptor management
- [x] Created debug_tests.rs - Toggle color & performance tests
- [x] Measured baseline performance: **10 FPS** with raw GPIO
- [x] Fixed LCD_CAM register access crash with safer HAL approach
- [x] Successfully initialized LCD_CAM peripheral
- [x] Configured GPIO matrix for LCD_CAM signals
- [x] Sent ST7789 commands through LCD_CAM
- [ ] Implement data transfer (not just commands)
- [ ] Complete DC pin automatic control
- [ ] Integrate DMA with LCD_CAM
- [ ] Performance validation

#### Current Progress (2025-07-19):
- **v4.95**: Baseline GPIO test - 10 FPS confirmed
- **v4.98**: LCD_CAM HAL working - peripheral accessible
- **v4.99**: GPIO matrix configured, commands working
- **v5.00**: LCD_CAM pixel drawing working - 7 FPS (slower due to byte-by-byte transfers)
- **v5.01**: LCD_CAM bulk transfer - **167 FPS achieved!** 🎉
- Successfully drawing colored rectangles through LCD_CAM with optimized bulk transfers
- 16.7x performance improvement over baseline GPIO
- Still using CPU-driven transfers, not true DMA yet

---
*This document will be updated as implementation progresses*