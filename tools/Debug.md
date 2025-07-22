# ESP32-S3 Display Dashboard - Debugging Guide

This guide covers the enhanced debugging infrastructure for the ESP32-S3 Display Dashboard project.

## Overview

The project now includes comprehensive debugging tools:
- **JTAG Hardware Debugging** - Step through code, inspect variables
- **Display Command Tracing** - Log every ST7789 command
- **Web Debug Interface** - Remote debugging via HTTP API
- **VS Code Integration** - One-click debugging experience
- **defmt Ultra-Low Overhead Logging** - Binary logging with RTT
- **Unit Testing Infrastructure** - Host-runnable tests
- **Miri Undefined Behavior Detection** - Memory safety verification
- **cargo-espflash** - Simplified flashing workflow

## JTAG Debugging

### Built-in USB-Serial/JTAG (Recommended)

The ESP32-S3 includes a built-in USB-Serial/JTAG controller that requires no additional hardware.

#### Setup
1. **Connect USB cable** to your ESP32-S3 T-Display
2. **Install OpenOCD** (if not already installed):
   ```bash
   brew install openocd  # macOS
   # or
   sudo apt install openocd  # Linux
   ```

3. **Start OpenOCD**:
   ```bash
   openocd -f tools/debug/openocd.cfg
   ```

4. **In another terminal, start GDB**:
   ```bash
   xtensa-esp32s3-elf-gdb target/xtensa-esp32s3-espidf/debug/esp32-s3-dashboard
   ```

#### VS Code Debugging

1. **Open VS Code** in the project directory
2. **Press F5** or go to Run → Start Debugging
3. **Select "ESP32-S3 USB-JTAG Debug"** from the dropdown
4. The debugger will automatically:
   - Start OpenOCD
   - Connect GDB
   - Load symbols
   - Break at main()

#### Useful GDB Commands

```gdb
# Custom commands (from .gdbinit)
reload       # Reset, load program, and halt
restart      # Reset and continue running
display_pins # Show display GPIO states
display_regs # Show LCD_CAM register values
st7789_trace # Enable ST7789 command tracing
memmap       # Show ESP32-S3 memory regions
tasks        # List FreeRTOS tasks

# Standard commands
break function_name     # Set breakpoint
continue               # Resume execution
step                   # Step into
next                   # Step over
finish                 # Step out
print variable         # Print variable value
info registers         # Show CPU registers
backtrace             # Show call stack
```

### External JTAG Adapter

If you prefer using an external adapter (ESP-Prog, J-Link):

1. **Connect JTAG pins**:
   - TCK → GPIO39
   - TDO → GPIO40
   - TDI → GPIO41
   - TMS → GPIO42
   - GND → GND
   - 3.3V → 3.3V (optional)

2. **Use external JTAG configuration**:
   ```bash
   openocd -f interface/ftdi/esp32_devkitj_v1.cfg -f board/esp32s3.cfg
   ```

## Display Command Tracing

The display driver now includes comprehensive command tracing to debug ST7789 communication issues.

### Enabling Trace

Command tracing is enabled by default. All ST7789 commands are logged with:
- Command code (hex)
- Command name (human-readable)
- Parameter data
- Timestamp

### Viewing Trace Logs

#### Serial Console
```
[ST7789] CMD: 0x36 (MADCTL) - params: [60]
[ST7789] CMD: 0x2A (CASET) - params: [00, 00, 00, A9]
[ST7789] CMD: 0x2B (RASET) - params: [00, 23, 01, 62]
[ST7789] COLOR: 0x2C (RAMWR) - 10240 bytes
```

#### Web Interface
```bash
# Get command history
curl http://<device-ip>/api/debug/display/commands

# Get display state
curl http://<device-ip>/api/debug/display/state

# Clear command history
curl -X POST http://<device-ip>/api/debug/display/clear
```

#### Telnet Monitoring
```bash
# Connect to device telnet server
telnet <device-ip> 23

# Or use the monitoring script with filtering
./scripts/monitor-telnet.py -f "ST7789"
```

### Command Reference

Common ST7789 commands you'll see in traces:

| Code | Name    | Description                  |
|------|---------|------------------------------|
| 0x01 | SWRESET | Software reset               |
| 0x11 | SLPOUT  | Sleep out                    |
| 0x13 | NORON   | Normal display mode on       |
| 0x21 | INVON   | Display inversion on         |
| 0x29 | DISPON  | Display on                   |
| 0x2A | CASET   | Column address set           |
| 0x2B | RASET   | Row address set              |
| 0x2C | RAMWR   | Memory write                 |
| 0x36 | MADCTL  | Memory access control        |
| 0x3A | COLMOD  | Interface pixel format       |

