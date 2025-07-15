#!/bin/bash
# Quick build test for ESP32-S3 Dashboard

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}ESP32-S3 Dashboard Build Test${NC}"
echo "============================="
echo ""

# Try to find and source environments
echo -e "${BLUE}Loading environment...${NC}"

# Rust environment
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Loaded Rust environment${NC}"
else
    echo -e "${YELLOW}⚠ Rust environment not found${NC}"
fi

# ESP environment
if [ -f "$HOME/export-esp.sh" ]; then
    source "$HOME/export-esp.sh"
    echo -e "${GREEN}✓ Loaded ESP environment${NC}"
else
    echo -e "${RED}✗ ESP environment not found${NC}"
    echo "Run ./setup-complete-toolchain.sh first"
    exit 1
fi

# Check tools
echo ""
echo -e "${BLUE}Tool availability:${NC}"
echo -n "  Cargo: "
if command -v cargo &> /dev/null; then
    echo -e "${GREEN}$(cargo --version)${NC}"
else
    echo -e "${RED}NOT FOUND${NC}"
    exit 1
fi

echo -n "  Rustc: "
if command -v rustc &> /dev/null; then
    echo -e "${GREEN}$(rustc --version)${NC}"
else
    echo -e "${RED}NOT FOUND${NC}"
    exit 1
fi

echo -n "  Xtensa GCC: "
if command -v xtensa-esp-elf-gcc &> /dev/null; then
    echo -e "${GREEN}Found${NC}"
else
    echo -e "${RED}NOT FOUND${NC}"
fi

# Set build environment
export CARGO_BUILD_TARGET="xtensa-esp32s3-espidf"
export ESP_IDF_VERSION="v5.3"

echo ""
echo -e "${BLUE}Build configuration:${NC}"
echo "  Target: $CARGO_BUILD_TARGET"
echo "  IDF Version: $ESP_IDF_VERSION"
echo "  Architecture: $(arch)"

# Clean and build
echo ""
echo -e "${BLUE}Starting build...${NC}"
cargo clean
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Build successful!${NC}"
    
    # Show binary info
    BINARY="target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
    if [ -f "$BINARY" ]; then
        echo -e "${BLUE}Binary info:${NC}"
        ls -lh "$BINARY"
        echo "  Size: $(du -h "$BINARY" | cut -f1)"
    fi
    
    # Offer to flash
    echo ""
    USB_DEVICES=$(ls /dev/tty.usb* 2>/dev/null)
    if [ -n "$USB_DEVICES" ]; then
        echo -e "${BLUE}USB devices found:${NC}"
        echo "$USB_DEVICES"
        echo ""
        read -p "Flash to device? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            cargo espflash flash --release --monitor
        fi
    else
        echo -e "${YELLOW}No USB devices found${NC}"
        echo "Connect your ESP32-S3 and run: cargo espflash flash --release --monitor"
    fi
else
    echo ""
    echo -e "${RED}✗ Build failed${NC}"
    echo "Check the error messages above"
    exit 1
fi