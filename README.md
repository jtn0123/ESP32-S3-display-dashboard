# ESP32-S3 Display Dashboard (Rust)

A modern dashboard implementation for the LilyGo T-Display-S3, written in Rust using ESP-IDF.

## 🚀 Rust Quick Start

```bash
# One-time setup
espup install                    # Install ESP32 toolchain
source ~/export-esp.sh          # Add toolchain to PATH

# Build and flash
cargo build --release           # Build the firmware
cargo run --release            # Build, flash, and monitor
cargo espflash flash --monitor # Alternative flash method
```

## 📋 Prerequisites

### Install Rust ESP32 Toolchain

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install espup (ESP32 toolchain installer)
cargo install espup

# Install ESP32 toolchain
espup install

# Source the environment (add to your shell profile)
source $HOME/export-esp.sh
```

### Install Additional Tools

```bash
# Install espflash for flashing
cargo install espflash cargo-espflash
```

## 🏗️ Project Structure

```
.
├── src/
│   ├── main.rs              # Entry point with ESP-IDF
│   ├── config.rs            # Configuration management
│   ├── sensors.rs           # Sensor abstraction
│   ├── display/
│   │   ├── mod.rs          # Display driver (ST7789)
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
├── Cargo.toml              # Dependencies
├── build.rs                # Build script
└── sdkconfig.defaults      # ESP-IDF config
```

## ✨ Key Features

- **ST7789 Display Driver** - 8-bit parallel interface with optimized drawing
- **4 Interactive Screens** - System, Network, Sensors, Settings
- **Web Configuration** - Change settings via web browser
- **OTA Updates** - Update firmware over WiFi
- **Power Management** - Auto-dim, sleep modes, wake on button
- **Persistent Settings** - Configuration saved in NVS flash

## 📊 Performance

- **Boot Time**: < 2 seconds
- **Display Refresh**: 30 FPS
- **Power Consumption**: 
  - Normal: ~120mA
  - Power Save: ~60mA
  - Sleep: ~20mA
- **Binary Size**: ~500KB

## 🛠️ Development

```bash
# Check code without building
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Run tests (host-side only)
cargo test --lib

# Monitor serial output
cargo espflash monitor
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
cargo build --release

# Upload via curl
curl -X POST http://<device-ip>/ota \
  -F "firmware=@target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
```

## 🐛 Troubleshooting

### "cargo: command not found"
```bash
source $HOME/export-esp.sh
```

### Compilation Errors
Ensure you have the correct target installed:
```bash
rustup target add xtensa-esp32s3-espidf
```

### Flash Connection Issues
- Hold BOOT button while connecting USB
- Use quality USB cable
- Try different USB port

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Test thoroughly on hardware
4. Submit pull request

## 📝 Migration from Arduino

This is a complete rewrite in Rust of the original Arduino implementation. Benefits include:
- Memory safety (no buffer overflows)
- Better error handling
- Modern async/await concurrency
- Type-safe hardware abstractions
- Improved performance

## 📄 License

Same as parent project

---

**Note**: Arduino implementation has been archived in the `dashboard/` directory for reference.