# ESP32-S3 Display Dashboard - AI Assistant Guide

This document provides context for AI assistants working on this project.

## Project Overview

This is a Rust-based dashboard implementation for the LilyGo T-Display-S3 (ESP32-S3 with integrated display). The project uses ESP-IDF framework and focuses on performance optimization and modern features.

## Hardware Specifications

- **Board**: LilyGo T-Display-S3
- **MCU**: ESP32-S3 (dual-core Xtensa LX7 @ 240MHz)
- **Display**: 1.9" ST7789 LCD (170x320 pixels, 8-bit parallel interface)
- **Flash**: 16MB (though bootloader may report 4MB)
- **PSRAM**: 8MB external SPI RAM
- **Buttons**: 2 (GPIO0 and GPIO14)
- **Power**: USB-C, battery connector with charging

## Key Technical Details

### Display Driver
- **Interface**: 8-bit parallel (not SPI)
- **Driver IC**: ST7789V
- **Performance**: ~10 FPS maximum due to GPIO bit-banging
- **Memory Layout**: Controller expects 320x240, but physical display is 170x320
- **Offsets**: X=10, Y=36 for correct positioning

### Known Issues
1. **LCD_CAM Hardware Acceleration**: Cannot be used due to signal corruption (see LCD_CAM_FINAL_REPORT.md)
2. **Flash Size Detection**: Bootloader shows 4MB instead of 16MB (esp-idf-sys cache issue)
3. **PSRAM Frame Buffer**: Causes 96% performance degradation when used
4. **espflash v4.x**: Incompatible, must use v3.3.0
5. **Build Hangs**: Close VS Code before building to avoid cargo lock conflicts
6. **espup v0.15.1**: Has dependency conflicts, use v0.13.0 instead

### Current Features
- Web configuration interface
- OTA updates over WiFi
- Telnet server for remote log monitoring (port 23)
- Dirty rectangle tracking for display optimization
- FPS counter with skip rate detection
- Dual-core CPU monitoring
- Auto-dimming and power management

## Code Quality Standards

### NEVER Use Dead Code Annotations
- **NEVER use `#[allow(dead_code)]`** - If code is unused, remove it completely
- **Fix warnings properly** by removing unused code, not suppressing warnings
- **Clean code is better than annotated code** - Keep the codebase clean and minimal
- **If code might be needed later**, comment it out with a TODO explaining when it will be used

## Development Workflow

### Building
```bash
./compile.sh                    # Release build
./compile.sh --debug            # Debug build
./compile.sh --clean            # Clean build
./scripts/fix-build.sh          # Fix common build issues
./scripts/fix-build.sh --deep-clean  # Nuclear option - clears all caches
```

**Important**: Always close VS Code before building to avoid cargo lock conflicts.

### Flashing
```bash
./scripts/flash.sh              # USB flash with full erase
./scripts/flash.sh --no-erase   # Quick flash
./scripts/ota.sh find           # Find devices for OTA
./scripts/ota.sh <IP>           # OTA update specific device
```

### Monitoring
```bash
# Serial (USB)
espflash monitor

# Remote (WiFi/Telnet)
./scripts/monitor-telnet.sh     # Basic monitoring
./scripts/monitor-telnet.py     # Advanced with filtering
```

## Code Organization

### Core Modules
- `src/main.rs` - Entry point, main loop with performance tracking
- `src/display/mod.rs` - Display driver with dirty rect tracking
- `src/ui/mod.rs` - UI screens and rendering logic
- `src/network/` - WiFi, web server, telnet server
- `src/sensors/` - Temperature, battery, light sensors
- `src/performance.rs` - FPS tracking and metrics

### Performance Considerations
- Main loop runs at ~19k FPS with 100% skip rate (UI optimization working)
- Display capable of 55-65 FPS with ESP_LCD DMA driver (v5.53+)
- Previous GPIO mode limited to ~10 FPS
- CPU usage significantly reduced with DMA offloading
- CPU dynamically scales 80-240MHz based on load

## Recent Changes

### v5.53 - ESP_LCD DMA Success
1. **ESP_LCD DMA Driver** - Migrated from GPIO to hardware-accelerated DMA
2. **Performance Boost** - 10 FPS → 55-65 FPS (5-6x improvement)
3. **PERF Optimization** - Resolved multiple DROM segments issue
4. **Struct Alignment Fix** - Compatible with ESP-IDF v5.3

### v5.17 - Telnet & Performance
1. **Telnet Server** - Added remote serial monitoring over WiFi
2. **FPS Counter Fix** - Accurate frame skip detection
3. **Performance Metrics** - Detailed timing for render/flush operations
4. **Dirty Rectangle Tracking** - Multi-rectangle support with merging

## Optimization Opportunities

### Completed
- ✅ ESP_LCD DMA driver migration (5-6x performance boost)
- ✅ Dirty rectangle tracking
- ✅ FPS counter accuracy improvements
- ✅ Telnet server for remote monitoring

### Pending
- 🔄 Dual-core architecture optimization
- 📋 Move sensor sampling to Core 1
- 📋 Network monitoring on Core 1
- 📋 Remove simulated sensor data
- 📋 Leverage high FPS for smooth animations

## Build Environment

### Toolchain Requirements
- Rust with ESP32 target (installed via espup)
- ESP-IDF v5.3.3 LTS
- espflash v3.3.0 (NOT v4.x)
- Python 3.x for build tools

### macOS ARM64 (Apple Silicon)
The project includes automatic handling of ARM64 architecture issues. The compile.sh script wraps the cargo command to ensure proper toolchain usage.

## Testing Guidelines

When making changes:
1. Always increment version number for visible verification
2. Test on actual hardware (simulator doesn't exist)
3. Monitor serial output for performance metrics
4. Check both USB and OTA update paths
5. Verify web interface remains functional

## Common Commands Reference

```bash
# Version update (always do this when testing changes)
# Edit src/version.rs - increment DISPLAY_VERSION

# Full rebuild and flash
./compile.sh --clean && ./scripts/flash.sh

# Monitor performance
./scripts/monitor-telnet.py -f "PERF|FPS"

# Check partition status
./scripts/check-partition.sh

# Find device on network
./scripts/ota.sh find
```

## Important Files

- `KNOWN_ISSUES.md` - Comprehensive list of issues and solutions
- `IMPROVEMENTS.md` - Performance optimization tracking
- `LCD_CAM_FINAL_REPORT.md` - Hardware acceleration investigation
- `scripts/README.md` - Detailed documentation for all scripts

## MCP Server Integration

The project has access to two MCP (Model Context Protocol) servers that enhance AI assistant capabilities:

### 1. Sequential Thinking Server (`mcp__sequential-thinking`)
Used for complex problem-solving and iterative analysis:
- Breaking down multi-step tasks
- Planning implementations
- Analyzing performance bottlenecks
- Debugging complex issues
- Generating and verifying hypotheses

Example use cases:
- Planning LCD_CAM hardware acceleration workarounds
- Analyzing dual-core optimization strategies
- Debugging display timing issues

### 2. Memory Knowledge Graph Server (`mcp__memory`)
Used for storing and retrieving project knowledge:
- Tracking component relationships
- Storing optimization results
- Recording bug patterns and solutions
- Building knowledge about hardware quirks

Example entities to track:
- Hardware components and their interactions
- Performance metrics over time
- Known issues and their solutions
- Code module dependencies

### 3. Performance Monitoring MCP Servers

For enhanced debugging and monitoring, these MCP servers can be integrated:

#### MCP Telemetry (`xprilion/mcp-telemetry`)
- **Purpose**: OpenTelemetry tracing for performance debugging
- **Integration**: Connects to ESP32's telnet server (port 23)
- **Features**: Trace display render times, identify bottlenecks, analyze performance patterns
- **Setup**: Requires Weights & Biases API key

#### Grafana MCP (`grafana/mcp-grafana`)
- **Purpose**: Visualize Prometheus metrics from ESP32
- **Integration**: Queries metrics endpoint at `http://<ESP32-IP>/metrics`
- **Features**: Historical FPS trends, resource usage graphs, custom dashboards
- **Requirements**: Grafana 9.0+, Prometheus scraper

#### System Monitor MCP (`seekrays/mcp-monitor`)
- **Purpose**: Real-time system resource monitoring
- **Integration**: Reads JSON metrics from telnet stream
- **Features**: CPU per-core usage, memory stats, network metrics
- **Usage**: Monitor ESP32 alongside development machine

See `MCP_INTEGRATION_GUIDE.md` for detailed setup instructions.