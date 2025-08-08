# ESP32-S3 Display Dashboard

[![Rust CI](https://github.com/jtn0123/ESP32-S3-Display-Dashboard/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/jtn0123/ESP32-S3-Display-Dashboard/actions/workflows/rust-ci.yml)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://jtn0123.github.io/ESP32-S3-Display-Dashboard/)

A modern, high-performance dashboard implementation for the LilyGo T-Display-S3, written in Rust using ESP-IDF.

## 🚀 Quick Start

```bash
# One-time setup
cargo install espup --version 0.13.0  # Note: v0.15.1 has issues
espup install --targets esp32s3 --std
source ~/export-esp.sh                 # Load environment

# Build and flash
./compile.sh                    # Build firmware
./scripts/flash.sh              # Flash via USB
./scripts/ota.sh find           # Find devices for OTA update

# See SETUP_GUIDE.md for detailed instructions and troubleshooting
```

## 📋 Prerequisites

### macOS (ARM64/M1/M2/M3)

This project includes optimized support for Apple Silicon Macs. The toolchain handles the ARM64 architecture automatically.

### Install Rust ESP32 Toolchain

```bash
# Recommended setup:
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install ESP toolchain (use specific version to avoid issues)
cargo install espup --version 0.13.0
espup install --targets esp32s3 --std

# 3. Source the environment (add to your shell profile)
source ~/export-esp.sh
```

⚠️ **Important**: Close VS Code before building to avoid lock conflicts. See [SETUP_GUIDE.md](SETUP_GUIDE.md) for common issues and solutions.

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
├── scripts/
│   ├── flash.sh            # Build & flash script
│   ├── ota.sh              # OTA update script
│   └── check-partition.sh  # Partition checker
└── setup-toolchain.sh      # Toolchain installer
```

## ✨ Key Features

### Display & UI
- **ESP_LCD DMA Driver** - Hardware-accelerated 55-65 FPS (v5.53+)
- **Dirty Rectangle Tracking** - Only update changed screen regions
- **Power Management** - Screen dimming/timeout with battery awareness
- **Visual Alerts** - Temperature, WiFi signal, and battery warnings
- **Multiple UI Screens** - System info, sensors, network, OTA status

### Networking & Updates
- **mDNS Support** - Access via `http://esp32.local/`
- **OTA Updates** - Secure wireless firmware updates with SHA256 validation
- **WiFi Auto-Reconnect** - Automatic recovery with exponential backoff
- **Remote Serial Monitoring** - Telnet server for wireless log streaming
- **Health Endpoint** - `/health` for uptime monitoring tools
- **Remote Control** - `/restart` endpoint and telnet-control.py script

### Performance & Reliability
- **Dual-Core Architecture** - Sensor monitoring on Core 1, UI on Core 0
- **Dynamic Frequency Scaling** - CPU scales 80-240MHz based on load
- **Persistent Uptime Tracking** - Total device uptime across reboots
- **Real Sensor Data** - Temperature, battery, and CPU monitoring
- **Performance Telemetry** - Built-in FPS and timing metrics

### Security & Development
- **OTA Password Protection** - Basic authentication for updates
- **SHA256 Firmware Validation** - Prevents corrupted updates
- **Development Scripts** - Quick flash, log filtering, and monitoring tools
- **Enhanced Logging** - Color-coded output with timestamps and module names
- **Compile-time WiFi Config** - Credentials compiled into firmware

## 📊 Performance Optimizations

This build includes several performance enhancements:

- **Link-Time Optimization (LTO)** - Reduces binary size by ~15%
- **Size-Optimized Build** - Compiler flag `-Os` for smaller code
- **WiFi Power Save** - MIN_MODEM mode after connection
- **Display Optimizations** - Dirty rectangle tracking, auto-dimming
- **Reliable GPIO Driver** - 10 FPS stable performance

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

### Flashing & OTA Updates

```bash
# USB Flash (always works, sets up OTA)
./scripts/flash.sh              # Full flash with erase
./scripts/flash.sh --no-erase   # Quick flash (preserves WiFi)

# Wireless OTA Updates  
./scripts/ota.sh find           # Find devices on network
./scripts/ota.sh 192.168.1.100  # Update specific device
./scripts/ota.sh auto           # Update all devices

# Diagnostics
./scripts/check-partition.sh    # Check partition status
```

See `scripts/README.md` for detailed documentation.

### Performance Monitoring (v5.37+)

The ESP32 now includes built-in Prometheus metrics for monitoring performance:

```bash
# One-time setup
./setup-mcp-servers.sh          # Install monitoring tools

# Start monitoring
./start-monitoring.sh            # Starts Prometheus & Grafana

# View dashboards
http://localhost:3000/d/esp32-dashboard/   # Grafana (admin/admin)

# Stop monitoring
./stop-monitoring.sh             # Stops monitoring services
```

Features:
- Real-time FPS, CPU, temperature, and memory tracking
- Historical performance graphs
- MCP integration for AI-assisted monitoring
- Zero performance impact on ESP32

See `MONITORING_SETUP.md` for detailed instructions.

#### Important: espflash Version Compatibility

This project requires **espflash v3.3.0** due to compatibility issues with v4.x:

```bash
# Check your espflash version
espflash --version

# If you have v4.x, downgrade to v3.3.0:
cargo install espflash@3.3.0 --force
cargo install cargo-espflash@3.3.0 --force
```

#### Flash Size Configuration

The ESP32-S3 T-Display has 16MB flash, but the bootloader may incorrectly detect only 4MB due to a known issue with esp-idf-sys. This doesn't affect functionality but requires manual flash size specification:

```bash
# Method 1: Use espflash with explicit flash size (recommended)
espflash flash --flash-size 16mb --port /dev/cu.usbmodem101 target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard

# Method 2: Use esptool.py directly for full control
.embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py \
  --chip esp32s3 --port /dev/cu.usbmodem101 --baud 921600 \
  --before default_reset --after hard_reset write_flash \
  --flash_mode dio --flash_freq 40m --flash_size 16MB \
  0x0 target/xtensa-esp32s3-espidf/release/bootloader.bin \
  0x8000 target/xtensa-esp32s3-espidf/release/partition-table.bin \
  0x10000 target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

**Note**: The bootloader will still report "SPI Flash Size : 4MB" during boot, but the application has access to the full 16MB when flashed with these methods.

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

# Monitor over WiFi (telnet)
./scripts/monitor-telnet.sh              # Using mDNS
./scripts/monitor-telnet.sh 192.168.1.x  # Using IP
./scripts/monitor-telnet.py -f "PERF"    # Filter logs

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

## 🔍 Remote Serial Monitoring

The device runs a telnet server on port 23 for wireless log streaming:

### Basic Monitoring
```bash
# Connect using mDNS (automatic discovery)
./scripts/monitor-telnet.sh

# Connect to specific IP
./scripts/monitor-telnet.sh 192.168.1.100

# Auto-reconnect on disconnect
./scripts/monitor-telnet.sh -r

# Save logs to file
./scripts/monitor-telnet.sh -s -f debug.log
```

### Advanced Monitoring (Python)
```bash
# Filter specific log levels
./scripts/monitor-telnet.py -f "ERROR|WARN"

# Show only performance metrics
./scripts/monitor-telnet.py -f "PERF|CORES"

# Highlight patterns with colors
./scripts/monitor-telnet.py -H "temperature.*[4-9][0-9]" RED

# Statistics only mode
./scripts/monitor-telnet.py --stats-only

# Scan network for devices
./scripts/monitor-telnet.py --scan
```

### Remote Control (New!)
```bash
# Enhanced telnet client with commands
./scripts/telnet-control.py esp32.local

# Available commands:
#   help     - Show available commands
#   stats    - Display device statistics
#   restart  - Restart the device remotely
#   filter X - Filter logs containing X
#   clear    - Clear the screen

# Scan for devices
./scripts/telnet-control.py --scan
```

### Direct Connection
```bash
# Using telnet
telnet esp32-dashboard.local 23

# Using netcat
nc esp32-dashboard.local 23
```

The telnet server provides:
- Real-time log streaming
- Last 100 log messages on connect
- Performance metrics (FPS, CPU, memory)
- Support for multiple simultaneous connections
- mDNS service discovery (_telnet._tcp)

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
./scripts/flash.sh --port /dev/tty.usbmodem14201
```

**"ESP-IDF App Descriptor missing" Error (espflash 4.x)**

This error occurs with espflash v4.x due to a section name mismatch. Solutions:
1. Downgrade to espflash 3.3.0 (recommended)
2. Use `--check-app-descriptor=false` flag with v4.x
3. Use esptool.py directly (see Flash Size Configuration above)

**"SPI Flash Size : 4MB" Boot Error**

If the bootloader reports 4MB instead of 16MB:
1. Clean build: `cargo clean`
2. Rebuild: `./compile.sh --release`
3. Flash with explicit size: `espflash flash --flash-size 16mb ...`

## 📚 Documentation

### Key Documents
- **[KNOWN_ISSUES.md](KNOWN_ISSUES.md)** - Consolidated list of known issues and attempted solutions
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture and design decisions
- **[OTA_DOCUMENTATION.md](OTA_DOCUMENTATION.md)** - Complete OTA update guide
- **[scripts/README.md](scripts/README.md)** - Detailed flashing and OTA scripts documentation

### Technical Reports
- **[LCD_CAM_FINAL_REPORT.md](LCD_CAM_FINAL_REPORT.md)** - Hardware acceleration investigation results
- **[BOOTLOADER_INVESTIGATION_REPORT.md](BOOTLOADER_INVESTIGATION_REPORT.md)** - Flash size detection analysis
- **[DISPLAY_COMMAND_INVESTIGATION.md](DISPLAY_COMMAND_INVESTIGATION.md)** - Display driver debugging

### Setup Guides
- **[WIFI_SETUP.md](WIFI_SETUP.md)** - WiFi configuration instructions
- **[FLASHING_GUIDE.md](FLASHING_GUIDE.md)** - Detailed flashing procedures

## 🧪 Testing

The project includes comprehensive testing infrastructure:

### Test Runners
- **`./scripts/run-rust-tests.sh`** - Run host-based Rust unit tests
- **`./scripts/run-python-tests.sh`** - Run Python integration tests against device
- **`./scripts/run-tests.sh`** - Run all tests (Rust + Python)

### Running Tests

```bash
# Run only Rust unit tests (fast, no device needed)
./scripts/run-rust-tests.sh

# Run Python integration tests (requires device)
./scripts/run-python-tests.sh --device-ip 192.168.1.100

# Run all tests
./scripts/run-tests.sh --all

# Run with verbose output
./scripts/run-rust-tests.sh --verbose
```

### Test Structure
- **Rust Unit Tests**: Located in `src/` files with `#[cfg(test)]` modules
- **Host Tests**: Platform-independent tests in `host-tests/` directory  
- **Python Integration Tests**: End-to-end tests in `tests/python/`

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Run tests: `./scripts/run-tests.sh --all`
4. Test thoroughly on hardware
5. Submit pull request

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
- Dirty rectangle tracking for efficient rendering
- Dual-core processing support

## 📄 License

Same as parent project

---

**Note**: Arduino implementation has been archived in the `legacy/` directory for reference.

## 🐞 Debugging & Diagnostics

The firmware includes enhanced, development-friendly debugging:

- Enhanced logger (serial + telnet)
  - Timestamped, colorized logs on serial; plain logs over telnet
  - Forwarded to an in-memory ring buffer for web/SSE consumption
  - Default level is DEBUG during development

- Runtime log level control
  - Set via HTTP:
    ```bash
    # JSON body
    curl -X POST http://<device-ip>/api/v1/debug/log-level -d '{"level":"debug"}' -H 'Content-Type: application/json'

    # or via query param
    curl -X POST 'http://<device-ip>/api/v1/debug/log-level?level=info'
    ```
  - Valid levels: off, error, warn, info, debug, trace

- Recent logs via HTTP and SSE
  - REST: `GET /api/v1/logs/recent?count=50` (default 50, max 500)
  - SSE: `GET /sse/logs` (server-sent events stream of recent logs)
  - Web page: `GET /logs`

- Health and metrics
  - Health: `GET /health` (lightweight JSON)
  - Prometheus: `GET /metrics` (optimized formatter; safe on contention)

- Panic and crash diagnostics
  - Custom panic hook logs location and message
  - Memory and crash diagnostics are dumped on panic
  - Periodic diagnostics thread logs heap and active request insights

- Telnet logging (wireless serial)
  - Port 23; includes last 100 log lines on connect
  - Scripts:
    ```bash
    ./scripts/monitor-telnet.sh            # mDNS
    ./scripts/monitor-telnet.sh <ip>       # by IP
    ./scripts/monitor-telnet.py -f "ERROR|WARN"  # filter
    ```

- Notes
  - In-memory log buffer is bounded (default ~2000 entries; drop-oldest policy)
  - Low-latency non-blocking appends; avoids stalls in hot paths
  - Safe locking and error handling across web/SSE/WS paths to prevent panics