#!/bin/bash

echo "ESP32-S3 Flash Verification Script"
echo "=================================="
echo

# Step 1: Check USB enumeration
echo "1. USB Device Check:"
ls /dev/tty.usbmodem* /dev/cu.usbmodem* 2>/dev/null || echo "   ❌ No USB modem device found"
echo

# Step 2: Verify bootloader hash
echo "2. Bootloader Version Check:"
BOOTLOADER_PATH="target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-f8090498544b0ecf/out/build/bootloader/bootloader.bin"
if [ -f "$BOOTLOADER_PATH" ]; then
    echo "   Bootloader file: $BOOTLOADER_PATH"
    .embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py image_info "$BOOTLOADER_PATH" 2>/dev/null | head -5 | grep -E "Image version:|Validation hash:" || echo "   Could not read bootloader info"
else
    echo "   ❌ Bootloader file not found"
fi
echo

# Step 3: Check merge flag in sdkconfig
echo "3. DROM Merge Flag Check:"
if [ -f "sdkconfig.defaults" ]; then
    grep "CONFIG_APP_RODATA_SEGMENT_MERGE" sdkconfig.defaults || echo "   ❌ Merge flag not found in sdkconfig.defaults"
else
    echo "   ❌ sdkconfig.defaults not found"
fi

# Also check in build output
BUILD_CONFIG="target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-f8090498544b0ecf/out/build/config/sdkconfig.h"
if [ -f "$BUILD_CONFIG" ]; then
    echo "   Build config check:"
    grep "CONFIG_APP_RODATA_SEGMENT_MERGE" "$BUILD_CONFIG" | head -1 || echo "   ❌ Merge flag not in build config"
fi
echo

# Step 4: Flash addresses used
echo "4. Flash Layout:"
echo "   Bootloader: 0x0     (18.9 KB)"
echo "   Partition:  0x8000  (3.0 KB)"
echo "   App:        0x10000 (204.8 KB)"
echo

echo "5. To Monitor Serial Output:"
echo "   Option 1: screen /dev/cu.usbmodem101 115200"
echo "   Option 2: ./scripts/monitor.sh"
echo "   Option 3: Arduino IDE Serial Monitor @ 115200"
echo
echo "After connecting, press RESET button on ESP32-S3"
echo "=================================="