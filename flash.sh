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
    echo "  --erase-flash Erase entire flash before programming"
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
ERASE_FLASH=false

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
        --erase-flash)
            ERASE_FLASH=true
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
    echo -e "${RED}✗ Error: Rust environment not found!${NC}"
    echo -e "${YELLOW}Please install Rust first:${NC} https://rustup.rs/"
    exit 1
fi

# Source ESP environment
if [ -f "$HOME/export-esp.sh" ]; then
    source "$HOME/export-esp.sh"
elif [ -f ~/esp-env.sh ]; then
    source ~/esp-env.sh
else
    echo -e "${RED}✗ Error: ESP environment not found!${NC}"
    echo -e "${YELLOW}Run ./setup-toolchain.sh first to install ESP toolchain${NC}"
    exit 1
fi

# Verify tools are available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Error: cargo not found!${NC}"
    exit 1
fi

# Check for cargo-espflash
if ! command -v cargo-espflash &> /dev/null; then
    echo -e "${YELLOW}cargo-espflash not found${NC}"
    echo -e "${BLUE}Installing cargo-espflash...${NC}"
    cargo install cargo-espflash
fi

# Auto-detect port if not specified
if [ -z "$PORT" ]; then
    echo -e "${BLUE}Auto-detecting USB port...${NC}"
    USB_DEVICES=$(ls /dev/tty.usb* /dev/cu.usb* 2>/dev/null | head -1)
    if [ -n "$USB_DEVICES" ]; then
        PORT="--port $USB_DEVICES"
        echo -e "${GREEN}  Found: $USB_DEVICES${NC}"
    else
        echo -e "${YELLOW}  No USB device found. Will try default port.${NC}"
    fi
fi

# Show configuration
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

# Clean if requested
if [ "$CLEAN" = true ]; then
    echo -e "${YELLOW}Cleaning previous build...${NC}"
    cargo clean
fi

# Set ESP-IDF version to 5.3.3 LTS
export ESP_IDF_VERSION="v5.3.3"
echo -e "${BLUE}ESP-IDF Version: v5.3.3 LTS${NC}"

# Determine binary path
if [ -n "$BUILD_MODE" ]; then
    BINARY_PATH="target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"
else
    BINARY_PATH="target/xtensa-esp32s3-espidf/debug/esp32-s3-dashboard"
fi

# Check if binary exists
if [ -f "$BINARY_PATH" ]; then
    echo -e "${GREEN}Binary found:${NC}"
    SIZE=$(ls -lh "$BINARY_PATH" | awk '{print $5}')
    echo -e "  Path: $BINARY_PATH"
    echo -e "  Size: $SIZE"
    echo -e "${YELLOW}Skipping build - using existing binary${NC}"
    echo -e "${BLUE}Tip: Run './compile.sh' or use --clean to rebuild${NC}"
else
    # Build if binary doesn't exist
    echo -e "${GREEN}Building project...${NC}"
    
    # Start timer
    BUILD_START=$(date +%s)
    
    cargo build $BUILD_MODE $VERBOSE

    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ Build failed!${NC}"
        exit 1
    fi
    
    # Calculate build time
    BUILD_END=$(date +%s)
    BUILD_TIME=$((BUILD_END - BUILD_START))

    echo -e "${GREEN}✓ Build successful!${NC}"
    echo -e "  Time: ${BUILD_TIME}s"
    # Show binary info
    SIZE=$(ls -lh "$BINARY_PATH" | awk '{print $5}')
    echo -e "  Binary: $BINARY_PATH"
    echo -e "  Size: $SIZE"
fi

# Flash to device
echo ""
echo -e "${GREEN}Starting flash process...${NC}"

# ALWAYS use espflash directly with 16MB flash size
# This avoids all the partition table confusion
if [ -f "$HOME/.cargo/bin/espflash" ]; then
    ESPFLASH="$HOME/.cargo/bin/espflash"
elif command -v espflash &> /dev/null; then
    ESPFLASH="espflash"
