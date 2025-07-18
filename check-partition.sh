#!/bin/bash
# Check which partition the device is running from

echo "Checking device partition status..."
echo "=================================="

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get device IP
DEVICE_IP="${1:-10.27.27.201}"

# Check device info
echo -e "${GREEN}Checking device at ${DEVICE_IP}...${NC}"
RESPONSE=$(curl -s "http://${DEVICE_IP}/api/system" 2>/dev/null)

if [ -z "$RESPONSE" ]; then
    echo -e "${RED}❌ Could not connect to device${NC}"
    exit 1
fi

# Extract version
VERSION=$(echo "$RESPONSE" | grep -o '"version":"[^"]*"' | cut -d'"' -f4)
echo -e "Device version: ${YELLOW}${VERSION}${NC}"

# Try OTA endpoint to see partition status
echo -e "\n${GREEN}Checking OTA status...${NC}"
OTA_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "http://${DEVICE_IP}/ota" 2>/dev/null)
HTTP_CODE=$(echo "$OTA_RESPONSE" | grep -o "HTTP_CODE:[0-9]*" | cut -d: -f2)

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✓ OTA is available - device is running from OTA partition${NC}"
    echo -e "  You can now use: ./ota.sh ${DEVICE_IP}"
else
    echo -e "${YELLOW}⚠ OTA endpoint returned: HTTP ${HTTP_CODE}${NC}"
    echo -e "${YELLOW}  Device is likely running from factory partition${NC}"
    echo -e "\nTo enable OTA:"
    echo -e "  1. The device needs OTA partitions (ota_0, ota_1)"
    echo -e "  2. Flash once more via USB to ensure proper partition table"
    echo -e "  3. Then do the first OTA update to move to ota_0"
fi

# Additional diagnostics
echo -e "\n${GREEN}Full system info:${NC}"
echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"