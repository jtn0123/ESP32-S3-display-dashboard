#!/bin/bash
# ESP32-S3 Dashboard - Flash Script
# This script compiles and flashes the Rust project to ESP32-S3

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}ESP32-S3 Dashboard - Flash Tool${NC}"
echo "==============================="
echo ""

# Function to show usage
usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --release    Build in release mode (default)"
    echo "  --debug      Build in debug mode"
    echo "  --clean      Clean before building"
    echo "  --monitor    Open serial monitor after flashing (default)"
    echo "  --no-monitor Skip serial monitor"
    echo "  --port PORT  Specify USB port (auto-detect if not specified)"
    echo "  --verbose    Verbose output"
    echo "  --help       Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                    # Build release and flash with monitor"
    echo "  $0 --debug            # Build debug and flash"
    echo "  $0 --port /dev/tty.usbmodem14201"
    exit 1
}

# Parse arguments
BUILD_MODE="--release"
CLEAN=false
MONITOR="--monitor"
PORT=""
VERBOSE=""
CARGO_ARGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_MODE="--release"
            CARGO_ARGS="--release"
            shift
            ;;
        --debug)
            BUILD_MODE=""
            CARGO_ARGS=""
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --monitor)
            MONITOR="--monitor"
            shift
            ;;
        --no-monitor)
            MONITOR=""
            shift
            ;;
        --port)
            PORT="--port $2"
            shift 2
            ;;
        --verbose)
            VERBOSE="--verbose"
            shift
            ;;
        --help|-h)
            usage
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            ;;
    esac
done

# Check architecture
ARCH=$(arch)
echo -e "${BLUE}Architecture: ${ARCH}${NC}"

# Source Rust environment
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
else
    echo -e "${RED}Error: Rust environment not found!${NC}"
    echo "Please install Rust first: https://rustup.rs/"
    exit 1
fi

# Source ESP environment
if [ -f "$HOME/export-esp.sh" ]; then
    source "$HOME/export-esp.sh"
elif [ -f ~/esp-env.sh ]; then
    source ~/esp-env.sh
else
    echo -e "${RED}Error: ESP environment not found!${NC}"
    echo "Run ./setup-toolchain.sh first to install ESP toolchain"
    exit 1
fi

# Verify tools are available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found!${NC}"
    exit 1
fi

# Check for cargo-espflash
if ! command -v cargo-espflash &> /dev/null; then
    echo -e "${YELLOW}cargo-espflash not found. Installing...${NC}"
    cargo install cargo-espflash
fi

# Auto-detect port if not specified
if [ -z "$PORT" ]; then
    echo -e "${BLUE}Auto-detecting USB port...${NC}"
    USB_DEVICES=$(ls /dev/tty.usb* /dev/cu.usb* 2>/dev/null | head -1)
    if [ -n "$USB_DEVICES" ]; then
        PORT="--port $USB_DEVICES"
        echo -e "  Found: $USB_DEVICES"
    else
        echo -e "${YELLOW}No USB device found. Will try default port.${NC}"
    fi
fi

# Show configuration
echo ""
echo -e "${BLUE}Flash Configuration:${NC}"
if [ -n "$BUILD_MODE" ]; then
    echo "  Mode: Release (optimized)"
else
    echo "  Mode: Debug"
fi
echo "  Target: xtensa-esp32s3-espidf"
if [ -n "$PORT" ]; then
    echo "  Port: $(echo $PORT | cut -d' ' -f2)"
fi
if [ -n "$MONITOR" ]; then
    echo "  Monitor: Yes"
else
    echo "  Monitor: No"
fi
echo ""

# Clean if requested
if [ "$CLEAN" = true ]; then
    echo -e "${YELLOW}Cleaning previous build...${NC}"
    cargo clean
    echo ""
fi

# Set ESP-IDF version to 5.3.3 LTS
export ESP_IDF_VERSION="v5.3.3"
echo -e "${BLUE}ESP-IDF Version: v5.3.3 LTS${NC}"

# Build first (using compile.sh logic)
echo -e "${GREEN}Building project...${NC}"
cargo build $BUILD_MODE $VERBOSE

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Build failed!${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Build successful!${NC}"

# Flash to device
echo ""
echo -e "${GREEN}Flashing to device...${NC}"

# Try cargo espflash first (recommended method)
cargo espflash flash $CARGO_ARGS $PORT $MONITOR $VERBOSE

FLASH_RESULT=$?

if [ $FLASH_RESULT -ne 0 ]; then
    echo ""
    echo -e "${YELLOW}Trying alternative flash method...${NC}"
    
    # Determine binary path
    if [ -n "$BUILD_MODE" ]; then
        BINARY_PATH="target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
    else
        BINARY_PATH="target/xtensa-esp32s3-espidf/debug/esp32-s3-dashboard"
    fi
    
    # Try espflash directly
    if command -v espflash &> /dev/null; then
        espflash flash "$BINARY_PATH" $PORT $MONITOR
        FLASH_RESULT=$?
    else
        echo -e "${RED}espflash not found. Please install it:${NC}"
        echo "  cargo install espflash"
        exit 1
    fi
fi

if [ $FLASH_RESULT -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Flash successful!${NC}"
    if [ -z "$MONITOR" ]; then
        echo ""
        echo "To monitor serial output, run:"
        echo "  espflash monitor $PORT"
    fi
else
    echo ""
    echo -e "${RED}✗ Flash failed!${NC}"
    echo ""
    echo "Troubleshooting tips:"
    echo "  1. Check that your ESP32-S3 is connected"
    echo "  2. Verify the correct port with: ls /dev/tty.usb*"
    echo "  3. Try specifying port: $0 --port /dev/tty.usbmodem14201"
    echo "  4. Ensure you have permissions to access the USB device"
    echo "  5. Try pressing BOOT button on the board while flashing"
    exit 1
fi