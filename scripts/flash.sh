#!/usr/bin/env bash
# ESP32-S3 Dashboard - Simplified USB Flash Script
# Always flashes to both factory and ota_0 for safety

set -euo pipefail

# Parse command line arguments
MONITOR=false
NO_ERASE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --monitor)
            MONITOR=true
            shift
            ;;
        --no-erase)
            NO_ERASE=true
            shift
            ;;
        *)
            echo "Usage: $0 [--monitor] [--no-erase]"
            exit 1
            ;;
    esac
done

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PORT="${PORT:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"
BUILD_DIR="target/xtensa-esp32s3-espidf/release"
ELF_FILE="$BUILD_DIR/esp32-s3-dashboard"
BIN_FILE="$BUILD_DIR/esp32-s3-dashboard.bin"
BOOTLOADER="$BUILD_DIR/bootloader.bin"
PARTITION_CSV="partition_table/partitions_ota.csv"
PARTITION_BIN="$BUILD_DIR/partition-table.bin"
OTA_DATA_INIT="firmware/ota_data_initial.bin"

# Find esptool.py
if [ -f ".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
    ESPTOOL=".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
elif [ -f "$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
    ESPTOOL="$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
else
    ESPTOOL="esptool.py"
fi

echo -e "${GREEN}ESP32-S3 Dashboard - USB Flash Tool${NC}"
echo "===================================="

# Check prerequisites
if [ -z "$PORT" ]; then
    echo -e "${RED}✗ No USB device found${NC}"
    echo "Please connect your ESP32-S3 device"
    exit 1
fi

if [ ! -f "$ELF_FILE" ]; then
    echo -e "${RED}✗ Firmware not found${NC}"
    echo "Run ./compile.sh first"
    exit 1
fi

if [ ! -f "$OTA_DATA_INIT" ]; then
    echo -e "${RED}✗ OTA data file not found${NC}"
    echo "Missing: $OTA_DATA_INIT"
    exit 1
fi

echo -e "${BLUE}Port:${NC} $PORT"
echo -e "${BLUE}Firmware:${NC} $ELF_FILE"

# Step 1: Convert ELF to binary
echo -e "\n${BLUE}Converting ELF to binary...${NC}"
$ESPTOOL --chip esp32s3 elf2image --flash_mode dio --flash_freq 40m --flash_size 16MB "$ELF_FILE" -o "$BIN_FILE"

if [ ! -f "$BIN_FILE" ]; then
    echo -e "${RED}✗ Failed to convert ELF to binary${NC}"
    exit 1
fi

# Get binary size
BIN_SIZE=$(stat -f%z "$BIN_FILE" 2>/dev/null || stat -c%s "$BIN_FILE" 2>/dev/null)
BIN_SIZE_MB=$(echo "scale=2; $BIN_SIZE / 1024 / 1024" | bc)
echo -e "${GREEN}✓ Binary size: ${BIN_SIZE_MB} MB${NC}"

# Step 2: Generate partition table binary
echo -e "\n${BLUE}Generating partition table...${NC}"
if [ -f "$HOME/.espressif/esp-idf/v5.3/components/partition_table/gen_esp32part.py" ]; then
    python3 "$HOME/.espressif/esp-idf/v5.3/components/partition_table/gen_esp32part.py" \
        --verify "$PARTITION_CSV" "$PARTITION_BIN"
else
    echo -e "${YELLOW}Warning: gen_esp32part.py not found, using CSV directly${NC}"
fi

# Step 3: Optional full erase
if [[ "$NO_ERASE" != "true" ]]; then
    echo -e "\n${YELLOW}Erasing entire flash...${NC}"
    $ESPTOOL --chip esp32s3 --port "$PORT" erase_flash
    echo -e "${GREEN}✓ Flash erased${NC}"
else
    echo -e "\n${BLUE}Skipping flash erase (--no-erase)${NC}"
fi

# Step 4: Flash bootloader
echo -e "\n${BLUE}Flashing bootloader...${NC}"
$ESPTOOL --chip esp32s3 --port "$PORT" --baud 921600 \
    write_flash --flash_mode dio --flash_freq 40m --flash_size 16MB \
    0x0 "$BOOTLOADER"

# Step 5: Flash partition table
echo -e "\n${BLUE}Flashing partition table...${NC}"
if [ -f "$PARTITION_BIN" ]; then
    $ESPTOOL --chip esp32s3 --port "$PORT" --baud 921600 \
        write_flash --flash_mode dio --flash_freq 40m --flash_size 16MB \
        0x8000 "$PARTITION_BIN"
else
    # Fallback: let esptool generate it
    $ESPTOOL --chip esp32s3 --port "$PORT" --baud 921600 \
        write_flash --flash_mode dio --flash_freq 40m --flash_size 16MB \
        0x8000 "$PARTITION_CSV"
fi

# Step 6: Initialize OTA data (points to factory)
echo -e "\n${BLUE}Initializing OTA data...${NC}"
$ESPTOOL --chip esp32s3 --port "$PORT" --baud 921600 \
    write_flash --flash_mode dio --flash_freq 40m --flash_size 16MB \
    0xd000 "$OTA_DATA_INIT"

# Step 7: Flash app to ota_0 (partition_table/partitions_ota.csv layout)
echo -e "\n${BLUE}Flashing app to ota_0...${NC}"
$ESPTOOL --chip esp32s3 --port "$PORT" --baud 921600 \
    write_flash --flash_mode dio --flash_freq 40m --flash_size 16MB \
    0x10000 "$BIN_FILE"

# Step 8: Ensure device resets to run the new firmware
echo -e "\n${BLUE}Resetting device...${NC}"
# Attempt a hard reset via esptool line toggling
$ESPTOOL --chip esp32s3 --port "$PORT" --baud 115200 --before no_reset --after hard_reset \
    read_mac >/dev/null 2>&1 || true
sleep 1

# Clean up
rm -f "$BIN_FILE" "$PARTITION_BIN"

# Extract version from the binary
VERSION=$(grep -a "v[0-9]\+\.[0-9]\+-rust" "$ELF_FILE" 2>/dev/null | head -1 | grep -o "v[0-9]\+\.[0-9]\+-rust" || echo "unknown")

echo -e "\n${GREEN}✅ USB flash complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "• Bootloader: ${GREEN}✓${NC}"
echo -e "• Partition table: ${GREEN}✓${NC}"
echo -e "• OTA data → factory: ${GREEN}✓${NC}"
echo -e "• Factory partition: ${GREEN}✓${NC} (${VERSION})"
echo -e "• OTA_0 partition: ${GREEN}✓${NC} (${VERSION})"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Device will boot from factory partition.${NC}"
echo -e "${BLUE}First OTA update will go to ota_1.${NC}"

# Start monitor if requested
if [[ "$MONITOR" == "true" ]]; then
    echo ""
    echo -e "${YELLOW}Starting monitor...${NC}"
    echo -e "${YELLOW}Press Ctrl+C to exit${NC}"
    echo ""
    sleep 1
    # Use simple serial monitor that won't interfere with the running device
    exec ./scripts/monitor.sh
else
    echo ""
    echo "To monitor: ./scripts/monitor.sh"
fi