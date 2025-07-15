# ESP32-S3 Dashboard Development Roadmap

## Future Improvements

### Performance Optimizations

#### Low Hanging Fruit (Easy)
- [ ] **IRAM Function Placement**
  - Add `#[ram]` attribute to critical interrupt handlers
  - Expected gain: +2 FPS on heavy redraws
  - Implementation: Find ISR functions and add attribute

- [ ] **Configure PSRAM for Framebuffer**
  - Move 108KB framebuffer to external PSRAM
  - Expected gain: Frees 108KB internal RAM
  - Implementation: Update sdkconfig, modify memory allocation

#### Medium Complexity
- [ ] **Interrupt-Driven Sensor Reading**
  - Replace 5-second polling with hardware timer interrupts
  - Use FreeRTOS queues for thread-safe data passing
  - Expected gain: -5ms UI jitter, smoother updates

- [ ] **Full PWM Backlight Control**
  - Implement proper LEDC PWM driver initialization
  - Smooth fading between brightness levels
  - Expected gain: Better power savings, smoother dimming

- [ ] **Compressed Asset Storage**
  - Pre-convert logos/images to RGB565 at build time
  - Store as binary blobs with include_bytes!
  - Expected gain: -200ms first frame decode time

### Feature Enhancements

#### Display Features
- [ ] **Touch Input Support**
  - Integrate touch controller driver
  - Add gesture recognition (swipe, tap, long press)
  - Enable direct screen navigation

- [ ] **Graph Widgets**
  - Real-time data visualization
  - Configurable time windows
  - Multiple data series support

- [ ] **Themes System**
  - User-selectable color themes
  - Dark/light/high-contrast modes
  - Custom theme creation

#### Connectivity
- [ ] **Bluetooth Support**
  - BLE for configuration
  - Classic Bluetooth for audio/data
  - Phone app integration

- [ ] **MQTT Integration**
  - Publish sensor data
  - Subscribe to control commands
  - Home Assistant integration

#### System Features
- [ ] **Deep Sleep Mode**
  - Wake on button press
  - Periodic wake for updates
  - Ultra-low power consumption

- [ ] **SD Card Logging**
  - Sensor data history
  - Error logs
  - Configuration backup

### Code Quality Improvements

- [ ] **Unit Test Coverage**
  - Add tests for display driver
  - Mock sensor testing
  - UI state machine tests

- [ ] **Documentation**
  - API documentation
  - Hardware setup guide
  - Troubleshooting guide

- [ ] **CI/CD Pipeline**
  - Automated builds
  - Size regression tracking
  - Performance benchmarks

### Hardware Support

- [ ] **Multiple Display Sizes**
  - 240x240 square displays
  - 480x320 larger displays
  - E-ink support

- [ ] **External Sensors**
  - BME280 temperature/humidity/pressure
  - Light sensors
  - Motion detection

## Priority Order

1. **PSRAM Configuration** - Biggest immediate impact
2. **Interrupt-driven sensors** - Better responsiveness
3. **Full PWM backlight** - Complete power optimization
4. **Touch input** - Major UX improvement
5. **MQTT/Bluetooth** - Connectivity features

## Notes

- Each optimization should be tested individually
- Performance metrics should be recorded before/after
- Power consumption should be measured for battery life estimates