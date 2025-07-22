# ESP LCD DMA Integration - Current State Summary

## Repository Status
- **Branch**: `lcd-dma` (just pushed to GitHub)
- **Version**: v5.44-watchdog
- **Build Status**: Compiles successfully
- **Runtime Status**: Crashes during pixel format tests due to DMA buffer overflow

## What's Been Done

### 1. ESP LCD DMA Integration
- Migrated from GPIO bit-banging to ESP-IDF's hardware-accelerated `esp_lcd` component
- Using I80 8-bit parallel bus with DMA transfers
- Feature flag `lcd-dma` is now the default
- Verification logging confirms ESP LCD DMA is active

### 2. Display Issues Addressed
- **6-Block Pattern**: Display shows 6 vertical colored blocks instead of proper content
  - Applied fixes: reduced clock to 5 MHz, aligned transfers to 12-byte boundaries
  - Anti-flicker fix increased clock back to 24 MHz
- **Flickering**: Implemented anti-flicker configuration
  - Clock speed: 24 MHz (was 5 MHz)
  - Synchronous transfers (queue depth = 1)
  - Transfer size: ~6.8KB (20 lines per transfer)
- **Watchdog Timeouts**: Added periodic watchdog resets in tests

### 3. Current Crash Issue
```
assert failed: panel_io_i80_tx_color esp_lcd_panel_io_i80.c:479 
(color_size <= (bus->num_dma_nodes * DMA_DESCRIPTOR_BUFFER_MAX_SIZE))
```
- Occurs in pixel format test when drawing 320x30 pixel buffer (19.2KB)
- DMA buffer configuration limits transfers to ~6.8KB
- Tests can't complete due to this limitation

## Key Files to Review

### Display Implementation
- `src/display/lcd_cam_display_manager.rs` - Main ESP LCD implementation
- `src/display/lcd_cam_esp_hal.rs` - Low-level ESP LCD initialization
- `src/display/esp_lcd_6block_fix.rs` - Attempts to fix 6-block pattern
- `src/display/esp_lcd_flicker_fix.rs` - Anti-flicker configuration
- `src/display/esp_lcd_pixel_test.rs` - Pixel format test (crashes here)

### Configuration
- `Cargo.toml` - Feature flags (lcd-dma is default)
- `src/version.rs` - Current version tracking
- `src/main.rs` - Main loop and initialization

## What Works
1. ESP-IDF I80 bus initializes successfully
2. Small buffer transfers work correctly
3. Hardware acceleration is functional
4. Display shows colored blocks (wrong pattern but proves communication)
5. Watchdog resets prevent test timeouts

## What Doesn't Work
1. Display content is scrambled (6-block pattern)
2. Large buffer transfers cause DMA overflow
3. Pixel format tests can't complete
4. Actual UI content doesn't render correctly

## Hypotheses for 6-Block Issue
1. **DMA Alignment**: 6-pixel pattern suggests alignment issues (6 pixels = 12 bytes in RGB565)
2. **Byte Ordering**: Possible endianness mismatch between CPU and DMA
3. **Transfer Size**: DMA descriptors might be splitting data incorrectly
4. **Timing**: Clock speed or data setup/hold times might be marginal

## Next Steps
1. **Immediate**: Reduce test buffer sizes to fit within DMA limits
2. **Debug**: Add logging to trace exact byte patterns being sent vs received
3. **Experiment**: Try different DMA configurations and alignment settings
4. **Alternative**: Consider using smaller transfer chunks with proper reassembly

## Build and Test Commands
```bash
# Build
./compile.sh

# Flash
./scripts/flash.sh

# Monitor
./scripts/monitor.sh

# Clean build
./compile.sh --clean
```

## Environment
- ESP32-S3 with 8MB PSRAM
- LilyGo T-Display-S3 (170x320 ST7789 display)
- ESP-IDF v5.3.3 LTS
- Rust with esp toolchain