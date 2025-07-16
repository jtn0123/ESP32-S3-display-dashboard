#!/bin/bash
# ESP32-S3 Dashboard - Compile Script
# This script compiles the Rust project for ESP32-S3

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}ESP32-S3 Dashboard - Rust Compiler${NC}"
echo "==================================="
echo ""

# Function to show usage
usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --release    Build in release mode (default)"
    echo "  --debug      Build in debug mode"
    echo "  --clean      Clean before building"
    echo "  --verbose    Verbose output"
    echo "  --help       Show this help message"
    exit 1
}

# Parse arguments
BUILD_MODE="--release"
CLEAN=false
VERBOSE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_MODE="--release"
            shift
            ;;
        --debug)
            BUILD_MODE=""
            shift
            ;;
        --clean)
            CLEAN=true
            shift
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

# Verify cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found!${NC}"
    exit 1
fi

# Show build configuration
echo -e "${BLUE}Build Configuration:${NC}"
if [ -n "$BUILD_MODE" ]; then
    echo "  Mode: Release (optimized)"
else
    echo "  Mode: Debug"
fi
echo "  Target: xtensa-esp32s3-espidf"
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

# Build the project
echo -e "${GREEN}Starting build...${NC}"
START_TIME=$(date +%s)

cargo build $BUILD_MODE $VERBOSE

BUILD_RESULT=$?
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

if [ $BUILD_RESULT -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Build successful!${NC}"
    echo -e "  Time: ${ELAPSED}s"
    
    # Show binary info
    if [ -n "$BUILD_MODE" ]; then
        BINARY_PATH="target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
    else
        BINARY_PATH="target/xtensa-esp32s3-espidf/debug/esp32-s3-dashboard"
    fi
    
    if [ -f "$BINARY_PATH" ]; then
        SIZE=$(du -h "$BINARY_PATH" | cut -f1)
        echo -e "  Binary: $BINARY_PATH"
        echo -e "  Size: $SIZE"
    fi
else
    echo ""
    echo -e "${RED}✗ Build failed!${NC}"
    echo ""
    echo "Troubleshooting tips:"
    echo "  1. Check error messages above"
    echo "  2. Try running with --clean option"
    echo "  3. Ensure toolchain is properly installed"
    echo "  4. Run ./check-toolchain.sh to verify setup"
    exit 1
fi