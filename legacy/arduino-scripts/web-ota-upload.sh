#!/bin/bash

# Web OTA Upload Script
# Simple script to upload firmware via web interface

echo "🌐 ESP32-S3 Web OTA Upload"
echo "========================="

# Configuration
IP="${1:-10.27.27.201}"
SKETCH_DIR="dashboard"

# Step 1: Compile and export binary
echo "📦 Compiling firmware..."
cd "$SKETCH_DIR"
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 --export-binaries . || exit 1

# Find the binary
BIN_FILE="build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin"
if [ ! -f "$BIN_FILE" ]; then
    echo "❌ Binary not found!"
    exit 1
fi

echo "✅ Binary ready: $(ls -lh "$BIN_FILE" | awk '{print $5}')"

# Step 2: Upload via web
echo ""
echo "📡 Web OTA Instructions:"
echo "========================"
echo "1. Open your web browser"
echo "2. Go to: http://$IP"
echo "3. Click 'Choose File' and select:"
echo "   $(pwd)/$BIN_FILE"
echo "4. Click 'Update'"
echo ""
echo "The dashboard will show upload progress on screen!"
echo ""

# Optional: Try to open browser automatically
if command -v open &> /dev/null; then
    echo "Opening browser..."
    open "http://$IP"
elif command -v xdg-open &> /dev/null; then
    echo "Opening browser..."
    xdg-open "http://$IP"
fi

echo "📍 Binary location (for manual upload):"
echo "$(pwd)/$BIN_FILE"