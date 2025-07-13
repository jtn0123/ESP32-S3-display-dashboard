# Web OTA Guide - Super Simple!

## ✨ Web OTA is now enabled!

### How to update your dashboard wirelessly:

1. **Check WiFi Status screen** on your device
   - Look for "WEB OTA" section
   - Note the URL (e.g., `http://10.27.27.201`)

2. **Prepare update**
   ```bash
   ./web-ota-upload.sh
   ```
   This compiles your code and tells you where the binary is

3. **Upload via browser**
   - Open the URL from step 1
   - Choose the .bin file
   - Click "Update"
   - Watch progress on device screen!

### That's it! 🎉

## Features:
- ✅ Ultra lightweight (adds ~5KB)
- ✅ No complex setup
- ✅ Works with any browser
- ✅ Shows progress on display
- ✅ Auto-restarts after update

## Comparison:
- **ArduinoOTA**: 15KB, doesn't work on ESP32-S3
- **AsyncElegantOTA**: 50KB+, needs async libraries
- **Our Web OTA**: 5KB, just works!

## Security:
For production, add password:
```cpp
server.on("/", HTTP_GET, []() {
  if (!server.authenticate("admin", "password")) {
    return server.requestAuthentication();
  }
  // ... rest of code
});
```