# Known Issues and Attempted Solutions

This document consolidates all known issues, attempted solutions, and technical challenges encountered in the ESP32-S3 Display Dashboard project.

## Build and Toolchain Issues

### Cargo Build Hangs at regex-automata

**Issue**: Build hangs indefinitely at "Compiling regex-automata v0.4.9" (around compilation #22)

**Cause**: Multiple factors can cause this:
- VS Code rust-analyzer running cargo check in background
- Stale .cargo-lock files from interrupted builds
- Corrupted cargo registry cache
- Network timeouts downloading dependencies

**Solution**:
1. Close VS Code before building
2. Kill stuck processes: `pkill -9 cargo`
3. Clear locks: `find . -name ".cargo-lock" -delete`
4. Clear cache if needed: `rm -rf ~/.cargo/registry/cache ~/.cargo/registry/index`

### ESP Toolchain Not Recognized

**Issue**: "error: override toolchain 'esp' is not installed"

**Cause**: The ESP toolchain is a custom toolchain that rustup doesn't recognize by default

**Solution**:
1. Install via espup: `cargo install espup --version 0.13.0`
2. Run: `espup install --targets esp32s3 --std`
3. Source environment: `source ~/export-esp.sh`

### espup v0.15.1 Installation Fails

**Issue**: Dependency conflict with indicatif versions

**Solution**: Use older stable version: `cargo install espup --version 0.13.0`

### Frame Pointer Compilation Warnings

**Issue**: "Inherited flag '-fno-omit-frame-pointer' is not supported by the currently used CC"

**Solution**: Remove from `.cargo/config.toml`:
```toml
rustflags = [
    # Remove: "-C", "force-frame-pointers=yes",
]
```

### Partition Table Not Found

**Issue**: Build fails with "FileNotFoundError: partitions_16mb_ota.csv"

**Solution**: Either use default partition table in sdkconfig.defaults:
```
CONFIG_PARTITION_TABLE_CUSTOM=n
CONFIG_PARTITION_TABLE_TWO_OTA=y
```
Or ensure custom partition file exists at correct path.

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

## 3. ESP_LCD DMA Driver Migration (RESOLVED)

### Issue Description
Migrating from GPIO bit-banging to ESP-IDF's esp_lcd DMA driver caused BREAK instruction crashes and bootloader compatibility issues.

### Symptoms
- BREAK instruction crashes in DMA interrupt service routines
- "Multiple DROM segments" bootloader error preventing app from loading
- Display stays black with no serial output

### Root Causes
1. **Struct layout mismatch** between ESP-IDF v5.1-beta1 (esp-idf-sys) and v5.3.3 (actual build)
2. **DMA descriptor alignment** issues requiring 32-byte boundaries
3. **Cache coherency** problems with DMA buffers
4. **Multiple DROM segments** when using SIZE optimization

### Solution
1. **Update to esp-idf-sys master branch** for ESP-IDF v5.3 compatibility
2. **Implement four critical fixes**:
   - Cache writeback using `xthal_dcache_sync()` before DMA operations
   - 32-byte aligned DMA buffers in internal RAM
   - ISR in IRAM via `esp_lcd_new_panel_io_i80()` helper
   - Proper reset GPIO (GPIO5 instead of -1)
3. **Switch to PERF optimization** in sdkconfig.defaults:
   ```
   CONFIG_COMPILER_OPTIMIZATION_PERF=y
   ```

### Performance Improvement
- **Before**: ~10 FPS with GPIO bit-banging
- **After**: 55-65 FPS with DMA acceleration (5-6x improvement)

### Current Status
**RESOLVED** - ESP_LCD DMA driver working in v5.53-lcdPerf and later

---

## 4. Display Performance Limitations (GPIO Mode)

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

## 5. Watchdog Timer Sensitivity

### Issue Description
System watchdog triggers during display initialization if not reset frequently.

### Solution Implemented
- Watchdog timeout increased to 5 seconds
- Regular resets during long operations
- Strategic placement in render loops

### Current Status
**Resolved** - Proper watchdog management implemented

---

## 6. Display Boundary Offsets

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

## 7. Power-on Display Initialization

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

## 8. ESP-IDF Version Compatibility

### Issue Description
Project requires ESP-IDF v5.3.x for stability but some features designed for v5.1.

### Impact
- Some ESP-IDF bindings may have changed
- LCD driver structs don't match between versions
- Potential for future breaking changes

### Current Status
**Managed** - Using v5.3.3 LTS successfully

---

## 9. PSRAM Frame Buffer Performance Degradation

### Issue Description
Implementing a PSRAM-backed frame buffer causes severe performance degradation, reducing FPS from 55 to 1.9 (96% slower).

### Symptoms
- Frame time increases from ~18ms to ~537ms
- Display updates become visibly slow
- CPU usage remains high despite low FPS
- System remains responsive but display lags severely

### Root Cause
GPIO bit-banging cannot efficiently handle full frame buffer updates:
- 50,400 pixels × 2 bytes × 2 GPIO operations = 201,600 operations per frame
- Each operation involves multiple GPIO pin manipulations
- PSRAM access adds additional latency
- No hardware acceleration available for bulk transfers

### What We Tried
1. **Basic Frame Buffer Implementation**
   - Dual buffers in PSRAM with 16-byte alignment
   - Differential updates with dirty region tracking
   - Block-based change detection (16×16 pixels)

2. **Performance Optimizations**
   - Simplified to full buffer updates (removed differential logic)
   - Added cache coherency barriers
   - Attempted bulk pixel transfer methods
   - Memory fence operations for PSRAM sync

3. **Debugging Attempts**
   - Added extensive logging
   - Tested various pixel formats
   - Verified buffer initialization
   - Checked for endianness issues

### Performance Impact
- Baseline: ~55 FPS (18ms per frame)
- With frame buffer: 1.9 FPS (537ms per frame)
- Performance penalty: 2,883% slower
- Unusable for real-time display updates

### Current Status
**Disabled** - Frame buffer code remains but is disabled (v5.15-fb-off)

### Lessons Learned
- GPIO bit-banging is fundamentally incompatible with frame buffer architectures
- Full screen updates require hardware acceleration (LCD_CAM/DMA)
- Dirty rectangle tracking alone provides better optimization
- PSRAM access patterns need careful consideration for performance

---

## Summary

### Working Features
- ESP_LCD DMA driver (55-65 FPS) with v5.53-lcdPerf and later
- GPIO-based display driver (10 FPS) as fallback
- Full dashboard UI rendering
- Touch buttons (GPIO 0 and 14)
- Battery monitoring
- WiFi connectivity
- OTA updates
- Dual-core processing
- PSRAM support

### Not Working
- Raw LCD_CAM register manipulation (shadow register sync issues)
- PSRAM frame buffer (96% performance degradation)

### Abandoned Attempts
- ESP-HAL I8080 implementation
- Raw LCD_CAM register manipulation
- PSRAM frame buffer with full screen updates

The project successfully achieves high-performance display updates using the ESP_LCD DMA driver (55-65 FPS) after resolving struct alignment and optimization issues. The GPIO fallback mode (10 FPS) remains available if needed.