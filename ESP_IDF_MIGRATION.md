# ESP-IDF Rust Migration Plan

## Overview

This document tracks the migration from Arduino to Rust using ESP-IDF (std) approach for the ESP32-S3 Display Dashboard.

## Why ESP-IDF Instead of Pure no_std?

- **WiFi/OTA Built-in**: No need to implement from scratch
- **Proven Stack**: Battle-tested networking and system features
- **Faster Development**: Focus on application logic, not low-level drivers
- **Future Features**: Easy to add BLE, HTTPS, MQTT, etc.

## Migration Status

- [x] Decision to use ESP-IDF approach
- [x] Update Cargo.toml for ESP-IDF
- [x] Create sdkconfig.defaults
- [x] Implement main.rs with ESP-IDF
- [x] Create configuration management system
- [x] Implement WiFi manager
- [x] Implement OTA service
- [x] Port display driver to ESP-IDF HAL
- [x] Create web configuration server
- [x] Port UI screens
- [x] Implement sensor readings
- [x] Add power management
- [ ] Remove Arduino code (pending hardware validation)
- [x] Update CI/CD pipeline

**Migration Complete!** 🎉 The Rust implementation is feature-complete and ready for hardware testing.

## Project Structure

```
esp32-s3-display-dashboard/
├── src/
│   ├── main.rs              # Entry point with ESP-IDF
│   ├── config.rs            # Configuration management
│   ├── display/
│   │   ├── mod.rs          # Display trait and types
│   │   ├── st7789_idf.rs  # ST7789 driver using ESP-IDF
│   │   └── graphics.rs     # Drawing primitives
│   ├── network/
│   │   ├── mod.rs          # Network management
│   │   ├── wifi.rs         # WiFi connection handling
│   │   ├── ota.rs          # OTA update service
│   │   └── web_server.rs   # Configuration web interface
│   ├── ui/
│   │   ├── mod.rs          # UI framework
│   │   ├── screens/        # Individual screens
│   │   ├── widgets/        # Reusable UI components
│   │   └── theme.rs        # Color schemes and styling
│   ├── sensors/
│   │   ├── mod.rs          # Sensor traits
│   │   ├── battery.rs      # Battery monitoring
│   │   └── temperature.rs  # Temperature reading
│   └── system/
│       ├── mod.rs          # System utilities
│       ├── power.rs        # Power management
│       └── storage.rs      # NVS configuration storage
├── Cargo.toml              # Rust dependencies
├── build.rs                # Build script for ESP-IDF
├── sdkconfig.defaults      # ESP-IDF configuration
├── partitions.csv          # Flash partition table
└── .cargo/
    └── config.toml         # Cargo build configuration
```

## Key Differences from Pure no_std

1. **Memory Management**: Use standard `Vec`, `String`, `HashMap` instead of `heapless`
2. **Error Handling**: Use `anyhow::Result` for cleaner error propagation
3. **Async Runtime**: Can use `tokio` or `async-std` if needed (though Embassy still works)
4. **Logging**: Use standard `log` crate with ESP-IDF backend
5. **Threading**: Full `std::thread` support for background tasks

## Development Workflow

```bash
# One-time setup
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-unknown-linux-gnu -o espup
chmod +x espup
./espup install
source ~/export-esp.sh

# Daily commands
cargo build --release           # Build firmware
cargo run --release            # Build, flash, and monitor
cargo test --lib              # Run host-side tests
cargo clippy -- -D warnings   # Lint code
```

## Performance Targets

- Boot time: < 500ms (acceptable with ESP-IDF)
- Display refresh: 30 FPS minimum
- OTA update: < 30 seconds for 1MB binary
- WiFi connection: < 5 seconds
- Idle power: < 50mA with WiFi connected

## Migration Phases

### Phase 1: Foundation (Current)
- Set up ESP-IDF build system
- Basic display output
- Serial logging

### Phase 2: Core Features
- Display driver with DMA
- Button input handling
- Basic UI rendering

### Phase 3: Connectivity
- WiFi connection manager
- OTA update system
- Web configuration interface

### Phase 4: Polish
- Power management
- Persistent settings
- Error recovery
- Performance optimization

### Phase 5: Cleanup
- Remove Arduino code
- Update all documentation
- Final testing