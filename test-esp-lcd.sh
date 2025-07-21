#!/bin/bash
# ESP LCD DMA Test Script

echo "ESP LCD DMA Hardware Test"
echo "========================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Step 1: Build with LCD DMA feature
echo -e "${YELLOW}Step 1: Building with ESP LCD DMA feature...${NC}"
echo "Command: cargo build --release --no-default-features --features lcd-dma"
echo ""

cargo build --release --no-default-features --features lcd-dma

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed! Check errors above.${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"
echo ""

# Step 2: Flash
echo -e "${YELLOW}Step 2: Flashing to device...${NC}"
echo "Make sure your ESP32-S3 is connected via USB"
echo "Press Enter to continue..."
read

./scripts/flash.sh

if [ $? -ne 0 ]; then
    echo -e "${RED}Flash failed! Check connection and try again.${NC}"
    exit 1
fi

echo -e "${GREEN}Flash successful!${NC}"
echo ""

# Step 3: Monitor
echo -e "${YELLOW}Step 3: Monitoring serial output...${NC}"
echo "Look for these key indicators:"
echo "  1. 'I (xxx) lcd_panel: new I80 bus' - ESP LCD initialized"
echo "  2. Color cycle messages (Red, Green, Blue, White)"
echo "  3. FPS measurement (should be >25)"
echo ""
echo "Press Ctrl+] to exit monitor"
echo ""

espflash monitor