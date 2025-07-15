# ESP32-S3 Display Dashboard

[![Rust CI](https://github.com/jtn0123/ESP32-S3-Display-Dashboard/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/jtn0123/ESP32-S3-Display-Dashboard/actions/workflows/rust-ci.yml)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://jtn0123.github.io/ESP32-S3-Display-Dashboard/)

A modern, high-performance dashboard implementation for the LilyGo T-Display-S3, written in Rust using ESP-IDF.

## 🚀 Quick Start

```bash
# One-time setup
./setup-toolchain.sh            # Install complete toolchain
source ~/esp-env.sh             # Load environment

# Build and flash
./flash.sh                      # Compile and flash with monitor
./compile.sh                    # Compile only
```

## 📋 Prerequisites

### macOS (ARM64/M1/M2/M3)

This project includes optimized support for Apple Silicon Macs. The toolchain handles the ARM64 architecture automatically.

### Install Rust ESP32 Toolchain

```bash
# Quick setup (recommended)
./setup-toolchain.sh

# Or manual setup:
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install ESP toolchain
cargo install espup
espup install

# 3. Source the environment (add to your shell profile)
source ~/export-esp.sh
```

## 🏗️ Project Structure

```
.
├── src/
│   ├── main.rs              # Entry point with ESP-IDF
│   ├── config.rs            # Configuration management
│   ├── sensors/             # Sensor implementations
│   ├── display/
│   │   ├── mod.rs          # Display driver (ST7789)
│   │   ├── lcd_bus.rs      # Low-level LCD bus interface
│   │   ├── colors.rs       # Color definitions
│   │   └── font5x7.rs      # Bitmap font
│   ├── network/
│   │   ├── wifi.rs         # WiFi manager
│   │   ├── ota.rs          # OTA updates
│   │   └── web_server.rs   # Web configuration
│   ├── system/
│   │   ├── button.rs       # Button handling
│   │   ├── power.rs        # Power management
│   │   └── storage.rs      # Persistent storage
│   └── ui/
│       └── mod.rs          # UI screens
├── Cargo.toml              # Dependencies (pinned versions)
├── build.rs                # Build script
├── sdkconfig.defaults      # ESP-IDF config
├── compile.sh              # Build script
├── flash.sh                # Build & flash script
└── setup-toolchain.sh      # Toolchain installer
```

## ✨ Key Features

- **ST7789 Display Driver** - 8-bit parallel interface with optimized drawing
- **Dirty Rectangle Tracking** - Only update changed screen regions
- **Dynamic Frequency Scaling** - CPU scales 80-240MHz based on load
- **Web Configuration** - Change settings via web browser
- **OTA Updates** - Update firmware over WiFi
- **Power Management** - Auto-dim, sleep modes, WiFi power save
- **Performance Monitoring** - Built-in telemetry in main loop

## 📊 Performance Optimizations

This build includes several performance enhancements:

- **Link-Time Optimization (LTO)** - Reduces binary size by ~15%
- **Size-Optimized Build** - Compiler flag `-Os` for smaller code
- **WiFi Power Save** - MIN_MODEM mode after connection
- **Display Optimizations** - Dirty rectangle tracking, auto-dimming
- **DMA Support** - Hardware-accelerated display updates

## 🛠️ Development

### Building

```bash
# Compile only (release mode - optimized)
./compile.sh

# Compile in debug mode
./compile.sh --debug

# Clean build
./compile.sh --clean

# Verbose output
./compile.sh --verbose
```

### Flashing

```bash
# Flash with auto-detected port
./flash.sh

# Flash to specific port
./flash.sh --port /dev/tty.usbmodem14201

# Flash without monitor
./flash.sh --no-monitor

# Flash debug build
./flash.sh --debug
```

### Other Commands

```bash
# Check code without building
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Monitor serial output only
espflash monitor

# Check toolchain status
./check-toolchain.sh
```

## 🔧 Configuration

Connect to the device's IP address (shown on screen or serial output) to access the web configuration interface:

```
http://<device-ip>
```

### Configurable Settings:
- WiFi credentials
- Display brightness
- Auto-dim timeout
- OTA update URL
- Update intervals

## 📡 OTA Updates

Build and upload firmware updates over WiFi:

```bash
# Build OTA binary
./compile.sh --release

# Upload via curl
curl -X POST http://<device-ip>/ota \
  -F "firmware=@target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
```

## 🐛 Troubleshooting

### ARM64 macOS Build Issues

This project includes automatic handling of ESP toolchain issues on Apple Silicon. The build scripts use a wrapper to ensure compatibility.

### Common Issues

**"cargo: command not found"**
```bash
source ~/esp-env.sh
```

**Build Failures**
```bash
# Clean and rebuild
./compile.sh --clean
```

**Flash Connection Issues**
- The ESP32-S3 T-Display auto-enters download mode - no button needed
- Use a quality USB-C cable
- Try different USB ports

**Port Detection**
```bash
# List available ports
ls /dev/tty.usb* /dev/cu.usb*

# Flash with specific port
./flash.sh --port /dev/tty.usbmodem14201
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Test thoroughly on hardware
4. Submit pull request

## 🔄 CI/CD

This project includes comprehensive CI workflows:

### Automated Checks
- **Code Formatting** - Enforces consistent style with `cargo fmt`
- **Linting** - Strict clippy checks with warnings as errors
- **Security Audit** - Checks dependencies for known vulnerabilities
- **Binary Size Tracking** - Monitors size changes in PRs
- **Build Matrix** - Tests both debug and release builds

## 📝 Migration from Arduino

This is a complete rewrite in Rust. The Arduino implementation has been moved to the `legacy/` directory for reference.

### Why Rust?
- **Memory Safety** - No buffer overflows or use-after-free
- **Performance** - Zero-cost abstractions, better optimization
- **Modern Tooling** - Cargo, integrated testing, excellent error messages
- **Type Safety** - Catch errors at compile time
- **Smaller Binaries** - ~1MB vs 1.4MB Arduino

### Key Improvements
- Modular architecture with clear separation of concerns
- Hardware abstraction layer for display and sensors
- Async/await for concurrent operations
- Comprehensive error handling with Result types
- DMA-accelerated display updates
- Dirty rectangle tracking for efficient rendering

## 📄 License

Same as parent project

---

**Note**: Arduino implementation has been archived in the `legacy/` directory for reference.