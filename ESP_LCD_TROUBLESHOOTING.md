# ESP LCD Troubleshooting Guide

## Common Issues and Solutions

### 1. Build Fails
```bash
# Clean build
./compile.sh --clean --no-default-features --features lcd-dma

# Check for cached issues
./scripts/fix-build.sh --deep-clean
```

### 2. No "lcd_panel" Message in Serial
**Symptom**: Missing `I (xxx) lcd_panel: new I80 bus` message

**Possible Causes**:
- ESP LCD not initializing
- Wrong GPIO pins
- Power issue

**Solutions**:
1. Add debug logging:
```rust
// In lcd_cam_esp_hal.rs, add after each esp_lcd call:
info!("esp_lcd_new_i80_bus returned: {:?}", ret);
```

2. Check return codes:
```rust
if ret != ESP_OK {
    error!("Failed with code: {}", ret);
}
```

### 3. Display Stays Black
**Symptom**: Serial shows success but display is black

**Check**:
1. Backlight pin (GPIO38) - should be HIGH
2. LCD power pin (GPIO15) - should be HIGH
3. Display connector seated properly

**Try**:
```rust
// In esp_lcd_test.rs, add after display init:
info!("Forcing backlight high...");
display.ensure_display_on()?;
Ets::delay_ms(1000);
```

### 4. Low FPS (<25)
**Try Lower Clock**:
```rust
// In lcd_cam_esp_hal.rs, change:
pclk_hz: 17_000_000,  // to:
pclk_hz: 10_000_000,  // Start slower
```

### 5. Display Corruption/Artifacts
**Possible Causes**:
- Clock too fast for cable length
- Transfer size too large
- Timing issues

**Solutions**:
1. Reduce clock speed
2. Reduce transfer size:
```rust
// In esp_lcd_config.rs
TransferSize::Lines50  // Instead of Lines100
```

### 6. Panic/Crash
**Get Full Backtrace**:
```bash
# Monitor with backtrace decode
espflash monitor --decode

# Or use addr2line manually
xtensa-esp32s3-elf-addr2line -e target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard 0x42001234
```

### 7. Memory Issues
**Check Free Heap**:
```rust
// Add to test:
let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
info!("Free heap: {} bytes", free_heap);
```

## Emergency Recovery

If device becomes unresponsive:

1. **Full Erase**:
```bash
espflash erase-flash
./scripts/flash.sh
```

2. **Revert to GPIO**:
```bash
git checkout main
./compile.sh && ./scripts/flash.sh
```

3. **Boot into safe mode** (hold BOOT button during reset)

## Debug Checkpoints

Add these to narrow down issues:

```rust
info!("CHECK 1: Before bus init");
// bus init code
info!("CHECK 2: After bus init");
// panel init code
info!("CHECK 3: After panel init");
// first draw
info!("CHECK 4: After first draw");
```

## Serial Debug Commands

While monitoring, try:
- `Ctrl+T` then `Ctrl+H` - Show help
- `Ctrl+T` then `Ctrl+R` - Reset device
- `Ctrl+]` - Exit monitor

## If Nothing Works

1. Document exact serial output
2. Check if reference template still works:
```bash
cd reference-template
idf.py build flash monitor
```
3. Compare pin definitions
4. Post to Espressif forum with logs