else
    echo -e "${RED}✗ espflash not found${NC}"
    echo -e "${YELLOW}Installing espflash v3.3.0...${NC}"
    cargo install espflash@3.3.0 --force
    ESPFLASH="$HOME/.cargo/bin/espflash"
fi

# Check espflash version
ESPFLASH_VERSION=$($ESPFLASH --version | awk '{print $2}')
echo -e "${BLUE}Using espflash version: $ESPFLASH_VERSION${NC}"

# Erase flash if requested
if [ "$ERASE_FLASH" = true ]; then
    echo -e "${YELLOW}Erasing entire flash memory...${NC}"
    $ESPFLASH erase-flash $PORT
    ERASE_RESULT=$?
    if [ $ERASE_RESULT -eq 0 ]; then
        echo -e "${GREEN}✓ Flash erased successfully${NC}"
    else
        echo -e "${RED}✗ Flash erase failed!${NC}"
        exit 1
    fi
fi

# Check if we should use esptool.py for better partition table handling
USE_ESPTOOL=false
CUSTOM_PARTITION_TABLE="partitions_ota_1_5mb.csv"
FALLBACK_PARTITION_TABLE="partitions_16mb_ota.csv"
DEFAULT_PARTITION_TABLE="$HOME/.espressif/esp-idf/v5.3/components/partition_table/partitions_two_ota.csv"

# Check if custom partition table exists
if [ -f "$CUSTOM_PARTITION_TABLE" ]; then
    echo -e "${BLUE}Found custom partition table: $CUSTOM_PARTITION_TABLE${NC}"
    echo -e "${GREEN}  App partitions: 1.5MB each (factory, ota_0, ota_1)${NC}"
    USE_ESPTOOL=true
fi

