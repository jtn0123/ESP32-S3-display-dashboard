#!/bin/bash
# Complete ESP32-S3 Rust Toolchain Setup for ARM64 macOS

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}ESP32-S3 Complete Toolchain Setup (Native ARM64)${NC}"
echo "================================================="
echo ""

# Check architecture
if [ "$(arch)" != "arm64" ]; then
    echo -e "${RED}Error: Not running on ARM64!${NC}"
    exit 1
fi

# Step 1: Install Rust if needed
echo -e "${BLUE}Step 1: Checking Rust installation...${NC}"
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Rust not found. Installing...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}✓ Rust already installed: $(rustc --version)${NC}"
fi

# Make sure cargo is in PATH
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Step 2: Install ESP toolchain
echo ""
echo -e "${BLUE}Step 2: Setting up ESP toolchain...${NC}"

# Install cargo-espflash if needed
if ! command -v cargo-espflash &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-espflash...${NC}"
    cargo install cargo-espflash
fi

# Use local espup-arm64 binary
ESPUP_BIN="./espup-arm64"
if [ ! -f "$ESPUP_BIN" ]; then
    echo -e "${YELLOW}Downloading espup for ARM64...${NC}"
    curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-apple-darwin -o "$ESPUP_BIN"
    chmod +x "$ESPUP_BIN"
fi

# Install ESP toolchain
echo -e "${BLUE}Installing ESP toolchain...${NC}"
"$ESPUP_BIN" install \
    --toolchain esp \
    --targets xtensa-esp32s3-espidf \
    --export-file ~/export-esp.sh

# Source the environment
source ~/export-esp.sh

# Step 3: Fix partition table warning
echo ""
echo -e "${BLUE}Step 3: Checking partition configuration...${NC}"
if grep -q "CONFIG_PARTITION_TABLE_CUSTOM" sdkconfig.defaults; then
    echo -e "${YELLOW}Removing custom partition table config...${NC}"
    # Comment out custom partition config
    sed -i.bak 's/^CONFIG_PARTITION_TABLE_CUSTOM/#CONFIG_PARTITION_TABLE_CUSTOM/' sdkconfig.defaults
    sed -i.bak 's/^CONFIG_PARTITION_TABLE_FILENAME/#CONFIG_PARTITION_TABLE_FILENAME/' sdkconfig.defaults
    echo -e "${GREEN}✓ Switched to default partition table${NC}"
fi

# Step 4: Create a permanent setup script
echo ""
echo -e "${BLUE}Step 4: Creating environment setup script...${NC}"
cat > ~/esp-env.sh << 'EOF'
#!/bin/bash
# ESP32 Rust Development Environment Setup

# Source Rust environment
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Source ESP environment
if [ -f "$HOME/export-esp.sh" ]; then
    source "$HOME/export-esp.sh"
fi

# Set up for native ARM64 builds
export CARGO_BUILD_TARGET="xtensa-esp32s3-espidf"
export ESP_IDF_VERSION="v5.3"

echo "ESP32-S3 Rust environment loaded (ARM64 native)"
echo "  Rust: $(rustc --version 2>/dev/null || echo 'not found')"
echo "  Target: $CARGO_BUILD_TARGET"
echo "  IDF: $ESP_IDF_VERSION"
EOF

chmod +x ~/esp-env.sh
echo -e "${GREEN}✓ Created ~/esp-env.sh${NC}"
echo -e "  Add to your shell profile: ${BLUE}source ~/esp-env.sh${NC}"

# Step 5: Test build
echo ""
echo -e "${BLUE}Step 5: Ready to build!${NC}"
echo "Next steps:"
echo "  1. Source the environment: source ~/esp-env.sh"
echo "  2. Build the project: cargo build --release"
echo "  3. Flash to device: cargo espflash flash --release --monitor"
echo ""
echo -e "${GREEN}Setup complete!${NC}"

# Offer to test build now
read -p "Test build now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    source ~/esp-env.sh
    cargo build --release
fi