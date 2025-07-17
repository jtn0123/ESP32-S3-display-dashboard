# ESP32-S3 Dashboard Flashing Guide

## Quick Start

Just run:
```bash
./flash.sh
```

That's it! The script handles everything automatically.

## What the Script Does

1. **Builds** your project in release mode (optimized for size)
2. **Flashes** using espflash v3.3.0 with `--flash-size 16mb` flag
3. **Monitors** serial output automatically (use `--no-monitor` to skip)

## Why This Works

The ESP32-S3 T-Display has 16MB of flash, but the default ESP-IDF partition table only allocates 1MB for apps. By using `espflash --flash-size 16mb`, we tell the tool to use the full flash capacity.

## Important Notes

- **Always use espflash v3.3.0** - Version 4.x has breaking changes
- **No custom partition tables needed** - The `--flash-size 16mb` flag handles it
- **Binary size limit**: ~15MB (plenty of room)

## Troubleshooting

### Port Issues
```bash
# List available ports
ls /dev/tty.usb* /dev/cu.usb*

# Specify port manually
./flash.sh --port /dev/cu.usbmodem1101
```

### Binary Too Large
If your binary exceeds 1MB even with optimization:
1. The script automatically uses espflash with 16MB flash size
2. No manual intervention needed

### Port Busy Error
```bash
# Find process using the port
lsof | grep usbmodem

# Kill the process
kill <PID>
```

## Build Options

```bash
./flash.sh --release    # Release build (default, optimized for size)
./flash.sh --debug      # Debug build (larger binary)
./flash.sh --clean      # Clean build
./flash.sh --no-monitor # Skip serial monitor after flashing
```

## Technical Details

- **Optimization**: Uses `opt-level = "z"` for smallest binary size
- **Flash method**: Direct espflash with explicit flash size
- **No partition tables**: Avoids ESP-IDF partition table complexity