if [ "$USE_ESPTOOL" = true ]; then
    # Use esptool.py for precise control over partition table flashing
    echo -e "${BLUE}Using esptool.py for partition table support...${NC}"
    
    # Find esptool.py
    if [ -f ".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
        ESPTOOL=".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
    elif [ -f "$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
        ESPTOOL="$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
    else
        echo -e "${YELLOW}esptool.py not found, falling back to espflash${NC}"
        USE_ESPTOOL=false
    fi
fi

if [ "$USE_ESPTOOL" = true ]; then
    # Convert partition table to binary
    echo -e "${BLUE}Converting partition table to binary...${NC}"
    python3 "$HOME/.espressif/esp-idf/v5.3/components/partition_table/gen_esp32part.py" \
        --verify "$CUSTOM_PARTITION_TABLE" partitions_16mb_ota.bin
    
    if [ ! -f "partitions_16mb_ota.bin" ]; then
        echo -e "${RED}Failed to generate partition table binary${NC}"
        echo -e "${YELLOW}Falling back to espflash...${NC}"
        USE_ESPTOOL=false
    fi
fi

if [ "$USE_ESPTOOL" = true ]; then
    # Find bootloader
    BOOTLOADER="target/xtensa-esp32s3-espidf/release/bootloader.bin"
    if [ ! -f "$BOOTLOADER" ]; then
        BOOTLOADER=$(find target -name "bootloader.bin" -type f 2>/dev/null | head -1)
    fi
    
    if [ -z "$BOOTLOADER" ] || [ ! -f "$BOOTLOADER" ]; then
        echo -e "${YELLOW}Bootloader not found, falling back to espflash${NC}"
        USE_ESPTOOL=false
    fi
fi

if [ "$USE_ESPTOOL" = true ]; then
    # Extract port for esptool
    PORT_PATH=$(echo $PORT | cut -d' ' -f2)
    if [ -z "$PORT_PATH" ]; then
        PORT_PATH=$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)
    fi
    
    echo -e "${GREEN}Flashing with esptool.py (better partition support)...${NC}"
    echo -e "  Bootloader: $BOOTLOADER"
    echo -e "  Partition:  partitions_16mb_ota.bin"
    echo -e "  App:        $BINARY_PATH"
    
    # Flash with esptool.py
    # First erase the partition table area to force a rewrite
    echo -e "${BLUE}Erasing partition table area...${NC}"
    $ESPTOOL \
        --chip esp32s3 \
        --port "$PORT_PATH" \
        --baud 921600 \
        erase_region 0x8000 0x1000
    
    # Now flash everything
    echo -e "${GREEN}Writing bootloader, partition table, and app...${NC}"
    $ESPTOOL \
        --chip esp32s3 \
        --port "$PORT_PATH" \
        --baud 921600 \
        --before default_reset \
        --after hard_reset \
        write_flash \
        --flash_mode dio \
        --flash_freq 40m \
        --flash_size 16MB \
        0x0 "$BOOTLOADER" \
        0x8000 partitions_16mb_ota.bin \
        0x10000 "$BINARY_PATH"
    FLASH_RESULT=$?
    
    # Clean up binary partition file
    rm -f partitions_16mb_ota.bin
    
    # Handle monitor if requested
    if [ $FLASH_RESULT -eq 0 ] && [ -n "$MONITOR" ]; then
        echo -e "${BLUE}Waiting for device to reset...${NC}"
        sleep 2  # Give device time to reset after flashing
        echo -e "${BLUE}Starting monitor...${NC}"
        espflash monitor --port "$PORT_PATH" --no-stub
    fi
else
    # Fall back to espflash
    echo -e "${BLUE}Using espflash...${NC}"
    
    # If monitor is requested, flash without monitor first then start it separately
    if [ -n "$MONITOR" ]; then
        FLASH_MONITOR=""
    else
        FLASH_MONITOR="$MONITOR"
    fi
    
    if [ -f "$CUSTOM_PARTITION_TABLE" ]; then
        $ESPFLASH flash --flash-size 16mb --partition-table "$CUSTOM_PARTITION_TABLE" "$BINARY_PATH" $PORT $FLASH_MONITOR
    elif [ -f "$DEFAULT_PARTITION_TABLE" ]; then
        echo -e "${YELLOW}Warning: Using default 1MB partitions - may be too small!${NC}"
        $ESPFLASH flash --flash-size 16mb --partition-table "$DEFAULT_PARTITION_TABLE" "$BINARY_PATH" $PORT $FLASH_MONITOR
    else
        echo -e "${YELLOW}Warning: No partition table found${NC}"
        $ESPFLASH flash --flash-size 16mb "$BINARY_PATH" $PORT $FLASH_MONITOR
    fi
    FLASH_RESULT=$?
    
    # Start monitor separately if requested
    if [ $FLASH_RESULT -eq 0 ] && [ -n "$MONITOR" ]; then
        echo -e "${BLUE}Waiting for device to reset...${NC}"
        sleep 2
        echo -e "${BLUE}Starting monitor...${NC}"
        # Extract port for monitor
        PORT_PATH=$(echo $PORT | cut -d' ' -f2)
        if [ -z "$PORT_PATH" ]; then
            PORT_PATH=$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)
        fi
        espflash monitor --port "$PORT_PATH" --no-stub
    fi
fi

if [ $FLASH_RESULT -eq 0 ]; then
    echo -e "${GREEN}✓ Flash successful!${NC}"
    if [ -z "$MONITOR" ]; then
        echo -e "${BLUE}To monitor serial output, run:${NC}"
        echo "  espflash monitor $PORT"
    fi
else
    echo -e "${RED}✗ Flash failed!${NC}"
    echo -e "${YELLOW}Troubleshooting tips:${NC}"
    echo "  1. Check that your ESP32-S3 is connected"
    echo "  2. Verify the correct port with: ls /dev/tty.usb*"
    echo "  3. Try specifying port: $0 --port /dev/tty.usbmodem14201"
    echo "  4. Ensure you have permissions to access the USB device"
    echo "  5. Try pressing BOOT button on the board while flashing"
    exit 1
fi