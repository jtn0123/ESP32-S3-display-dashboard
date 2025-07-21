# ESP LCD DMA Test Instructions

## Quick Test (5 minutes)

1. **Enable Test Mode**
   ```bash
   # Already enabled in main.rs:
   # const RUN_ESP_LCD_TEST: bool = true;
   ```

2. **Build & Flash**
   ```bash
   ./compile.sh --no-default-features --features lcd-dma && ./scripts/flash.sh
   ```

3. **Monitor Serial**
   ```bash
   espflash monitor
   ```

## Expected Results

### Serial Output
```
[ESP_LCD_TEST] Starting black screen test...
I (xxx) lcd_panel: new I80 bus(iomux), clk=17MHz, D0~D7 with 8-bit width
[ESP_LCD_TEST] Display initialized successfully!
[ESP_LCD_TEST] Test 1: Filling screen black...
[ESP_LCD_TEST] Test 2: Color cycle test...
[ESP_LCD_TEST] - Red
[ESP_LCD_TEST] - Green
[ESP_LCD_TEST] - Blue
[ESP_LCD_TEST] - White
[ESP_LCD_TEST] Test 3: Drawing rectangles...
[ESP_LCD_TEST] Test 4: Text rendering...
[ESP_LCD_TEST] Quick benchmark: XX.X FPS (target: >25 FPS)
```

### Display Output
1. Black screen (1 second)
2. Color cycle: Red → Green → Blue → White (500ms each)
3. Four colored rectangles
4. Text showing:
   - "ESP LCD Working!"
   - "DMA Enabled"
   - "v5.37-dma"

## Success Criteria

✅ **Checkpoint A PASS** if:
- Serial shows "I (xxx) lcd_panel" message
- Display shows any visible output
- No crash/restart

✅ **Performance PASS** if:
- FPS > 25 (2.5x GPIO baseline)
- Display updates smooth
- No glitches/artifacts

## Troubleshooting

### No Serial Output
- Check USB cable
- Verify port: `ls /dev/tty.usb*`
- Try: `espflash monitor --port /dev/tty.usbmodem101`

### No "lcd_panel" Message
- ESP LCD not initializing
- Check pin connections
- Verify power supply

### Display Black
- Check backlight pin (GPIO38)
- Check LCD power pin (GPIO15)
- Try increasing delays in test

### Low FPS
- Expected: 25-45 FPS
- If <25 FPS, try clock optimization in Phase 3

## Next Steps

If tests pass:
1. Record FPS number
2. Set `RUN_ESP_LCD_TEST = false` in main.rs
3. Continue to Phase 3 optimization
4. Implement DisplayBackend trait

If tests fail:
1. Save serial output
2. Check LCD_CAM_FINAL_REPORT.md for similar issues
3. Post to Espressif forum with logs