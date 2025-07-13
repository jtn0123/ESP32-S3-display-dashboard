#!/bin/bash

# ESP32-S3 Dashboard OTA Upload Script
# Automatically finds and uploads to your dashboard over WiFi

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}ESP32-S3 Dashboard OTA Upload Script${NC}"
echo "===================================="

# Configuration
BOARD="esp32:esp32:lilygo_t_display_s3"
HOSTNAME="esp32-dashboard"
SKETCH_DIR="dashboard"

# Change to sketch directory
cd "$SKETCH_DIR" 2>/dev/null || { echo -e "${RED}Error: dashboard directory not found${NC}"; exit 1; }
echo -e "Working directory: ${GREEN}$(pwd)${NC}"

# Try to find device by hostname first
echo -e "\n${YELLOW}Looking for device...${NC}"
DEVICE_IP=""

# Method 1: Try mDNS hostname
if ping -c 1 -W 1 ${HOSTNAME}.local >/dev/null 2>&1; then
    DEVICE_IP="${HOSTNAME}.local"
    echo -e "${GREEN}Found device via mDNS: ${DEVICE_IP}${NC}"
else
    # Method 2: Try to extract IP from arduino-cli board list
    echo "Scanning network for Arduino devices..."
    BOARD_LIST=$(arduino-cli board list --format json 2>/dev/null || echo "{}")
    
    # Look for our device in the list
    DEVICE_IP=$(echo "$BOARD_LIST" | grep -o '"address":"[^"]*esp32-dashboard[^"]*"' | cut -d'"' -f4 || true)
    
    if [ -z "$DEVICE_IP" ]; then
        # Method 3: Scan local network for port 3232 (OTA port)
        echo "Performing network scan for OTA devices..."
        
        # Get local network range
        LOCAL_IP=$(ipconfig getifaddr en0 2>/dev/null || ipconfig getifaddr en1 2>/dev/null || echo "")
        if [ -n "$LOCAL_IP" ]; then
            NETWORK_BASE=$(echo $LOCAL_IP | cut -d. -f1-3)
            echo "Scanning ${NETWORK_BASE}.0/24 network..."
            
            for i in {1..254}; do
                if timeout 0.1 nc -z ${NETWORK_BASE}.$i 3232 2>/dev/null; then
                    echo -e "${GREEN}Found OTA device at ${NETWORK_BASE}.$i${NC}"
                    DEVICE_IP="${NETWORK_BASE}.$i"
                    break
                fi
            done
        fi
    else
        echo -e "${GREEN}Found device in board list: ${DEVICE_IP}${NC}"
    fi
fi

# Check if device was found
if [ -z "$DEVICE_IP" ]; then
    echo -e "${RED}Error: Could not find esp32-dashboard on the network${NC}"
    echo "Make sure:"
    echo "  1. The device is powered on and connected to WiFi"
    echo "  2. You're on the same network as the device"
    echo "  3. The device shows 'OTA Ready' on the WiFi Status screen"
    echo ""
    echo "You can also manually specify the IP address:"
    echo "  ./upload-ota.sh 192.168.1.100"
    exit 1
fi

# Allow manual IP override
if [ -n "$1" ]; then
    DEVICE_IP="$1"
    echo -e "${YELLOW}Using manual IP address: ${DEVICE_IP}${NC}"
fi

# Compile the sketch
echo -e "\n${YELLOW}Compiling sketch...${NC}"
if arduino-cli compile --fqbn "$BOARD" .; then
    echo -e "${GREEN}Compilation successful!${NC}"
else
    echo -e "${RED}Compilation failed!${NC}"
    exit 1
fi

# Upload via OTA
echo -e "\n${YELLOW}Uploading to ${DEVICE_IP} via OTA...${NC}"
echo "Watch your device screen for progress..."

if arduino-cli upload -p "$DEVICE_IP" --fqbn "$BOARD" --protocol network .; then
    echo -e "${GREEN}OTA Upload successful!${NC}"
    echo -e "${GREEN}Your device will restart automatically.${NC}"
else
    echo -e "${RED}OTA Upload failed!${NC}"
    echo "Troubleshooting:"
    echo "  - Check if device shows 'OTA Ready' on WiFi screen"
    echo "  - Ensure good WiFi signal strength"
    echo "  - Try moving closer to the device"
    echo "  - Use USB cable upload as fallback"
    exit 1
fi

echo -e "\n${GREEN}Dashboard updated successfully via OTA!${NC}"