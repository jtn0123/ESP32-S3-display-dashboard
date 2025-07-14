# Hardware Setup Guide

## ESP32-S3 T-Display Pin Connections

This guide documents the hardware connections for the ESP32-S3 T-Display board used in this project.

## Display Connections (Built-in)

The display is built into the T-Display board and uses the following pins:

| Function | GPIO Pin | Description |
|----------|----------|-------------|
| D0       | GPIO39   | Data bit 0 |
| D1       | GPIO40   | Data bit 1 |
| D2       | GPIO41   | Data bit 2 |
| D3       | GPIO42   | Data bit 3 |
| D4       | GPIO45   | Data bit 4 |
| D5       | GPIO46   | Data bit 5 |
| D6       | GPIO47   | Data bit 6 |
| D7       | GPIO48   | Data bit 7 |
| WR       | GPIO8    | Write strobe |
| DC       | GPIO7    | Data/Command |
| CS       | GPIO6    | Chip Select |
| RST      | GPIO5    | Reset |
| BL       | GPIO38   | Backlight |

## Button Connections (Built-in)

| Button   | GPIO Pin | Pull-up | Active |
|----------|----------|---------|--------|
| Button 1 | GPIO0    | Internal| Low    |
| Button 2 | GPIO14   | Internal| Low    |

## Battery Monitoring

| Function | GPIO Pin | Description |
|----------|----------|-------------|
| VBAT     | GPIO4    | Battery voltage (through divider) |

**Note**: The battery voltage is measured through a voltage divider (ratio 2:1), so the actual battery voltage is 2x the ADC reading.

## Optional External Connections

### I2C (for external sensors)
| Function | GPIO Pin |
|----------|----------|
| SDA      | GPIO43   |
| SCL      | GPIO44   |

### UART (for debugging)
| Function | GPIO Pin |
|----------|----------|
| TX       | GPIO43   |
| RX       | GPIO44   |

## Power Supply

- **USB-C**: 5V power and programming
- **Battery**: 3.7V LiPo battery connector (JST 1.25mm)
- **Operating Voltage**: 3.3V (internal regulator)

## Development Setup

### 1. Install ESP32 Rust Toolchain

```bash
# Install espup
cargo install espup
espup install

# Source the environment
source ~/export-esp.sh
```

### 2. Install Flash Tool

```bash
cargo install espflash
```

### 3. Build and Flash

```bash
# Build the project
cargo build --release

# Flash to device
espflash flash target/xtensa-esp32s3-none-elf/release/esp32-s3-dashboard

# Or use cargo run (configured in .cargo/config.toml)
cargo run --release
```

### 4. Monitor Serial Output

```bash
# Flash and monitor in one command
cargo run --release

# Or just monitor
espflash monitor
```

## Troubleshooting

### Device Not Found
- Ensure USB-C cable supports data (not charge-only)
- Install CP210x or CH34x drivers if needed
- Check device appears as `/dev/ttyUSB0` (Linux) or `/dev/cu.usbserial-*` (macOS)

### Flash Fails
- Hold BOOT button while connecting USB
- Try slower baud rate: `espflash flash --speed 460800`
- Ensure sufficient power (use USB directly, not through hub)

### Display Issues
- Verify all GPIO pins in config.toml match your board
- Check display initialization sequence in st7789.rs
- Ensure backlight pin is set high

### Button Not Working
- Buttons use internal pull-up, active low
- Debounce time can be adjusted in config.toml
- Check GPIO0 isn't held low (would keep in boot mode)

## Board Specifications

- **MCU**: ESP32-S3 (dual-core Xtensa LX7)
- **RAM**: 512KB SRAM
- **Flash**: 16MB
- **Display**: 1.9" 170x320 ST7789V
- **Connectivity**: WiFi 802.11 b/g/n, Bluetooth 5.0
- **USB**: USB-C with native USB support