#!/bin/bash

# Quick OTA upload - simplified version
# Usage: ./ota.sh [ip_address]

set -e

echo "🚀 ESP32 OTA Upload"

# Try to find device or use provided IP
IP="${1:-esp32-dashboard.local}"

# Compile and upload
cd dashboard
echo "📦 Compiling..."
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 . || exit 1

echo "📡 Uploading to $IP..."
arduino-cli upload -p "$IP" --fqbn esp32:esp32:lilygo_t_display_s3 --protocol network . || exit 1

echo "✅ OTA upload complete!"