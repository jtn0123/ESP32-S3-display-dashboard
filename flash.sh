#!/bin/bash
# ESP32-S3 Dashboard Flash Script

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}ESP32-S3 Dashboard - Rust Flash Tool${NC}"
echo "======================================"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found!${NC}"
    echo "Please install Rust and run: source ~/export-esp.sh"
    exit 1
fi

# Default to release build
BUILD_TYPE=${1:-release}

if [ "$BUILD_TYPE" == "debug" ]; then
    echo -e "${YELLOW}Building in debug mode...${NC}"
    cargo build
else
    echo -e "${GREEN}Building in release mode...${NC}"
    cargo build --release
fi

# Check if build succeeded
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

# Flash and monitor
echo -e "${GREEN}Flashing to device...${NC}"
cargo espflash flash --monitor --release

# Alternative method if cargo-espflash is not installed
if [ $? -ne 0 ]; then
    echo -e "${YELLOW}Trying alternative flash method...${NC}"
    espflash flash target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard --monitor
fi