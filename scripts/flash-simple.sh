#!/usr/bin/env bash
# Simplified flash script that ensures proper reset and monitoring

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Parse args
MONITOR=false
NO_ERASE=""
for arg in "$@"; do
    case $arg in
        --monitor) MONITOR=true ;;
        --no-erase) NO_ERASE="--no-erase" ;;
    esac
done

PORT="${PORT:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo -e "${RED}No device found${NC}"
    exit 1
fi

echo -e "${GREEN}ESP32-S3 Flash + Monitor${NC}"
echo "========================"
echo -e "${BLUE}Port:${NC} $PORT"

# First, use espflash for flashing (it handles resets better)
echo -e "\n${BLUE}Flashing with espflash...${NC}"
if ! espflash flash --port "$PORT" --flash-size 16mb --flash-freq 40mhz --flash-mode dio $NO_ERASE target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard; then
    echo -e "${RED}Flash failed!${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Flash complete${NC}"

# Give device a moment to start booting
sleep 1

# Now monitor to see what's happening
if [ "$MONITOR" = true ]; then
    echo -e "\n${YELLOW}Starting monitor...${NC}"
    echo -e "${YELLOW}Press Ctrl+] to exit${NC}\n"
    
    # Use espflash monitor which handles USB-JTAG/CDC properly
    exec espflash monitor --port "$PORT"
else
    echo -e "\nTo monitor: espflash monitor --port $PORT"
fi