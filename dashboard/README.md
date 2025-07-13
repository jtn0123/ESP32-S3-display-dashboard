# Dashboard WiFi Setup

## WiFi Configuration

This dashboard requires WiFi credentials to connect to your network.

### Setup Instructions

1. **Copy the template file:**
   ```bash
   cp wifi_config.h.example wifi_config.h
   ```

2. **Edit wifi_config.h with your credentials:**
   ```cpp
   #define WIFI_SSID "YourNetworkName"
   #define WIFI_PASSWORD "YourPassword"
   ```

3. **Upload to ESP32:**
   ```bash
   arduino-cli upload -p /dev/cu.usbmodem101 --fqbn esp32:esp32:lilygo_t_display_s3 dashboard.ino
   ```

### Security Notes

- ⚠️ **NEVER commit wifi_config.h to git** - it contains sensitive credentials
- ✅ The .gitignore file automatically excludes wifi_config.h
- ✅ Use wifi_config.h.example as a template for others
- ✅ WiFi credentials are only stored locally on your machine

### Supported Networks

- ✅ 2.4GHz WiFi networks (ESP32 requirement)
- ❌ 5GHz networks are not supported by ESP32
- ✅ WPA/WPA2 security
- ✅ Open networks (not recommended)