## Web Debug Interface

### Endpoints

#### GET /api/debug/display/commands
Returns the last 100 ST7789 commands sent to the display.

Response:
```json
{
  "count": 15,
  "commands": [
    {
      "cmd": "0x36",
      "name": "MADCTL",
      "data": ["0x60"]
    },
    {
      "cmd": "0x2A",
      "name": "CASET",
      "data": ["0x00", "0x00", "0x00", "0xA9"]
    }
  ]
}
```

#### GET /api/debug/display/state
Returns current display configuration and debug state.

Response:
```json
{
  "display": {
    "width": 320,
    "height": 170,
    "orientation": "landscape",
    "driver": "ESP_LCD_I80"
  },
  "debug": {
    "trace_enabled": true,
    "command_history_size": 100
  }
}
```

#### POST /api/debug/display/clear
Clears the command history buffer.

### Debug Dashboard

Access the web interface at `http://<device-ip>/` and use browser dev tools to:
1. Monitor API responses
2. Track command sequences
3. Compare working vs non-working states

## Pixel Test Patterns

Test patterns help verify coordinate mapping and color accuracy.

### Available Patterns

1. **Solid Color Fill** - Verify basic display operation
2. **Grid Pattern** - Check pixel alignment
3. **Color Bars** - Verify RGB color order
4. **Corner Pixels** - Test coordinate boundaries
5. **Gradient** - Check color depth

### Running Tests

```bash
# Run all display tests
./compile.sh && ./scripts/flash.sh

# Monitor test output
./scripts/monitor-lcd-test.sh
```

## Troubleshooting Display Issues

### Black Screen
1. Check command trace for initialization sequence
2. Verify power pins (GPIO15 for LCD power, GPIO38 for backlight)
3. Compare command sequence with working reference
4. Check MADCTL value (should be 0x60 for landscape)

### Wrong Colors
1. Check RGB/BGR order in initialization
2. Verify pixel format (RGB565 = 0x55)
3. Check endianness of color data

### Shifted Image
1. Verify CASET/RASET parameters
2. Check X/Y offset values (Y=35 for T-Display-S3)
3. Verify display dimensions match physical panel

### Performance Issues
1. Monitor FPS counter in logs
2. Check I80 bus frequency settings
3. Verify DMA configuration
4. Use web API to check command frequency

## Advanced Debugging

### Memory Dumps
```gdb
# Dump GPIO registers
x/32wx 0x60004000

# Dump LCD_CAM registers  
x/32wx 0x60041000

# Dump frame buffer (if in PSRAM)
x/1000hx 0x3C000000
```

### Logic Analyzer
For hardware-level debugging:
1. Connect logic analyzer to display data pins
2. Trigger on WR signal
3. Decode parallel bus protocol
4. Compare with expected ST7789 timing

### Performance Profiling
```rust
// Add timing markers in code
let start = std::time::Instant::now();
// ... operation ...
log::info!("Operation took: {:?}", start.elapsed());
```

## Common Issues and Solutions

### Issue: OpenOCD Can't Connect
**Solution**: 
- Ensure no other program is using the USB port
- Try `sudo` if permission denied
- Check USB cable quality
- Verify CONFIG_ESP_CONSOLE_USB_SERIAL_JTAG=y in sdkconfig

### Issue: GDB Can't Load Symbols
**Solution**:
- Build with debug profile: `./compile.sh --debug`
- Ensure path to ELF file is correct
- Use absolute paths in GDB

### Issue: Breakpoints Not Hit
**Solution**:
- Ensure optimization is disabled for debugging
- Check if code is actually executed
- Try hardware breakpoints: `hbreak function_name`

### Issue: Display Commands Not Traced
**Solution**:
- Verify trace module is included in build
- Check if traced functions are being called
- Monitor serial output for trace messages

## Tips and Best Practices

1. **Always increment version** when testing display changes
2. **Save command traces** from working configurations
3. **Use telnet monitoring** for real-time debugging without USB
4. **Compare init sequences** between working and non-working code
5. **Document findings** in issue-specific markdown files
6. **Use web API** for remote debugging without physical access

## defmt - Ultra-Low Overhead Logging

defmt provides binary logging with minimal performance impact, perfect for timing-sensitive display debugging.

### Setup

1. **Build with defmt feature**:
   ```bash
   cargo build --features defmt
   ```

2. **Run with probe-rs**:
   ```bash
   probe-rs run --chip esp32s3
   ```

### Usage in Code

