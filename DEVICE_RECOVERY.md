# Device Recovery Guide

## Current Issue
The ESP32-S3 display is black and unresponsive after flashing. This is likely due to one of:
1. Display buffer allocation failure (most likely)
2. Panic/crash during initialization
3. Power management putting device to sleep

## Recovery Steps

### 1. Manual Boot Mode Entry
Since the T-Display-S3 supports automatic download mode, but it's not responding:

1. **Hold BOOT button** (GPIO0)
2. **Press and release RESET button** while holding BOOT
3. **Release BOOT button** after 1 second
4. The device should now be in download mode

### 2. Flash Minimal Firmware
Once in download mode, flash with these commands:

```bash
# First, try to erase flash completely
esptool.py --chip esp32s3 --port /dev/cu.usbmodem101 erase_flash

# If that works, flash the current firmware
./scripts/flash.sh

# If the script fails, use direct esptool command:
esptool.py --chip esp32s3 --port /dev/cu.usbmodem101 --baud 460800 \
  --before default_reset --after hard_reset write_flash \
  --flash_mode dio --flash_freq 40m --flash_size 16MB \
  0x0 target/xtensa-esp32s3-espidf/release/bootloader.bin \
  0x8000 target/xtensa-esp32s3-espidf/release/partition-table.bin \
  0x10000 target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard.bin
```

### 3. If Device Still Won't Respond

Try these troubleshooting steps:

1. **Check USB cable** - Use a data cable, not charge-only
2. **Try different USB port** - Preferably USB 2.0
3. **Check port availability**:
   ```bash
   ls /dev/cu.usb*
   ls /dev/tty.usb*
   ```
4. **Reset USB subsystem** (macOS):
   ```bash
   sudo killall -STOP -c usbd
   sudo killall -CONT -c usbd
   ```

### 4. Root Cause Analysis

Based on our testing, the issue is likely:
- **Binary size**: Current build is 1.43-1.5MB
- **Display allocation**: Needs ~109KB of SRAM
- **Recent changes**: Stability module added heap monitoring

The device worked fine at commit `42e3d05` but issues started after `c10d434`.

### 5. Build a Safe Version

To get back to a working state:

```bash
# Revert to known good commit
git checkout 42e3d05

# Clean build
./compile.sh --clean

# Flash with full erase
./scripts/flash.sh
```

### 6. Fix for Current Version

The current version likely needs:
1. Reduce binary size (remove unused features)
2. Optimize SRAM usage
3. Add early serial output to debug boot issues

## Prevention

For future development:
1. Always increment version number when testing
2. Use `./scripts/pre-flash-check.py` before flashing
3. Monitor binary size trends
4. Test on device after each significant change

## Emergency Recovery

If nothing else works:
1. Use Arduino IDE with a simple blink sketch
2. Flash MicroPython to verify hardware works
3. Use Espressif's flash download tool (Windows)

The hardware is robust - this is a software issue that can be recovered.