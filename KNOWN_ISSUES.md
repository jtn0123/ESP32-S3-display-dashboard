# Known Issues and Attempted Solutions

This document consolidates all known issues, attempted solutions, and technical challenges encountered in the ESP32-S3 Display Dashboard project.

## 1. LCD_CAM Hardware Acceleration Failure

### Issue Description
The LCD_CAM peripheral appears to function internally but fails to drive GPIO pins, resulting in no display output despite registers indicating successful operation.

### Symptoms
- LCD_CAM registers all read as `0x00000000` after configuration
- START bit sets and clears indicating transfer completion
- No watchdog timeouts or system crashes
- Display remains black (backlight on, no data)
- Benchmark shows 167 FPS but no actual pixel output

### Root Cause
The LCD_CAM peripheral uses shadow registers that require undocumented synchronization sequences to copy configuration values to live registers. Without proper synchronization, the peripheral operates internally but doesn't drive GPIO pins.

### What We Tried
1. **Basic Register Configuration**
   ```rust
   - Clock enable (SYSTEM_PERIP_CLK_EN1_REG)
   - Reset clear (SYSTEM_PERIP_RST_EN1_REG)
   - Output enables (LCD_DOUT_EN in CTRL1)
   - GPIO Matrix routing (signals 132-154)
   - FIFO configuration
   - Data output mode (0xFF for all pins)
   ```

2. **Shadow Register Fix Attempts**
   ```rust
   - LCD_UPDATE bit in USER register
   - Proper sequencing with delays
   - Memory barriers around register access
   - Clock disable/enable sequences
   ```

3. **Advanced Configuration**
   ```rust
   - LCD_MISC register bits (LCD_CD_CMD_SET, LCD_CD_DATA_SET, etc.)
   - AFIFO_ADDR_BRIDGE_EN for FIFO bridge
   - Multiple clock divider settings
   - Pin drive strength maximization
   ```

4. **Community Solutions**
   - ESP-HAL I8080 implementation (exists but has similar issues)
   - ESP-IDF LCD driver wrapper (too complex, binding mismatches)
   - Various timing and sequencing attempts

### Why It Failed
- Missing undocumented output driver enable sequence
- ESP-IDF uses complex `lcd_ll_enable_output_always_on()` function
- Possible silicon-specific workarounds required
- Shadow register synchronization sequence not fully documented

### Current Status
**Not Working** - Abandoned in favor of GPIO bit-banging at 10 FPS

### References
- [LCD_CAM_DETAILED_ANALYSIS.md](LCD_CAM_DETAILED_ANALYSIS.md)
- [LCD_CAM_FINAL_REPORT.md](LCD_CAM_FINAL_REPORT.md)
- [LCD_CAM_PERFORMANCE_REPORT.md](LCD_CAM_PERFORMANCE_REPORT.md)

---

## 2. ESP-IDF Bootloader Flash Size Issue

### Issue Description
The bootloader reports 4MB flash size during boot despite the ESP32-S3 T-Display having 16MB flash.

### Symptoms
```
I (36) boot: SPI Flash Size : 4MB  ← Incorrect, should be 16MB
```

### Root Cause
esp-idf-sys v0.36.1 caches a pre-built bootloader from ESP-IDF v5.1-beta1 which defaults to 4MB flash size.

### Workaround
Always specify flash size explicitly when flashing:
```bash
espflash flash --flash-size 16mb --port /dev/cu.usbmodem101 [binary]
```

Or use esptool.py directly:
```bash
esptool.py --chip esp32s3 --flash_size 16MB write_flash 0x10000 [binary]
```

### Impact
- Bootloader reports incorrect size but app has full 16MB access
- OTA partitions work correctly with proper partition table
- No functional impact, just incorrect boot message

### Current Status
**Working with Workaround** - Document in README for users

---

## 3. Display Performance Limitations

### Issue Description
GPIO bit-banging achieves only 10 FPS due to blocking write operations.

### Current Implementation
- Direct GPIO pin manipulation for 8-bit parallel interface
- Synchronous blocking writes
- No DMA support for GPIO mode
- CPU-intensive busy-wait loops

### Performance Metrics
- Full screen clear: ~100ms
- Typical frame render: ~80-100ms
- Maximum theoretical: ~12 FPS
- Practical sustained: 10 FPS

### Optimization Opportunities
1. **Dirty Rectangle Tracking** (partially implemented)
2. **Partial Screen Updates** (not implemented)
3. **Double Buffering** (frame buffer allocated but unused)
4. **DMA for GPIO** (not available on ESP32-S3)

### Current Status
**Acceptable** - 10 FPS sufficient for dashboard application

---

## 4. Watchdog Timer Sensitivity

### Issue Description
System watchdog triggers during display initialization if not reset frequently.

### Solution Implemented
- Watchdog timeout increased to 5 seconds
- Regular resets during long operations
- Strategic placement in render loops

### Current Status
**Resolved** - Proper watchdog management implemented

---

## 5. Display Boundary Offsets

### Issue Description
ST7789 controller memory doesn't align with physical display area.

### Solution
```rust
const DISPLAY_X_START: u16 = 10;   // Left boundary offset
const DISPLAY_Y_START: u16 = 36;   // Top boundary offset
const DISPLAY_WIDTH: u16 = 300;    // Actual visible width
const DISPLAY_HEIGHT: u16 = 168;   // Actual visible height
```

### Current Status
**Resolved** - Proper offsets discovered and implemented

---

## 6. Power-on Display Initialization

### Issue Description
Display requires specific power sequencing and timing for reliable initialization.

### Solution
1. LCD power pin (GPIO 15) must be set high and kept alive
2. 500ms delay after power-on before initialization
3. Multiple software resets with delays
4. Comprehensive memory initialization to avoid artifacts

### Current Status
**Resolved** - Reliable initialization sequence implemented

---

## 7. ESP-IDF Version Compatibility

### Issue Description
Project requires ESP-IDF v5.3.x for stability but some features designed for v5.1.

### Impact
- Some ESP-IDF bindings may have changed
- LCD driver structs don't match between versions
- Potential for future breaking changes

### Current Status
**Managed** - Using v5.3.3 LTS successfully

---

## Summary

### Working Features
- GPIO-based display driver (10 FPS)
- Full dashboard UI rendering
- Touch buttons (GPIO 0 and 14)
- Battery monitoring
- WiFi connectivity
- OTA updates
- Dual-core processing
- PSRAM support

### Not Working
- LCD_CAM hardware acceleration
- High-performance display updates (>10 FPS)

### Abandoned Attempts
- ESP-HAL I8080 implementation
- ESP-IDF LCD driver wrapper
- Direct LCD_CAM register manipulation
- DMA-based display updates

The project successfully achieves its goal of a functional IoT dashboard despite the LCD_CAM limitations. The 10 FPS performance is adequate for dashboard applications where smooth animations are not required.