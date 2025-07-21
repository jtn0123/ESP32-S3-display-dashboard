#!/bin/bash
# Complete ESP LCD DMA Test Runner

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${BOLD}ESP LCD DMA Complete Test Runner${NC}"
echo "======================================"
echo "This will:"
echo "1. Build with ESP LCD DMA feature"
echo "2. Flash your device"
echo "3. Monitor and analyze results"
echo ""

# Check if device is connected
echo -e "${YELLOW}Checking for connected ESP32-S3...${NC}"
if ls /dev/tty.usb* 2>/dev/null | grep -q .; then
    echo -e "${GREEN}✓ Found USB device(s):${NC}"
    ls /dev/tty.usb* | sed 's/^/  /'
else
    echo -e "${RED}✗ No USB device found!${NC}"
    echo "Please connect your ESP32-S3 T-Display and try again."
    exit 1
fi

# Step 1: Build
echo ""
echo -e "${BLUE}Step 1: Building with ESP LCD DMA...${NC}"
echo "Running: cargo build --release --no-default-features --features lcd-dma"

# Capture build output
BUILD_LOG=$(mktemp)
cargo build --release --no-default-features --features lcd-dma 2>&1 | tee "$BUILD_LOG"

if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo -e "${RED}✗ Build failed!${NC}"
    echo "See build log above for errors."
    rm "$BUILD_LOG"
    exit 1
fi

echo -e "${GREEN}✓ Build successful!${NC}"
rm "$BUILD_LOG"

# Step 2: Flash
echo ""
echo -e "${BLUE}Step 2: Flashing to device...${NC}"
echo -e "${YELLOW}This will erase and reprogram your device.${NC}"
echo "Press Enter to continue or Ctrl+C to cancel..."
read

# Run flash script
./scripts/flash.sh

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Flash failed!${NC}"
    echo "Check your connection and try again."
    exit 1
fi

echo -e "${GREEN}✓ Flash successful!${NC}"

# Step 3: Monitor with analysis
echo ""
echo -e "${BLUE}Step 3: Monitoring test output...${NC}"
echo ""
echo "Expected test sequence:"
echo "1. ESP LCD initialization"
echo "2. Black screen"
echo "3. Color cycle (Red → Green → Blue → White)"
echo "4. Rectangle test"
echo "5. Text display"
echo "6. FPS measurement"
echo ""
echo -e "${YELLOW}Starting monitor (Ctrl+C to exit)...${NC}"
echo ""

# Create a timestamp for the log
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="esp_lcd_test_${TIMESTAMP}.log"

# Monitor and save output
echo "Saving output to: $LOG_FILE"
echo ""

# Use the Python monitor if available, otherwise use plain espflash
if [ -f "monitor-esp-lcd-test.py" ]; then
    espflash monitor | tee "$LOG_FILE" | python3 monitor-esp-lcd-test.py
else
    espflash monitor | tee "$LOG_FILE"
fi

echo ""
echo -e "${GREEN}Test completed!${NC}"
echo "Log saved to: $LOG_FILE"
echo ""
echo "Next steps:"
echo "1. Check if display showed test pattern"
echo "2. Review FPS measurement (should be >25)"
echo "3. If successful, disable test mode in main.rs"