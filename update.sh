#!/bin/bash

# ESP32 Web OTA Update Script
echo "🚀 ESP32 Web OTA Update"
echo "======================="

# Allow manual IP as argument
if [ -n "$1" ]; then
    IP="$1"
    echo "Using manual IP: $IP"
else
    # Try to find device
    echo "Finding device..."
    IP=$(ping -c 1 -W 1 esp32-dashboard.local 2>/dev/null | grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' | head -1)
    
    if [ -z "$IP" ]; then
        # Fallback to last known IP
        IP="10.27.27.201"
        echo "⚠️  Could not find device via mDNS, trying $IP"
    else
        echo "✅ Found device at $IP"
    fi
fi

# Compile
echo ""
echo "📦 Compiling firmware..."
cd dashboard || { echo "❌ dashboard directory not found"; exit 1; }

if arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 --export-binaries .; then
    echo "✅ Compilation successful"
else
    echo "❌ Compilation failed"
    exit 1
fi

# Check binary exists
if [ ! -f "build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin" ]; then
    echo "❌ Binary file not found"
    exit 1
fi

# Upload
echo ""
echo "📡 Uploading to $IP..."
echo "   Watch device screen for progress"

# Use curl with timeout and show response
if curl -f -s -S --max-time 30 \
    -F "update=@build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin" \
    "http://$IP/update" -o response.txt; then
    
    # Check response
    if grep -q "OK" response.txt 2>/dev/null; then
        echo "✅ Upload successful!"
        rm -f response.txt
    else
        echo "⚠️  Upload completed but device returned: $(cat response.txt)"
        rm -f response.txt
    fi
    
    echo ""
    echo "🎉 Update complete!"
    echo "   Device will restart in a few seconds"
else
    echo "❌ Upload failed - could not connect to $IP"
    echo "   Check that device shows Web OTA ready on WiFi screen"
    rm -f response.txt
    exit 1
fi