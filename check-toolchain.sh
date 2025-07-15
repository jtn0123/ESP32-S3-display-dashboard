#!/bin/bash
# ESP32-S3 Dashboard - Toolchain Verification Script

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}ESP32-S3 Toolchain Status Check${NC}"
echo "================================"
echo ""

# Architecture
echo -e "${BLUE}System Information:${NC}"
echo "  Architecture: $(arch)"
echo "  macOS Version: $(sw_vers -productVersion)"
echo "  Current Shell: $SHELL"
echo ""

# Check export file
echo -e "${BLUE}ESP Environment:${NC}"
if [ -f ~/export-esp.sh ]; then
    echo -e "  ${GREEN}✓${NC} export-esp.sh found"
    # Source it to check tools
    source ~/export-esp.sh 2>/dev/null
else
    echo -e "  ${RED}✗${NC} export-esp.sh not found"
fi

# Check Rust
echo ""
echo -e "${BLUE}Rust Toolchain:${NC}"
if command -v rustc &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} rustc: $(rustc --version)"
    echo "     Default target: $(rustc -vV | grep host | cut -d' ' -f2)"
else
    echo -e "  ${RED}✗${NC} rustc not found"
fi

if command -v cargo &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} cargo: $(cargo --version)"
else
    echo -e "  ${RED}✗${NC} cargo not found"
fi

# Check for xtensa target
if rustup target list --installed 2>/dev/null | grep -q xtensa-esp32s3; then
    echo -e "  ${GREEN}✓${NC} xtensa-esp32s3 target installed"
else
    echo -e "  ${YELLOW}⚠${NC} xtensa-esp32s3 target not found"
fi

# Check ESP tools
echo ""
echo -e "${BLUE}ESP Tools:${NC}"

# Xtensa GCC
if command -v xtensa-esp32s3-elf-gcc &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} xtensa-gcc: $(xtensa-esp32s3-elf-gcc --version | head -1)"
else
    echo -e "  ${RED}✗${NC} xtensa-esp32s3-elf-gcc not found"
fi

# ESP Clang
if [ -n "$ESP_CLANG_PATH" ]; then
    if [ -f "$ESP_CLANG_PATH/bin/clang" ]; then
        echo -e "  ${GREEN}✓${NC} esp-clang: $ESP_CLANG_PATH"
        echo "     Version: $($ESP_CLANG_PATH/bin/clang --version | head -1)"
    else
        echo -e "  ${YELLOW}⚠${NC} ESP_CLANG_PATH set but clang not found"
    fi
else
    echo -e "  ${YELLOW}⚠${NC} ESP_CLANG_PATH not set"
fi

# espflash
if command -v espflash &> /dev/null; then
    echo -e "  ${GREEN}✓${NC} espflash: $(espflash --version)"
else
    echo -e "  ${YELLOW}⚠${NC} espflash not found"
fi

# Check environment variables
echo ""
echo -e "${BLUE}Environment Variables:${NC}"
env_vars=("ESP_IDF_VERSION" "ESP_IDF_TOOLS_PATH" "LIBCLANG_PATH" "CC" "CXX")
for var in "${env_vars[@]}"; do
    if [ -n "${!var}" ]; then
        echo -e "  ${GREEN}✓${NC} $var: ${!var}"
    else
        echo -e "  ${YELLOW}-${NC} $var: (not set)"
    fi
done

# Check project config
echo ""
echo -e "${BLUE}Project Configuration:${NC}"

# Check Cargo.toml
if [ -f Cargo.toml ]; then
    echo -e "  ${GREEN}✓${NC} Cargo.toml found"
    # Extract key dependencies
    echo "     Key dependencies:"
    grep -E "esp-idf-sys|esp-idf-svc|esp-idf-hal" Cargo.toml | sed 's/^/       /'
else
    echo -e "  ${RED}✗${NC} Cargo.toml not found"
fi

# Check sdkconfig
if [ -f sdkconfig.defaults ]; then
    echo -e "  ${GREEN}✓${NC} sdkconfig.defaults found"
    # Check for custom partition
    if grep -q "CONFIG_PARTITION_TABLE_CUSTOM" sdkconfig.defaults; then
        echo -e "     ${YELLOW}⚠${NC} Custom partition table configured"
    else
        echo -e "     ${GREEN}✓${NC} Using default partition table"
    fi
else
    echo -e "  ${YELLOW}⚠${NC} sdkconfig.defaults not found"
fi

# USB devices
echo ""
echo -e "${BLUE}USB Devices:${NC}"
usb_devices=$(ls /dev/tty.usb* 2>/dev/null)
if [ -n "$usb_devices" ]; then
    echo "$usb_devices" | while read device; do
        echo -e "  ${GREEN}✓${NC} $device"
    done
else
    echo -e "  ${YELLOW}⚠${NC} No USB devices found"
fi

echo ""
echo -e "${BLUE}Summary:${NC}"
# Count issues
issues=0
if ! command -v cargo &> /dev/null; then ((issues++)); fi
if ! command -v xtensa-esp32s3-elf-gcc &> /dev/null; then ((issues++)); fi
if [ ! -f ~/export-esp.sh ]; then ((issues++)); fi

if [ $issues -eq 0 ]; then
    echo -e "  ${GREEN}✓ Toolchain appears to be properly configured${NC}"
    echo -e "  Run ${BLUE}./build-native-arm64.sh${NC} to build the project"
else
    echo -e "  ${YELLOW}⚠ Found $issues potential issues${NC}"
    echo -e "  Run ${BLUE}./build-native-arm64.sh${NC} to install missing components"
fi