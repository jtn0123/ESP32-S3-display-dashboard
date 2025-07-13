#!/bin/bash

# ESP32-S3 Dashboard Upload Script
# This script ensures consistent uploads regardless of current directory

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
DASHBOARD_DIR="$SCRIPT_DIR/dashboard"

echo -e "${YELLOW}ESP32-S3 Dashboard Upload Script${NC}"
echo "================================"

# Change to dashboard directory
cd "$DASHBOARD_DIR" || { echo -e "${RED}Error: Dashboard directory not found${NC}"; exit 1; }

# Show current directory
echo -e "Working directory: ${GREEN}$(pwd)${NC}"

# Check if board is connected
echo -e "\n${YELLOW}Checking for connected board...${NC}"
BOARD_PORT=$(arduino-cli board list | grep "ESP32" | awk '{print $1}')

if [ -z "$BOARD_PORT" ]; then
    echo -e "${RED}Error: No ESP32 board detected${NC}"
    echo "Please check your USB connection"
    exit 1
fi

echo -e "${GREEN}Found board on port: $BOARD_PORT${NC}"

# Compile the sketch
echo -e "\n${YELLOW}Compiling sketch...${NC}"
if arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 .; then
    echo -e "${GREEN}Compilation successful!${NC}"
else
    echo -e "${RED}Compilation failed!${NC}"
    exit 1
fi

# Upload to board
echo -e "\n${YELLOW}Uploading to board...${NC}"
if arduino-cli upload -p "$BOARD_PORT" --fqbn esp32:esp32:lilygo_t_display_s3 .; then
    echo -e "${GREEN}Upload successful!${NC}"
    echo -e "\n${GREEN}Dashboard uploaded successfully to $BOARD_PORT${NC}"
else
    echo -e "${RED}Upload failed!${NC}"
    exit 1
fi