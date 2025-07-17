# How OTA Updates Work on ESP32-S3 Dashboard

## Quick Answer

**You need to manually go to a URL** - the OTA system does NOT automatically find and update your device. Here's how it works:

1. **Find your device's IP**: Check the Network Status screen on the device
2. **Open web browser**: Go to `http://<device-ip>:8080/ota`  
3. **Upload firmware**: Select the compiled binary and click upload
4. **Device restarts**: Automatically reboots with new firmware

## Detailed Explanation

### Current Implementation (Manual OTA)

The current OTA implementation is **manual/pull-based**, meaning:
- ✅ You must manually visit the OTA web page
- ✅ You select and upload the firmware file yourself
- ✅ The device only updates when you explicitly tell it to
- ❌ No automatic discovery or updates
- ❌ No push notifications for available updates

### WiFi Setup

Your WiFi credentials are now configured at compile time:

1. **First Time Setup**:
   ```bash
   # Copy the example file
   cp wifi_config.h.example wifi_config.h
   
   # Edit with your credentials
   # (Already done - your SSID: Batcave, Password: greeneville!!!)
   ```

2. **Security**: 
   - `wifi_config.h` is in `.gitignore` - won't be uploaded to GitHub
   - Credentials are compiled into the firmware
   - Can still be changed via web interface after flashing

### OTA Process Flow

```
1. Device boots and connects to WiFi (using compiled credentials)
     ↓
2. OTA web server starts on port 8080
     ↓
3. Device shows its IP on Network Status screen
     ↓
4. You navigate to http://<ip>:8080/ota in browser
     ↓
5. You select firmware file and upload
     ↓
6. Device receives firmware, validates, and installs
     ↓
7. Device automatically restarts with new firmware
```

### Using the OTA Script

The unified `ota.py` script provides all OTA functionality:

```bash
# Build new firmware
./compile.sh --release

# Update a specific device
python ota.py 192.168.1.100

# Auto-discover and update ALL devices on network
python ota.py --auto

# Just scan to find devices (no update)
python ota.py --scan

# Update all devices in parallel (faster)
python ota.py --auto --parallel

# See what would be updated without doing it
python ota.py --auto --dry-run
```

### Future Enhancement Ideas

If you want automatic updates in the future, here are some approaches:

1. **mDNS Discovery** (Find devices automatically):
   - Device broadcasts its presence as "esp32-dashboard.local"
   - Update script can find all devices on network
   - Still requires manual trigger to update

2. **Pull-based Auto Updates**:
   - Device periodically checks GitHub releases
   - Downloads and installs updates automatically
   - Shows notification before updating

3. **Push-based Updates**:
   - Central server tracks all devices
   - Push updates to specific devices
   - Requires more infrastructure

### Current Network Features

When connected to WiFi, your device provides:
- **Configuration Page**: `http://<device-ip>/` (port 80)
- **OTA Update Page**: `http://<device-ip>:8080/ota`
- **Status Display**: Shows IP, connection status, and OTA state

### Security Notes

⚠️ **Current implementation has NO authentication!**
- Anyone on your network can update the device
- For home use this is usually fine
- For production, add password protection

### Troubleshooting WiFi Connection

If device won't connect to WiFi:
1. **Check credentials**: Ensure wifi_config.h has correct SSID/password
2. **Rebuild firmware**: `./compile.sh --clean`
3. **Check router**: 2.4GHz network, WPA2, DHCP enabled
4. **Monitor serial**: `./flash.sh --no-monitor` then `espflash monitor`

### Quick Reference

- **Device IP**: Check Network Status screen (press USER button to navigate)
- **OTA URL**: `http://<device-ip>:8080/ota`
- **Config URL**: `http://<device-ip>/`
- **Default Port**: 8080 for OTA, 80 for config
- **File to Upload**: `target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard`