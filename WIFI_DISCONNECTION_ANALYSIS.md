# WiFi Disconnection Analysis

## Issue Summary
The ESP32-S3 device experiences WiFi disconnections with error code `0x6374c0` shortly after connecting. This appears to be related to power save mode or timing issues when starting network services.

## Error Pattern
```
wifi:state: run -> init (0x6374c0)
wifi:pm stop, total sleep time: X us / Y us
```

## Root Causes Identified

1. **Power Save Mode Timing**: The WiFi power save mode disable call may not be taking effect immediately
2. **Service Start Timing**: Starting network services (telnet, web server) too quickly after WiFi connection
3. **Socket Exhaustion**: The original socket exhaustion issue compounds the WiFi stability problem

## Fixes Applied

### 1. WiFi Power Save Disable Timing
- Added power save disable immediately after WiFi start
- Added delay after power save disable to ensure it takes effect
- Added stabilization delay after connection

### 2. Service Start Delay
- Added 3-second delay after IP assignment before starting services
- This allows WiFi stack to fully stabilize

### 3. Socket Exhaustion Prevention
- Set `max_open_sockets: 7` (LWIP maximum)
- Added `Connection: close` headers to all HTTP responses
- Enabled LRU purging for automatic connection cleanup

## Current Status
- Device still freezing under stress test
- May need manual reset to recover
- Further investigation needed to determine if it's WiFi or web server issue

## Next Steps
1. Test with single connections to isolate the issue
2. Monitor WiFi state during normal operation
3. Consider increasing delays or adjusting WiFi configuration
4. Investigate if the issue is specific to concurrent connections