#!/bin/bash
# ESP32-S3 Dashboard - Native ARM64 Build Script for macOS

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}ESP32-S3 Dashboard - Native ARM64 Build${NC}"
echo "========================================="
echo -e "${BLUE}Architecture: $(arch)${NC}"
echo ""

# Verify we're on ARM64
if [ "$(arch)" != "arm64" ]; then
    echo -e "${RED}Error: Not running on ARM64!${NC}"
    echo "This script is for native ARM64 builds."
    exit 1
fi

# Check if toolchain is already set up
check_toolchain() {
    if [ -f ~/export-esp.sh ]; then
        echo -e "${GREEN}✓ ESP toolchain export file found${NC}"
        source ~/export-esp.sh
        
        # Check for required tools
        if command -v xtensa-esp32s3-elf-gcc &> /dev/null; then
            echo -e "${GREEN}✓ Xtensa GCC found: $(xtensa-esp32s3-elf-gcc --version | head -1)${NC}"
        else
            echo -e "${YELLOW}⚠ Xtensa GCC not found in PATH${NC}"
            return 1
        fi
        
        if command -v cargo &> /dev/null; then
            echo -e "${GREEN}✓ Cargo found: $(cargo --version)${NC}"
        else
            echo -e "${RED}✗ Cargo not found!${NC}"
            return 1
        fi
        
        return 0
    else
        echo -e "${YELLOW}ESP toolchain not found${NC}"
        return 1
    fi
}

# Install toolchain if needed
install_toolchain() {
    echo -e "${YELLOW}Installing ESP-RS toolchain for ARM64...${NC}"
    
    # Use the local espup-arm64 binary
    ESPUP_BIN="./espup-arm64"
    
    if [ ! -f "$ESPUP_BIN" ]; then
        echo -e "${RED}Error: espup-arm64 not found in current directory${NC}"
        exit 1
    fi
    
    # Make sure it's executable
    chmod +x "$ESPUP_BIN"
    
    # Install toolchain for native ARM64
    echo -e "${BLUE}Running: $ESPUP_BIN install${NC}"
    "$ESPUP_BIN" install \
        --toolchain esp \
        --targets xtensa-esp32s3-espidf \
        --export-file ~/export-esp.sh
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Failed to install toolchain${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Toolchain installed successfully${NC}"
}

# Main execution
echo -e "${BLUE}Step 1: Checking toolchain...${NC}"
if ! check_toolchain; then
    echo -e "${YELLOW}Toolchain not properly set up${NC}"
    read -p "Install toolchain now? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_toolchain
        source ~/export-esp.sh
    else
        echo -e "${RED}Cannot continue without toolchain${NC}"
        exit 1
    fi
fi

# Source the environment
echo -e "${BLUE}Step 2: Setting up environment...${NC}"
source ~/export-esp.sh

# Set build environment for native ARM64
export CARGO_BUILD_TARGET="xtensa-esp32s3-espidf"
export ESP_IDF_VERSION="v5.3"

# Use LLVM/Clang for ARM64 (not GCC mode)
unset ESP_IDF_SYS_COMPILER_FAMILY
export CC="clang"
export CXX="clang++"

echo -e "${GREEN}Environment configured for native ARM64 build${NC}"
echo "  TARGET: $CARGO_BUILD_TARGET"
echo "  IDF: $ESP_IDF_VERSION"
echo "  CC: $CC"

# Check partition table config
echo -e "${BLUE}Step 3: Checking partition table...${NC}"
if grep -q "CONFIG_PARTITION_TABLE_CUSTOM" sdkconfig.defaults 2>/dev/null; then
    echo -e "${YELLOW}Warning: Custom partition table configured${NC}"
    echo "Consider using default partition table for initial testing"
else
    echo -e "${GREEN}✓ Using default partition table${NC}"
fi

# Clean build directory
echo -e "${BLUE}Step 4: Cleaning previous build...${NC}"
cargo clean

# Build the project
echo -e "${BLUE}Step 5: Building project (release mode)...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    echo "Troubleshooting tips:"
    echo "  1. Check that all dependencies are compatible with ARM64"
    echo "  2. Verify esp-idf-sys version matches your IDF version"
    echo "  3. Try 'cargo update' to refresh dependencies"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"

# Find the binary
BINARY_PATH="target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at expected path${NC}"
    echo "Expected: $BINARY_PATH"
    exit 1
fi

# Show binary info
echo -e "${BLUE}Binary info:${NC}"
ls -lh "$BINARY_PATH"
echo "Size: $(du -h "$BINARY_PATH" | cut -f1)"

# Flash options
echo -e "${BLUE}Step 6: Flash to device${NC}"
echo "Available ports:"
ls /dev/tty.usb* 2>/dev/null || echo "  No USB devices found"

read -p "Flash to device now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Try cargo espflash first
    if command -v cargo-espflash &> /dev/null; then
        echo -e "${BLUE}Using cargo espflash...${NC}"
        cargo espflash flash --release --monitor
    else
        # Try espflash directly
        PORT=$(ls /dev/tty.usbmodem* 2>/dev/null | head -1)
        if [ -z "$PORT" ]; then
            echo -e "${RED}No USB device found!${NC}"
            echo "Please connect your ESP32-S3 device"
            exit 1
        fi
        
        echo -e "${BLUE}Flashing to $PORT...${NC}"
        espflash flash "$BINARY_PATH" --port "$PORT" --monitor
    fi
fi

echo -e "${GREEN}Done!${NC}"