# Hardware Testing Checklist

## Before First Power-On

- [ ] Verify board model is LILYGO T-Display-S3
- [ ] Check USB-C cable supports data transfer
- [ ] Install USB drivers if needed (CP210x/CH34x)
- [ ] Verify no shorts on power pins
- [ ] Battery disconnected for initial testing

## Software Setup

- [ ] Install Rust ESP toolchain:
  ```bash
  cargo install espup
  espup install
  source ~/export-esp.sh
  ```
- [ ] Install flash tool:
  ```bash
  cargo install espflash cargo-espflash
  ```
- [ ] Replace Cargo.toml with Cargo.toml.esp32
- [ ] Verify target in .cargo/config.toml

## Initial Testing Steps

### 1. Display Test
- [ ] Flash basic display test (solid color fill)
- [ ] Verify backlight control (GPIO38)
- [ ] Test all color channels (R, G, B)
- [ ] Check display boundaries (170x320)

### 2. Button Test
- [ ] Test Button 1 (GPIO0) - should not trigger boot mode
- [ ] Test Button 2 (GPIO14)
- [ ] Verify debouncing works
- [ ] Test long press detection

### 3. Incremental Feature Testing

#### Phase 1: Basic Display
```rust
// Test 1: Fill screen with color
display.clear(Color::RED);
display.flush().await;
```

#### Phase 2: Text Rendering
```rust
// Test 2: Draw text
display.draw_text(10, 10, "Hello ESP32", Color::WHITE);
display.flush().await;
```

#### Phase 3: Graphics
```rust
// Test 3: Draw shapes
display.draw_rect(10, 10, 50, 30, Color::BLUE);
display.draw_circle(100, 100, 20, Color::GREEN);
display.flush().await;
```

#### Phase 4: Animation
```rust
// Test 4: Simple animation
let mut x = 0;
loop {
    display.clear(Color::BLACK);
    display.fill_rect(x, 50, 20, 20, Color::WHITE);
    display.flush().await;
    x = (x + 1) % 300;
    Timer::after_millis(16).await;
}
```

#### Phase 5: Sensors
- [ ] Test battery voltage reading (GPIO4)
- [ ] Verify voltage divider calculation
- [ ] Test internal temperature sensor

#### Phase 6: Power Management
- [ ] Test sleep mode entry/exit
- [ ] Verify wake on button press
- [ ] Test brightness control (if PWM available)

## Common Issues and Solutions

### Display Not Working
1. Check GPIO pin assignments in config.toml
2. Verify ST7789 initialization sequence
3. Ensure backlight is enabled (GPIO38 HIGH)
4. Try different SPI/parallel modes

### Buttons Not Responding
1. Verify pull-up resistors enabled
2. Check active-low logic
3. Increase debounce time
4. Test with simple GPIO read

### High Power Consumption
1. Ensure WiFi disabled when not needed
2. Check display refresh rate
3. Verify sleep modes working
4. Monitor with power meter

### Memory Issues
1. Check stack size in sdkconfig
2. Monitor heap usage
3. Reduce framebuffer size if needed
4. Use release build for testing

## Performance Benchmarks

Record these metrics during testing:

- [ ] Boot time: _____ ms
- [ ] Display refresh rate: _____ FPS
- [ ] Button response time: _____ ms
- [ ] Idle power consumption: _____ mA
- [ ] Active power consumption: _____ mA
- [ ] Available heap memory: _____ KB
- [ ] Flash usage: _____ KB

## Final Validation

- [ ] All screens navigate correctly
- [ ] Animations run smoothly (>20 FPS)
- [ ] No memory leaks over 1 hour
- [ ] Power consumption within spec
- [ ] OTA update works (if implemented)
- [ ] Settings persist across reboot

## Sign-off

- Date: _____________
- Tested by: _____________
- Board serial: _____________
- Firmware version: _____________
- Test result: [ ] PASS [ ] FAIL

Notes:
_________________________________
_________________________________
_________________________________