```rust
use defmt::{info, debug, trace};

// Automatic timestamps
defmt::info!("Display init started");

// Efficient parameter formatting
defmt::debug!("ST7789 CMD: {:#04x} params: {=[u8]:x}", cmd, data);

// Performance measurement
let timer = DefmtTimer::start("render");
// ... render code ...
timer.stop(); // Automatically logs duration
```

### Benefits
- Near-zero overhead when disabled
- Automatic timestamps
- Binary format reduces bandwidth
- Works alongside JTAG debugging

## Unit Testing Infrastructure

The project now includes `dashboard-core` - a library crate with hardware-independent logic that can be tested on the host.

### Running Tests

```bash
# Run all tests
cargo test -p dashboard-core

# Run with output
cargo test -p dashboard-core -- --nocapture

# Run specific test
cargo test -p dashboard-core test_rgb_conversion
```

### Test Coverage

- **Color utilities** - RGB conversions, blending
- **Display math** - Rectangle operations, coordinate transforms
- **Configuration** - Serialization, validation

### Writing New Tests

Add tests to `dashboard-core/src/*.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(42), 84);
    }
}
```

## Miri - Undefined Behavior Detection

Miri catches memory safety issues in safe Rust code.

### Setup

```bash
# Install miri (nightly required)
rustup +nightly component add miri

# Run miri on dashboard-core
cd dashboard-core
cargo +nightly miri test
```

### What Miri Catches

- Use after free
- Out of bounds access
- Data races
- Uninitialized memory
- Invalid enum discriminants

### Limitations

- Only works on host-runnable code
- Cannot test ESP-IDF specific code
- Slower than normal tests

## cargo-espflash

Simplified flashing and monitoring tool.

### Installation

```bash
cargo install cargo-espflash
```

### Usage

```bash
# Flash and monitor
cargo espflash flash --monitor

# Just monitor
cargo espflash monitor

# Flash specific binary
cargo espflash flash --release --monitor

# Save to file
cargo espflash save-image esp32s3 output.bin
```

### Benefits
- Automatic port detection
- Integrated monitor
- Progress indicators
- Binary image export

## Code Quality Tools

### Clippy (Already in CI)

```bash
# Run locally
cargo clippy -- -D warnings

# With all targets
cargo clippy --all-targets --all-features -- -D warnings

# Auto-fix suggestions
cargo clippy --fix
```

### rustfmt

```bash
# Check formatting
cargo fmt -- --check

# Auto-format
cargo fmt
```

## Memory Debugging

### Heap Tracing (ESP-IDF)

Enable in `sdkconfig.defaults`:
```
CONFIG_HEAP_TRACING_STANDALONE=y
CONFIG_HEAP_POISONING_LIGHT=y
```

Use in code:
```rust
unsafe {
    esp_idf_sys::heap_trace_start(
        esp_idf_sys::heap_trace_mode_t_HEAP_TRACE_LEAKS
    );
    
    // ... code to trace ...
    
    esp_idf_sys::heap_trace_stop();
    esp_idf_sys::heap_trace_dump();
}
```

## Complete Debugging Workflow

1. **Development Phase**:
   - Use `defmt` for lightweight logging
   - Run `cargo clippy` before commits
   - Test with `cargo test -p dashboard-core`

2. **Debugging Display Issues**:
   - Enable command trace
   - Monitor via telnet: `./scripts/monitor-telnet.py`
   - Check web API: `curl http://<ip>/api/debug/display/commands`
   - Use pixel test patterns

3. **Deep Debugging**:
   - Start OpenOCD + GDB
   - Set breakpoints in display driver
   - Inspect LCD_CAM registers
   - Compare command sequences

4. **Memory Issues**:
   - Enable heap tracing
   - Run with `defmt` memory tracking
   - Use `miri` on testable code

5. **Performance**:
   - Use `DefmtTimer` for measurements
   - Monitor FPS via telnet
   - Profile with JTAG sampling

## Resources

- [ESP32-S3 Technical Reference](https://www.espressif.com/sites/default/files/documentation/esp32-s3_technical_reference_manual_en.pdf)
- [ST7789 Datasheet](https://www.crystalfontz.com/controllers/Sitronix/ST7789V/)
- [OpenOCD Documentation](https://openocd.org/doc/html/index.html)
- [GDB Quick Reference](https://darkdust.net/files/GDB%20Cheat%20Sheet.pdf)
- [defmt Book](https://defmt.ferrous-systems.com/)
- [Miri Documentation](https://github.com/rust-lang/miri)
- [probe-rs Documentation](https://probe.rs/)