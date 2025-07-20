#!/bin/bash
# ESP32-S3 Dashboard - Build Fix Script
# This script fixes common build issues

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}ESP32-S3 Dashboard - Build Fix Script${NC}"
echo "====================================="
echo ""

# Kill any stuck processes
echo -e "${YELLOW}Killing any stuck cargo/rust processes...${NC}"
pkill -9 cargo 2>/dev/null
pkill -9 rustc 2>/dev/null
pkill -9 rust-analyzer 2>/dev/null
echo "✓ Processes cleaned"

# Remove lock files
echo -e "${YELLOW}Removing lock files...${NC}"
find . -name ".cargo-lock" -delete 2>/dev/null
rm -f ~/.cargo/.package-cache* 2>/dev/null
echo "✓ Lock files removed"

# Clear target directory locks
echo -e "${YELLOW}Clearing target directory...${NC}"
rm -rf target/.cargo-lock target/*/.cargo-lock target/*/*/.cargo-lock 2>/dev/null
echo "✓ Target locks cleared"

# Optional: Clear cargo cache (only if requested)
if [ "$1" == "--deep-clean" ]; then
    echo -e "${YELLOW}Deep cleaning cargo cache...${NC}"
    rm -rf ~/.cargo/registry/cache
    rm -rf ~/.cargo/registry/index
    cargo clean
    rm -rf .embuild
    echo "✓ Deep clean completed"
fi

# Check environment
echo -e "${YELLOW}Checking environment...${NC}"
if [ -z "$IDF_PATH" ]; then
    echo -e "${RED}Warning: IDF_PATH not set. Sourcing export-esp.sh...${NC}"
    source ~/export-esp.sh
fi

# Verify toolchain
if ! rustup show | grep -q "esp"; then
    echo -e "${RED}Error: ESP toolchain not found!${NC}"
    echo "Please run: cargo install espup --version 0.13.0 && espup install --targets esp32s3 --std"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Build environment fixed!${NC}"
echo ""
echo "Next steps:"
echo "  1. Make sure VS Code is closed"
echo "  2. Run: source ~/export-esp.sh"
echo "  3. Run: ./compile.sh"
echo ""
echo "For deep clean (clears all caches), run: $0 --deep-clean"