# ESP32-S3 Dashboard OTA (Over-The-Air) Update Implementation

## Overview

This guide documents the OTA update functionality implemented in the ESP32-S3 Dashboard project. The implementation allows you to update your device firmware wirelessly without needing a physical USB connection.

## Features

- **Web-based OTA interface**: Upload firmware via a simple web page
- **Progress tracking**: Real-time update progress displayed on device screen
- **Dual partition support**: Safe updates with rollback capability
- **Status monitoring**: Visual feedback during update process

## How OTA Works

1. The ESP32-S3 has two app partitions configured (app0 and app1)
2. The currently running firmware is on one partition
3. New firmware is written to the inactive partition
4. After successful verification, the bootloader switches to the new partition
5. If update fails, the device continues using the old firmware

## Usage Instructions

### Prerequisites

1. Device must be connected to WiFi
2. Know your device's IP address (shown on Network Status screen)

### Performing an OTA Update

1. **Build the new firmware**:
   ```bash
   ./compile.sh --release
   ```
   
2. **Choose your update method**:

   **Option A: Command Line (Recommended)**
   ```bash
   # Update a specific device
   python ota.py 192.168.1.100
   
   # Auto-discover and update all devices
   python ota.py --auto
   
   # Just find devices without updating
   python ota.py --scan
   ```
   
   **Option B: Web Browser**
   - Open a web browser
   - Navigate to: `http://<device-ip>:8080/ota`
   - Click "Choose File" and select the binary
   - Click "Upload Firmware"
   
3. **Device will automatically restart** after successful update

### Monitoring OTA Status on Device

The Network Status screen (screen 2) displays:
- Current OTA status (Ready, Downloading, Verifying, etc.)
- Download progress bar when updating
- Web URLs for configuration and OTA access

## Auto-Discovery Features

The ESP32 now supports automatic network discovery:

1. **mDNS Broadcasting**: Device advertises itself as `esp32-dashboard.local`
2. **Service Discovery**: Announces OTA service on port 8080
3. **Network Scanning**: Fallback method scans subnet for devices

### Using Auto-Discovery

```bash
# Find all ESP32 devices on your network
python ota.py --scan

# Update all discovered devices
python ota.py --auto

# Update in parallel (faster for multiple devices)
python ota.py --auto --parallel
```

The script will:
- First try mDNS discovery (if zeroconf is installed)
- Fall back to network scanning
- Show all found devices with their versions
- Ask for confirmation before updating

## Architecture Details

### Components

1. **OTA Manager** (`src/ota/manager.rs`):
   - Handles ESP-IDF OTA API calls
   - Manages update state and progress
   - Validates firmware integrity

2. **Web Server** (`src/ota/web_server.rs`):
   - Serves HTML interface on port 8080
   - Handles firmware upload via HTTP POST
   - Provides real-time progress updates

3. **UI Integration** (`src/ui/mod.rs`):
   - Displays OTA status on Network screen
   - Shows progress bar during updates
   - Color-coded status indicators

### Partition Table

The device uses a custom partition table (`partition_table/partitions.csv`):
```
nvs,      data, nvs,     0x9000,  0x6000,
otadata,  data, ota,     0xf000,  0x2000,
app0,     app,  ota_0,   0x20000, 0x2D0000,  # ~2.8MB
app1,     app,  ota_1,   ,        0x2D0000,  # ~2.8MB
spiffs,   data, spiffs,  ,        0xF0000,   # ~960KB
```

### Safety Features

1. **Rollback Protection**: If new firmware crashes, bootloader can revert
2. **Size Validation**: Firmware size must be reasonable (< 4MB)
3. **Integrity Check**: ESP-IDF validates firmware before marking as bootable
4. **Progress Monitoring**: Real-time feedback prevents accidental interruption

## Troubleshooting

### Common Issues

1. **"No Update Partition" Error**:
   - Ensure partition table is correctly configured
   - Rebuild with clean flash: `./flash.sh --clean`

2. **Upload Fails**:
   - Check WiFi connection is stable
   - Verify correct IP address and port (8080)
   - Ensure firmware file is the bare binary (not .elf)

3. **Device Doesn't Restart**:
   - Wait 2-3 seconds after "Update successful" message
   - If stuck, manually reset the device

4. **OTA Server Not Accessible**:
   - Verify device is connected to WiFi
   - Check firewall isn't blocking port 8080
   - Ensure OTA server started successfully (check logs)

### Debug Commands

View current partition info:
```bash
espflash partition-table /dev/cu.usbmodem101
```

Monitor device logs during OTA:
```bash
espflash monitor /dev/cu.usbmodem101
```

## Security Considerations

⚠️ **WARNING**: Current implementation lacks authentication!

For production use, consider:
1. Adding password protection to OTA interface
2. Implementing firmware signing/encryption
3. Using HTTPS instead of HTTP
4. Restricting OTA access to specific IP ranges

## Future Enhancements

- [ ] Automatic update checks from GitHub releases
- [ ] Firmware signature verification
- [ ] Update scheduling (e.g., nightly updates)
- [ ] Backup/restore configuration during updates
- [ ] Progress indication via LED or display brightness

## Version Information

When updating firmware, the version displayed on screen helps verify the update succeeded:
- Current version format: `v4.11`
- Always increment version when making changes
- Version shown on boot screen and system info

## Development Tips

1. **Testing OTA locally**:
   - Use two devices: one for current, one for new firmware
   - Test rollback by intentionally uploading bad firmware
   - Monitor serial output during update process

2. **Reducing firmware size**:
   - Use `--release` build for smaller binaries
   - Remove debug symbols and unused features
   - Check partition utilization before updates

3. **Version management**:
   - Update version string in `main.rs` before building
   - Tag git commits that correspond to OTA releases
   - Keep changelog of what changed between versions