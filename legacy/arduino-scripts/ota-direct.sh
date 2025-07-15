#!/bin/bash

# Direct OTA upload using espota.py
# This bypasses arduino-cli and uses the ESP32 OTA tool directly

set -e

echo "🚀 ESP32 Direct OTA Upload"

# Configuration
ESPOTA="/Users/justin/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py"
IP="${1:-10.27.27.201}"  # Use discovered IP or provided one
PORT="3232"  # Default OTA port

# Check if we have python3
if ! command -v python3 &> /dev/null; then
    echo "❌ Python3 is required for OTA uploads"
    exit 1
fi

# Compile first
echo "📦 Compiling..."
cd dashboard
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 --export-binaries . || exit 1

# Find the compiled binary
BIN_FILE="build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin"
if [ ! -f "$BIN_FILE" ]; then
    echo "❌ Binary file not found at $BIN_FILE"
    echo "Looking for binary..."
    find . -name "*.bin" -type f
    exit 1
fi

echo "📡 Uploading to $IP:$PORT..."
echo "🖥️  Watch your device screen for progress!"

# Use espota.py directly
python3 "$ESPOTA" -i "$IP" -p "$PORT" -f "$BIN_FILE" -d -r

echo "✅ OTA upload complete!"