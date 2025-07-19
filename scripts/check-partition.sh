#!/usr/bin/env bash
# ESP32-S3 Dashboard - Partition Status Checker

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
DEVICE_IP="${1:-}"

echo -e "${GREEN}ESP32-S3 Partition Status Checker${NC}"
echo "================================="

# Function to check via network
check_network() {
    local ip=$1
    
    echo -e "\n${BLUE}Checking device at ${ip}...${NC}"
    
    # Check if device is reachable
    if ! curl -s --connect-timeout 2 "http://${ip}/api/system" >/dev/null 2>&1; then
        echo -e "${RED}✗ Device not reachable${NC}"
        return 1
    fi
    
    # Get system info
    echo -e "\n${BLUE}System Info:${NC}"
    local system_info=$(curl -s "http://${ip}/api/system")
    if [ -n "$system_info" ]; then
        echo "$system_info" | python3 -m json.tool || echo "$system_info"
    fi
    
    # Check OTA status
    echo -e "\n${BLUE}OTA Status:${NC}"
    local ota_status=$(curl -s "http://${ip}/api/ota/status" 2>/dev/null)
    if [ -n "$ota_status" ] && [[ ! "$ota_status" =~ "Not Found" ]]; then
        echo "$ota_status" | python3 -m json.tool || echo "$ota_status"
    else
        echo -e "${YELLOW}OTA endpoint not available${NC}"
        echo "Device may be running from factory partition"
    fi
    
    # Try to determine partition
    echo -e "\n${BLUE}Partition Analysis:${NC}"
    if [[ "$ota_status" =~ "unavailable" ]]; then
        echo "• Boot partition: Factory (no OTA available)"
        echo "• Next OTA will go to: ota_0"
    else
        echo "• Boot partition: OTA slot (ota_0 or ota_1)"
        echo "• OTA updates are available"
    fi
}

# Function to check via USB
check_usb() {
    local port="${PORT:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"
    
    if [ -z "$port" ]; then
        echo -e "${RED}✗ No USB device found${NC}"
        return 1
    fi
    
    echo -e "\n${BLUE}Checking device on ${port}...${NC}"
    
    # Find partition tool
    local parttool=""
    if [ -f "$HOME/.espressif/esp-idf/v5.3/components/partition_table/parttool.py" ]; then
        parttool="$HOME/.espressif/esp-idf/v5.3/components/partition_table/parttool.py"
    else
        echo -e "${YELLOW}parttool.py not found${NC}"
        echo "Install ESP-IDF to get detailed partition info"
        return 1
    fi
    
    # Get partition info
    echo -e "\n${BLUE}Partition Table:${NC}"
    python3 "$parttool" --port "$port" get_partition_info --info name type subtype offset size 2>/dev/null || {
        echo -e "${YELLOW}Could not read partition table${NC}"
        echo "Device may need to be in download mode"
    }
}

# Main logic
if [ -n "$DEVICE_IP" ]; then
    # Network check
    check_network "$DEVICE_IP"
else
    # Try to find device
    echo -e "${BLUE}No IP specified, searching for devices...${NC}"
    
    # Try common IPs
    found=0
    for ip in "10.27.27.201" "192.168.1.201" "192.168.0.201"; do
        if curl -s --connect-timeout 1 "http://${ip}/api/system" >/dev/null 2>&1; then
            echo -e "${GREEN}✓ Found device at ${ip}${NC}"
            check_network "$ip"
            found=1
            break
        fi
    done
    
    if [ $found -eq 0 ]; then
        echo -e "${YELLOW}No device found on network${NC}"
        echo -e "\n${BLUE}Checking USB connection...${NC}"
        check_usb
    fi
fi

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo "Usage: $0 [device-ip]"
echo "Example: $0 192.168.1.100"