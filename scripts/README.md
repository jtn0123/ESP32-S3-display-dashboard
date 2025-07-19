# ESP32-S3 Dashboard Scripts

This folder contains the essential scripts for flashing and updating your ESP32-S3 device.

## Scripts Overview

### flash.sh - USB Flashing
The main script for flashing firmware via USB. This script ensures your device always boots by:
- Flashing to BOTH factory and ota_0 partitions
- Initializing OTA data to point to factory
- Converting ELF files to binary automatically

**Usage:**
```bash
# Full flash (erases everything first)
./scripts/flash.sh

# Fast flash (skip erase for development)
./scripts/flash.sh --no-erase
```

### ota.sh - Wireless Updates
Performs Over-The-Air updates via WiFi. Features:
- Automatic device discovery
- Progress tracking
- Version verification
- Binary conversion

**Usage:**
```bash
# Find devices on network
./scripts/ota.sh find

# Update specific device
./scripts/ota.sh 192.168.1.100

# Update all devices
./scripts/ota.sh auto
```

### check-partition.sh - Diagnostics
Checks partition status and OTA availability.

**Usage:**
```bash
# Check specific device
./scripts/check-partition.sh 192.168.1.100

# Auto-discover and check
./scripts/check-partition.sh
```

## Partition Layout

```
nvs,      0x9000,   24KB   - Non-volatile storage
otadata,  0xf000,   8KB    - OTA state (which partition boots)
factory,  0x20000,  1.5MB  - Factory app (default boot)
ota_0,    0x1A0000, 1.5MB  - First OTA slot
ota_1,    0x320000, 1.5MB  - Second OTA slot
```

## Workflow

1. **Initial Setup**: `./scripts/flash.sh`
   - Erases flash, sets up partitions
   - Installs firmware to both factory and ota_0
   - Device boots from factory

2. **Development**: `./scripts/flash.sh --no-erase`
   - Quick updates during development
   - Preserves WiFi credentials

3. **Production Updates**: `./scripts/ota.sh <ip>`
   - Wireless updates
   - Automatic slot management
   - No downtime

## Troubleshooting

**Device won't boot (black screen)**
- Run `./scripts/flash.sh` without arguments
- This does a full erase and reflash

**OTA fails with "BeginFailed"**
- Version hasn't changed in Cargo.toml
- Device storage is corrupted - use USB flash

**Can't find device on network**
- Check WiFi credentials after flash erase
- Use `./scripts/check-partition.sh` for diagnostics

## Technical Details

The OTA system works by:
1. Device boots from partition specified in otadata
2. OTA updates write to inactive partition
3. After successful update, otadata is updated
4. Device reboots from new partition
5. If boot fails, watchdog resets to previous partition