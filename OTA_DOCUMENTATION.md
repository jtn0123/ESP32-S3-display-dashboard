# ESP32-S3 OTA (Over-The-Air) Update Documentation

## Overview

This ESP32-S3 Dashboard project supports Over-The-Air (OTA) firmware updates, allowing you to update the device wirelessly without needing a USB connection after the initial setup.

## Architecture

### Partition Layout (16MB Flash)

The device uses a custom partition table optimized for OTA updates:

```
nvs,      data, nvs,     0x9000,   0x6000   (24KB)  - Non-volatile storage
otadata,  data, ota,     0xf000,   0x2000   (8KB)   - OTA state data
factory,  app,  factory, 0x20000,  0x180000 (1.5MB) - Factory app (fallback)
ota_0,    app,  ota_0,   0x1A0000, 0x180000 (1.5MB) - OTA slot 0
ota_1,    app,  ota_1,   0x320000, 0x180000 (1.5MB) - OTA slot 1
```

### Boot Process

1. **Bootloader** reads the `otadata` partition to determine which app to boot
2. **OTA Data** contains:
   - Sequence numbers for ota_0 and ota_1
   - CRC32 checksums
   - The partition with the highest valid sequence number boots
3. **Alternating Updates**: Each OTA update alternates between ota_0 and ota_1

### OTA Flow

1. Device receives firmware binary via HTTP POST to `/ota/update`
2. Firmware is written to the inactive OTA partition
3. OTA data is updated to point to the new partition
4. Device reboots and loads from the new partition
5. If boot fails, device can fall back to previous partition

## Initial Setup

### 1. First-Time Flash (USB Required)

The device must be flashed via USB at least once to enable OTA:

```bash
# Standard flash with OTA support
./flash.sh

# Or use the comprehensive OTA setup script
./flash-ota-proper.sh
```

This initializes:
- Custom partition table with OTA slots
- Bootloader configured for OTA
- OTA data pointing to ota_0
- Application in both factory and ota_0 partitions

### 2. Network Configuration

Ensure the device is connected to your WiFi network. The device will display its IP address on the screen once connected.

## Using OTA Updates

### Quick Update

Update a device at a known IP:

```bash
./ota.sh 192.168.1.100
```

### Find and Update

Find devices on your network:

```bash
# Quick find - returns first device found
./ota.sh find

# Full network scan
./ota.sh scan

# Auto-discover and update all devices
./ota.sh auto
```

### Build and Update

```bash
# Build release firmware
./compile.sh --release

# Update specific device
./ota.sh 192.168.1.100

# Or update all devices on network
./ota.sh auto
```

## OTA Script Features

The `ota.sh` script includes:

- **Automatic ELF to Binary Conversion**: Converts ELF executables to ESP32 binary format
- **mDNS Discovery**: Fast device discovery using mDNS (esp32-display.local)
- **Network Scanning**: Falls back to network scan if mDNS fails
- **Progress Feedback**: Shows upload progress and status
- **Error Handling**: Clear error messages for common issues
- **Batch Updates**: Update multiple devices at once

## Troubleshooting

### Common Issues

1. **"OTA update failed: Failed to find OTA partition"**
   - Device is running from factory partition
   - Flash once more via USB to enable OTA

2. **"Failed to write OTA data: WriteFailed"**
   - Firmware size exceeds partition size (1.5MB limit)
   - Reduce firmware size or optimize build

3. **"No ESP32 devices found"**
   - Ensure device is powered on and connected to WiFi
   - Check you're on the same network subnet
   - Try specifying IP directly

4. **Black Screen After OTA**
   - OTA succeeded but app may have issues
   - Use USB to reflash if needed

### Binary Size Optimization

If your firmware exceeds 1.5MB:

```toml
# In Cargo.toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true         # Link-time optimization
strip = true       # Strip symbols
```

### Manual OTA Testing

Test OTA endpoint manually:

```bash
# Check device status
curl http://192.168.1.100/api/system

# Upload firmware manually
curl -X POST --data-binary @firmware.bin http://192.168.1.100/ota/update
```

## Security Considerations

**WARNING**: The current OTA implementation has no authentication or encryption. For production use, implement:

- HTTPS/TLS for encrypted transfers
- Authentication tokens or certificates
- Firmware signature verification
- Rollback protection

## Development Notes

### Key Files

- `/src/ota/mod.rs` - OTA manager implementation
- `/src/ota/manager.rs` - Core OTA logic
- `/src/network/web_server.rs` - HTTP endpoints
- `/partitions/partitions_16mb_ota.csv` - Partition table
- `/ota.sh` - OTA upload script
- `/flash.sh` - USB flashing script
- `/flash-ota-proper.sh` - Comprehensive OTA setup

### How OTA Manager Works

1. Checks current running partition
2. If on factory, manually finds ota_0 partition
3. Accepts firmware binary data
4. Writes to next OTA partition
5. Updates OTA data to boot from new partition
6. Triggers system restart

### Adding OTA Support to Other Projects

1. Copy the partition table CSV
2. Update `sdkconfig.defaults`:
   ```
   CONFIG_PARTITION_TABLE_CUSTOM=y
   CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions/partitions_16mb_ota.csv"
   CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y
   ```
3. Implement OTA manager (see `/src/ota/`)
4. Add HTTP endpoints for OTA
5. Use the OTA scripts for updates

## Version History

- v0.1.0 - Initial OTA implementation with 1.5MB partitions
- Factory partition included for development/recovery
- Automatic binary conversion in scripts
- mDNS support for device discovery