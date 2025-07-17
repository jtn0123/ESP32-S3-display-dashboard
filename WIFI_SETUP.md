# WiFi Configuration Guide

## Quick Setup

The ESP32-S3 Dashboard needs your WiFi credentials to connect to your network.

### Method 1: Edit config.toml (Recommended for Development)

1. Open `config.toml` in your editor
2. Find the `[wifi]` section around line 67
3. Replace the placeholders:
   ```toml
   [wifi]
   ssid = "YOUR_ACTUAL_WIFI_NAME"
   password = "YOUR_ACTUAL_PASSWORD"
   ```

### Method 2: Use Environment Variables (Recommended for Security)

Set these before building:
```bash
export WIFI_SSID="Your WiFi Name"
export WIFI_PASSWORD="Your Password"
```

### Method 3: Runtime Configuration

The dashboard will start in AP mode if it can't connect, allowing you to configure WiFi via web interface at `http://192.168.4.1`

## Troubleshooting

- **"Network not found" error**: Your WiFi network name (SSID) wasn't found during scan
- **Case sensitive**: WiFi SSIDs are case-sensitive - check exact spelling
- **5GHz networks**: ESP32-S3 only supports 2.4GHz networks
- **Hidden networks**: Not supported in current version

## Security Note

Never commit your actual WiFi credentials to version control. Use `.gitignore` for any files containing credentials.