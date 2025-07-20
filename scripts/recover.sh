#!/usr/bin/env bash
# ESP32-S3 Recovery Script - For stuck devices

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo -e "${RED}No device found${NC}"
    exit 1
fi

echo -e "${BLUE}ESP32-S3 Recovery Tool${NC}"
echo "======================"
echo -e "${BLUE}Port:${NC} $PORT"
echo ""

# Find esptool
if [ -f ".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
    ESPTOOL=".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
elif [ -f "$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
    ESPTOOL="$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
else
    ESPTOOL="esptool.py"
fi

echo -e "${YELLOW}Attempting device recovery...${NC}"
echo ""

# Method 1: Try with lower baud rate
echo "1. Trying connection at lower baud rate..."
if $ESPTOOL --chip esp32s3 --port "$PORT" --baud 115200 read_mac 2>/dev/null; then
    echo -e "${GREEN}✓ Connected at 115200 baud${NC}"
    echo "Now erasing flash..."
    $ESPTOOL --chip esp32s3 --port "$PORT" --baud 115200 erase_flash
    echo -e "${GREEN}✓ Recovery complete!${NC}"
    exit 0
fi

# Method 2: Try with no stub
echo "2. Trying connection without stub..."
if $ESPTOOL --chip esp32s3 --port "$PORT" --no-stub read_mac 2>/dev/null; then
    echo -e "${GREEN}✓ Connected without stub${NC}"
    echo "Now erasing flash..."
    $ESPTOOL --chip esp32s3 --port "$PORT" --no-stub erase_flash
    echo -e "${GREEN}✓ Recovery complete!${NC}"
    exit 0
fi

# Method 3: Try trace mode for debugging
echo "3. Checking with trace mode..."
$ESPTOOL --chip esp32s3 --port "$PORT" --trace read_mac 2>&1 | head -20

echo ""
echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${RED}Device appears to be unresponsive.${NC}"
echo ""
echo "Please try:"
echo "1. Unplug the USB cable"
echo "2. Wait 5 seconds"
echo "3. Plug it back in"
echo "4. Run: ./scripts/flash.sh"
echo ""
echo "If that doesn't work, the device may need:"
echo "- A different USB cable"
echo "- Manual BOOT/RESET button sequence"
echo "- Power cycling"
echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"