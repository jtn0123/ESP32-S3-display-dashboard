#!/bin/bash
# ESP32-S3 Dashboard - Toolchain Setup Script
# This script sets up the complete Rust toolchain for ESP32-S3 development

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}ESP32-S3 Rust Toolchain Setup${NC}"
echo "============================="
echo ""

# Check architecture
ARCH=$(arch)
if [ "$ARCH" != "arm64" ]; then
    echo -e "${YELLOW}Warning: This script is optimized for ARM64 macOS${NC}"
    echo "Architecture detected: $ARCH"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Step 1: Install Rust if not present
echo -e "${BLUE}Step 1: Checking Rust installation...${NC}"
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Rust not found. Installing...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}✓ Rust already installed: $(rustc --version)${NC}"
fi

# Source cargo environment
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Step 2: Install cargo-espflash
echo ""
echo -e "${BLUE}Step 2: Installing cargo-espflash...${NC}"
if ! command -v cargo-espflash &> /dev/null; then
    cargo install cargo-espflash
else
    echo -e "${GREEN}✓ cargo-espflash already installed${NC}"
fi

# Step 3: Install espflash
echo ""
echo -e "${BLUE}Step 3: Installing espflash...${NC}"
if ! command -v espflash &> /dev/null; then
    cargo install espflash
else
    echo -e "${GREEN}✓ espflash already installed${NC}"
fi

# Step 4: Download and setup ESP toolchain
echo ""
echo -e "${BLUE}Step 4: Setting up ESP toolchain...${NC}"

# Download espup if not present
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

# Step 5: Create environment setup script
echo ""
echo -e "${BLUE}Step 5: Creating environment script...${NC}"
cat > ~/esp-env.sh << 'EOF'
#!/bin/bash
# ESP32-S3 Rust Development Environment

# Source Rust environment
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Source ESP environment
if [ -f "$HOME/export-esp.sh" ]; then
    source "$HOME/export-esp.sh"
fi

# Set environment variables
export CARGO_BUILD_TARGET="xtensa-esp32s3-espidf"
export ESP_IDF_VERSION="v5.3"

echo "ESP32-S3 Rust environment loaded"
echo "  Rust: $(rustc --version 2>/dev/null || echo 'not found')"
echo "  Target: $CARGO_BUILD_TARGET"
echo "  IDF: $ESP_IDF_VERSION"
EOF

chmod +x ~/esp-env.sh

# Step 6: Verify installation
echo ""
echo -e "${BLUE}Step 6: Verifying installation...${NC}"
./check-toolchain.sh

# Step 7: Final instructions
echo ""
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "To use the toolchain, run one of these commands:"
echo ""
echo "  source ~/esp-env.sh          # Load environment"
echo "  ./compile.sh                 # Compile only"
echo "  ./flash.sh                   # Compile and flash to device"
echo ""
echo "For your shell profile (.zshrc or .bashrc), add:"
echo "  source ~/esp-env.sh"
echo ""
echo -e "${BLUE}Quick test:${NC}"
echo "  ./compile.sh --clean"