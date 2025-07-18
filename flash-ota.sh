#!/bin/bash
# Proper OTA flash script that initializes everything correctly

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}ESP32-S3 OTA Flash Tool${NC}"
echo "========================"

# Configuration
PORT="${1:-/dev/cu.usbmodem101}"
PARTITION_TABLE="partitions_ota_safe.csv"
BINARY="target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Binary not found: $BINARY${NC}"
    echo "Run ./compile.sh first"
    exit 1
fi

# Find tools
ESPTOOL="$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
if [ ! -f "$ESPTOOL" ]; then
    ESPTOOL="$(which esptool.py)"
fi

GEN_PART="$HOME/.espressif/esp-idf/v5.3/components/partition_table/gen_esp32part.py"

echo -e "${BLUE}Configuration:${NC}"
echo "  Port: $PORT"
echo "  Partition: $PARTITION_TABLE"
echo "  Binary: $BINARY"

# Step 1: Generate partition binary
echo -e "\n${BLUE}Generating partition table binary...${NC}"
python3 "$GEN_PART" --verify "$PARTITION_TABLE" partition-table.bin

# Step 2: Create blank OTA data
echo -e "${BLUE}Creating initial OTA data...${NC}"
# OTA data is 0x2000 bytes, all 0xFF means no valid app selected yet
dd if=/dev/zero bs=1 count=8192 2>/dev/null | tr '\000' '\377' > ota_data_initial.bin

# Step 3: Find bootloader
BOOTLOADER=$(find target -name "bootloader.bin" -type f 2>/dev/null | head -1)
if [ -z "$BOOTLOADER" ]; then
    echo -e "${RED}Bootloader not found!${NC}"
    exit 1
fi

echo -e "\n${BLUE}Files to flash:${NC}"
echo "  Bootloader: $BOOTLOADER"
echo "  Partition: partition-table.bin"
echo "  OTA Data: ota_data_initial.bin"
echo "  App: $BINARY"

# Step 4: Erase and flash everything
echo -e "\n${YELLOW}Erasing entire flash...${NC}"
$ESPTOOL --chip esp32s3 --port "$PORT" erase_flash

echo -e "\n${GREEN}Flashing all components...${NC}"
$ESPTOOL \
    --chip esp32s3 \
    --port "$PORT" \
    --baud 921600 \
    --before default_reset \
    --after hard_reset \
    write_flash \
    --flash_mode dio \
    --flash_freq 40m \
    --flash_size 16MB \
    0x0 "$BOOTLOADER" \
    0x8000 partition-table.bin \
    0xe000 ota_data_initial.bin \
    0x10000 "$BINARY"

if [ $? -eq 0 ]; then
    echo -e "\n${GREEN}✓ Flash successful!${NC}"
    echo -e "${BLUE}OTA partitions initialized:${NC}"
    echo "  - app0 (ota_0): 1.5MB at 0x10000"
    echo "  - app1 (ota_1): 1.5MB at 0x190000"
    echo ""
    echo "The device will boot from ota_0."
    echo "First OTA update will go to ota_1."
    
    # Cleanup
    rm -f partition-table.bin ota_data_initial.bin
    
    echo -e "\n${BLUE}Starting monitor in 3 seconds...${NC}"
    sleep 3
    
    # Monitor
    ~/.espressif/python_env/idf5.3_py3.13_env/bin/python3 -c "
import serial
import sys

try:
    ser = serial.Serial('$PORT', 115200)
    print('Connected to $PORT - Press Ctrl+C to exit')
    print('=' * 50)
    
    while True:
        if ser.in_waiting:
            data = ser.read(ser.in_waiting)
            sys.stdout.write(data.decode('utf-8', errors='ignore'))
            sys.stdout.flush()
            
except KeyboardInterrupt:
    print('\nExiting...')
    ser.close()
except Exception as e:
    print(f'Error: {e}')
"
else
    echo -e "${RED}✗ Flash failed!${NC}"
    rm -f partition-table.bin ota_data_initial.bin
    exit 1
fi