# ESP32-S3 Dashboard OTA Update Guide

## Overview

The ESP32-S3 Dashboard supports Over-The-Air (OTA) firmware updates via WiFi. This allows you to update the device without physical USB connection.

## Prerequisites

- Device connected to WiFi network
- Computer on same network as ESP32
- Python 3 installed (for upload script)
- Compiled firmware binary (.bin file)

## OTA Methods

### 1. Web Interface (Recommended)

The dashboard includes a built-in web server for easy updates:

1. Find device IP (shown on display or serial monitor)
2. Open browser: `http://<device-ip>/`
3. Click "Firmware Update"
4. Select your .bin file
5. Click "Upload"
6. Wait for update to complete (device will reboot)

### 2. Command Line

Using curl:
```bash
curl -X POST http://<device-ip>/ota \
  -F "firmware=@target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
```

Using the Python script:
```bash
python3 ota-upload.py <device-ip> <firmware.bin>
```

### 3. Direct Network Update

For production deployments:
```bash
# Set OTA server URL in device config
curl -X POST http://<device-ip>/api/config \
  -H "Content-Type: application/json" \
  -d '{"ota_url": "http://your-server.com/firmware/latest.bin"}'
```

## Building OTA Binary

### For Rust Version
```bash
cargo build --release
# Binary at: target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

### For Arduino Version
```bash
cd dashboard
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 .
# Binary at: build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin
```

## Security Considerations

1. **Network Security**: Use secure WiFi (WPA2/WPA3)
2. **HTTPS**: For production, use HTTPS for OTA server
3. **Authentication**: Add password protection to web interface
4. **Verification**: Implement firmware signature verification

## Troubleshooting

### Update Fails to Start
- Verify device is on network (ping IP address)
- Check firewall settings
- Ensure binary is compiled for correct board

### Update Starts but Fails
- Check available flash space
- Verify binary integrity
- Try reducing WiFi traffic during update
- Check power supply stability

### Device Bootloops After Update
- Hold BOOT button during power on
- Flash via USB with known good firmware
- Check partition table configuration

## Partition Layout

The device uses the following partition scheme for OTA:

```
# Name,   Type, SubType, Offset,  Size
nvs,      data, nvs,     0x9000,  0x5000
otadata,  data, ota,     0xe000,  0x2000
app0,     app,  ota_0,   0x10000, 0x1E0000
app1,     app,  ota_1,   0x1F0000,0x1E0000
spiffs,   data, spiffs,  0x3D0000,0x30000
```

This allows for two app partitions, enabling safe rollback if update fails.

## Best Practices

1. **Test Updates**: Always test OTA on development device first
2. **Version Tracking**: Include version info in firmware
3. **Incremental Updates**: Update one device before fleet
4. **Backup Plan**: Keep USB cable handy for recovery
5. **Monitor Battery**: Don't OTA update on low battery

## Advanced Features

### Auto-Update Check
Configure automatic update checking:
```json
{
  "auto_update": true,
  "update_interval": 3600,
  "ota_url": "http://server.com/firmware/latest"
}
```

### Update Notification
The device will show update availability on display before installing.

### Rollback Protection
If new firmware fails to boot 3 times, device automatically rolls back to previous version.