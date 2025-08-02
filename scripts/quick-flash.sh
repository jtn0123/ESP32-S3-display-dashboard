#!/bin/bash
# Quick build, flash and monitor script for development
# Combines compile.sh, flash.sh and monitor in one command

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}ESP32-S3 Dashboard - Quick Flash & Monitor${NC}"
echo "==========================================="

# Parse arguments
MONITOR_TYPE="serial"  # Default to serial monitor
NO_ERASE=false
CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --telnet)
            MONITOR_TYPE="telnet"
            shift
            ;;
        --no-erase)
            NO_ERASE=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --telnet     Use telnet monitor instead of serial"
            echo "  --no-erase   Skip full chip erase (faster)"
            echo "  --clean      Clean build before compiling"
            echo "  --help       Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Step 1: Compile
echo -e "\n${YELLOW}Step 1: Building...${NC}"
if [ "$CLEAN" = true ]; then
    ./compile.sh --clean
else
    ./compile.sh
fi

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

# Step 2: Flash
echo -e "\n${YELLOW}Step 2: Flashing...${NC}"
if [ "$NO_ERASE" = true ]; then
    ./scripts/flash.sh --no-erase
else
    ./scripts/flash.sh
fi

if [ $? -ne 0 ]; then
    echo -e "${RED}Flash failed!${NC}"
    exit 1
fi

# Step 3: Monitor
echo -e "\n${YELLOW}Step 3: Starting monitor...${NC}"
if [ "$MONITOR_TYPE" = "telnet" ]; then
    echo "Waiting 10 seconds for device to boot and connect to WiFi..."
    sleep 10
    echo -e "${GREEN}Starting telnet monitor (Ctrl+C to exit)${NC}"
    ./scripts/monitor-telnet.py
else
    echo -e "${GREEN}Starting serial monitor (Ctrl+] to exit)${NC}"
    espflash monitor
fi