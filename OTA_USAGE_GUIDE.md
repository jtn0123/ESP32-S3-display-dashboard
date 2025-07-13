# OTA (Over-The-Air) Update Guide

## 🚀 OTA is now enabled on your ESP32-S3 Dashboard!

### How to use OTA updates:

1. **First Upload (Cable Required)**
   - Upload the code once using USB cable with your normal method:
   ```bash
   ./upload.sh
   ```

2. **Find Your Device**
   - Check the WiFi Status screen on your dashboard
   - Look for the OTA hostname: `esp32-dashboard`
   - Note your device's IP address

3. **Future Updates (No Cable!)**
   - Make your code changes
   - In Arduino IDE: Tools → Port → Select "esp32-dashboard at [IP]"
   - Click Upload - it will update over WiFi!
   
   Or use Arduino CLI:
   ```bash
   arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 dashboard
   arduino-cli upload -p esp32-dashboard.local --fqbn esp32:esp32:lilygo_t_display_s3 dashboard
   ```

4. **During OTA Update**
   - Dashboard screen shows "OTA UPDATE IN PROGRESS..."
   - Progress bar displays update percentage
   - Device automatically restarts when complete

### Tips:
- Device must be connected to WiFi for OTA to work
- Keep device powered during updates
- Update takes about 30-60 seconds
- If update fails, you can always use USB cable

### Security (Optional):
To add password protection, uncomment this line in the code:
```cpp
// ArduinoOTA.setPassword("your-ota-password");
```

### Troubleshooting:
- Can't find device? Check WiFi Status screen for IP
- Update fails? Ensure good WiFi signal
- Still issues? Use USB cable as fallback

Now you can update your dashboard from anywhere on your network! 🎉