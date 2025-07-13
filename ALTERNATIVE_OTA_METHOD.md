# Alternative OTA Method for ESP32-S3

Due to ESP32-S3's native USB implementation, ArduinoOTA sometimes has issues with port binding. Here are alternative methods:

## Method 1: Arduino IDE (Most Reliable)
1. Open Arduino IDE
2. Tools → Port → Look for "esp32-dashboard at 10.27.27.201"
3. If it appears, select it and upload normally
4. If not, see Method 2

## Method 2: Web-Based OTA (Recommended for ESP32-S3)
We can implement HTTP-based OTA in a future update, which is more reliable for ESP32-S3:
- Upload firmware to a web server
- Device downloads and installs it
- No port issues

## Method 3: ESP Web Tools
Use https://web.esphome.io/ or similar web flashers that support network devices.

## Current Status
- OTA code is properly implemented
- Device advertises itself via mDNS
- Port 3232 binding issue on ESP32-S3

## Workaround
For now, continue using USB upload with the `./upload.sh` script. The OTA infrastructure is in place for when we switch to HTTP-based OTA.

## Next Steps
1. Implement HTTP Update Server method
2. Or use ESP-IDF's native OTA implementation
3. Or wait for ArduinoOTA library updates for better ESP32-S3 support