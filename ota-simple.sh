#!/bin/bash

# Simplest OTA upload method
echo "🚀 OTA Upload (Simple Method)"

cd dashboard

# Step 1: Compile with export
echo "📦 Compiling..."
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 --export-binaries .

# Step 2: Upload using IP directly
echo "📡 Uploading via OTA..."
echo "Note: If this fails, try using Arduino IDE with Tools > Port > 'esp32-dashboard at 10.27.27.201'"

# Try different methods
echo "Attempting upload to 10.27.27.201..."

# Method 1: Direct IP
~/Library/Arduino15/packages/esp32/tools/esptool_py/4.8.1/esptool.py \
    --chip esp32s3 \
    --port "socket://10.27.27.201:3232" \
    --baud 921600 \
    write_flash 0x10000 build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin \
    2>/dev/null || {
    
    # Method 2: Using espota.py without -r flag
    echo "Trying espota.py..."
    python3 ~/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
        -i 10.27.27.201 \
        -f build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin \
        -d || {
        
        echo "❌ Auto-upload failed."
        echo ""
        echo "📱 Manual OTA Instructions:"
        echo "1. Open Arduino IDE"
        echo "2. Tools → Port → Select 'esp32-dashboard at 10.27.27.201'"
        echo "3. Click Upload"
        echo ""
        echo "The binary is ready at:"
        echo "dashboard/build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin"
    }
}