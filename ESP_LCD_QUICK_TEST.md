# ESP LCD Quick Test Guide

## 🚀 One Command Test

Just run:
```bash
./run-esp-lcd-test.sh
```

This will:
1. ✓ Build with ESP LCD DMA
2. ✓ Flash your device
3. ✓ Monitor and analyze output

## 📋 What You Should See

### On Serial Monitor:
```
ESP LCD DMA Hardware Test v5.37-dma
========================================
✓ ESP LCD INITIALIZED: I (1234) lcd_panel: new I80 bus(iomux), clk=17MHz
🎨 [ESP_LCD_TEST] Test 1: Filling screen black...
🎨 [ESP_LCD_TEST] - Red
🎨 [ESP_LCD_TEST] - Green
🎨 [ESP_LCD_TEST] - Blue
🎨 [ESP_LCD_TEST] - White
✓ PERFORMANCE PASS: [ESP_LCD_TEST] Quick benchmark: 28.3 FPS (target: >25 FPS)
```

### On Display:
1. Black screen (1 sec)
2. Red screen (0.5 sec)
3. Green screen (0.5 sec)
4. Blue screen (0.5 sec)
5. White screen (0.5 sec)
6. 4 colored rectangles
7. Text showing:
   - "ESP LCD Working!"
   - "DMA Enabled"
   - "v5.37-dma"

## ✅ Success Indicators

- **Serial**: Shows "ESP LCD INITIALIZED"
- **Display**: Shows colors and text
- **FPS**: Greater than 25
- **No crashes**: Stays in test loop

## ❌ If It Fails

### No LCD initialization message:
```bash
# Add more debug - edit src/display/lcd_cam_esp_hal.rs
# Change line 98 to:
error!("Failed to create I80 bus: error code {} (0x{:x})", ret, ret);
```

### Display stays black:
1. Check power LED on board
2. Try manual test:
```bash
# Just build and flash without scripts
cargo build --release --no-default-features --features lcd-dma
espflash flash target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
espflash monitor
```

### Low FPS:
- Edit `src/display/esp_lcd_config.rs`
- Change default clock from 17 to 10 MHz

## 📊 Results Log

The test creates a timestamped log file:
```
esp_lcd_test_YYYYMMDD_HHMMSS.log
```

## 🔄 Next Steps

If test **PASSES**:
1. Edit `src/main.rs`
2. Set `RUN_ESP_LCD_TEST = false`
3. Test with full UI

If test **FAILS**:
1. Share the log file
2. Note what appeared on display
3. Check ESP_LCD_TROUBLESHOOTING.md