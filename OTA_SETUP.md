# OTA (Over-The-Air) Update Setup

## Current Status

The ESP32-S3 Dashboard now uses the ESP-IDF standard two OTA partition layout. OTA updates are fully enabled!

## Enabling OTA Support

To enable OTA updates, you need to:

1. **Use the OTA partition table**:
   ```bash
   # Copy the OTA configuration
   cp sdkconfig.defaults.ota sdkconfig.defaults
   
   # Clean build to ensure partition table is updated
   ./compile.sh --clean
   ```

2. **Flash with the new partition table**:
   ```bash
   # This will erase all data and use the new partition scheme
   ./flash.sh --erase-flash
   ```

3. **Connect to WiFi**:
   - The dashboard will need an active WiFi connection for OTA
   - Configure WiFi through the web interface at http://esp32-dashboard.local/

4. **Upload firmware**:
   - Navigate to http://esp32-dashboard.local:8080/ota
   - Select your firmware binary file
   - Click Upload

## Partition Layout

### Standard Two OTA Partition Layout (Current)
The ESP-IDF standard two OTA partition table provides:
- NVS: 16KB (for WiFi/BT and app data)
- OTA Data: 8KB (tracks active partition)
- PHY Init: 4KB (RF calibration data)
- OTA_0: ~7.9MB (first app partition)
- OTA_1: ~7.9MB (second app partition)
- Maximum app size per partition: ~7.9MB

Note: No core dump partition is included in the standard layout. Core dump errors at boot are expected and can be ignored.

## Building for OTA

When building for OTA, ensure your app size is under 2MB:
```bash
./compile.sh --release
ls -lh target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

## Rollback Protection

The OTA configuration includes rollback protection:
- If a new firmware fails to boot properly, the device will automatically rollback to the previous version
- The app must call `esp_ota_mark_app_valid_cancel_rollback()` to confirm successful boot

## Troubleshooting

1. **"No update partition available" error**:
   - You're using the factory partition table
   - Follow the steps above to enable OTA support

2. **App too large for OTA**:
   - Reduce features or optimize code size
   - Current size must be under 2MB for OTA partitions

3. **OTA upload fails**:
   - Ensure WiFi is connected
   - Check that OTA server is running (port 8080)
   - Verify firmware binary is valid