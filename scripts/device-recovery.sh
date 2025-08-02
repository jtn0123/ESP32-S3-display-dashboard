#!/bin/bash
# ESP32-S3 T-Display Device Recovery Script

echo "ESP32-S3 T-Display Recovery Tool"
echo "================================"
echo

PORT="${1:-/dev/cu.usbmodem101}"
echo "Using port: $PORT"

# Check if port exists
if [ ! -e "$PORT" ]; then
    echo "Error: Port $PORT not found"
    echo "Available ports:"
    ls /dev/cu.usb* /dev/tty.usb* 2>/dev/null || echo "No USB devices found"
    exit 1
fi

echo
echo "Recovery Steps:"
echo "1. Hold down the BOOT button (bottom button)"
echo "2. While holding BOOT, press and release RESET (top button)"
echo "3. Release the BOOT button"
echo "4. The device should now be in download mode"
echo
echo "Press Enter when ready..."
read

# Try to connect with various settings
echo "Attempting connection..."

# Method 1: Standard flash
echo "Method 1: Standard flash with erase..."
./scripts/flash.sh

if [ $? -ne 0 ]; then
    echo
    echo "Method 1 failed. Trying alternative method..."
    echo
    
    # Method 2: Direct esptool with different settings
    echo "Method 2: Direct esptool with slow speed..."
    ESPTOOL=".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
    
    # First just try to read chip ID
    $ESPTOOL --chip esp32s3 --port $PORT --baud 115200 --before usb_reset --after hard_reset chip_id
    
    if [ $? -eq 0 ]; then
        echo "Device responding! Attempting full flash..."
        
        # Full flash at lower speed
        $ESPTOOL --chip esp32s3 --port $PORT --baud 460800 \
            --before default_reset --after hard_reset write_flash \
            --flash_mode dio --flash_freq 40m --flash_size 16MB \
            0x0 target/xtensa-esp32s3-espidf/release/bootloader.bin \
            0x8000 target/xtensa-esp32s3-espidf/release/partition-table.bin \
            0x10000 target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard.bin
    else
        echo
        echo "Device not responding. Please check:"
        echo "1. USB cable is connected properly"
        echo "2. Device is powered on"
        echo "3. Try the BOOT/RESET button sequence again"
        echo
        echo "You can also try:"
        echo "- Different USB port"
        echo "- Different USB cable" 
        echo "- Power cycle the device completely"
    fi
fi

echo
echo "If the device is still not responding:"
echo "1. Unplug the USB cable"
echo "2. Wait 5 seconds"
echo "3. Hold BOOT button"
echo "4. Plug in USB while holding BOOT"
echo "5. Release BOOT after 2 seconds"
echo "6. Run this script again"