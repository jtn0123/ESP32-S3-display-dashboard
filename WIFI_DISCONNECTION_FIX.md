# WiFi Disconnection Issue - Fixed

## Problem
The ESP32-S3 was disconnecting from WiFi approximately 4 seconds after successfully connecting and obtaining an IP address.

### Symptoms
- WiFi connects successfully during boot
- Gets IP address (10.27.27.201)
- Web server starts
- ~4 seconds later: `wifi:state: run -> init (0x6374c0)`
- Device becomes unreachable over network

### Root Cause
The WiFi power save mode (`WIFI_PS_MIN_MODEM`) was causing disconnections when there was network activity. This aggressive power saving mode can disconnect WiFi when:
- Web server is active
- Telnet server is running
- Multiple network services are enabled

### Solution
Disabled WiFi power save mode by changing from:
```rust
esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_MIN_MODEM);
```

To:
```rust
esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE);
```

### Trade-offs
- **Pro**: Stable WiFi connection, no disconnections
- **Con**: Slightly higher power consumption (~20-30mA more)

### Additional Improvements Needed
1. **Reconnection Logic**: Currently, if WiFi disconnects for any reason, there's no automatic reconnection
2. **Event Handlers**: Should implement WiFi event handlers to detect and handle disconnections
3. **Configurable Power Save**: Could make power save mode configurable based on use case

### Testing
The fix has been compiled and is ready to flash. With power save disabled, the WiFi connection should remain stable even under heavy network load.