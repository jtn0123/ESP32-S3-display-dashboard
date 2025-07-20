#!/bin/bash
# ESP32-S3 Dashboard - Cleanup Script
# This script removes junk files and old artifacts

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}ESP32-S3 Dashboard - Cleanup Script${NC}"
echo "==================================="
echo ""

# Dry run mode by default
DRY_RUN=true
if [ "$1" == "--force" ]; then
    DRY_RUN=false
    echo -e "${YELLOW}Running in FORCE mode - files will be deleted!${NC}"
else
    echo -e "${GREEN}Running in DRY RUN mode - no files will be deleted${NC}"
    echo -e "Use '$0 --force' to actually delete files"
fi
echo ""

# Function to remove files
remove_file() {
    if [ "$DRY_RUN" = true ]; then
        echo "  Would remove: $1"
    else
        rm -f "$1" 2>/dev/null && echo "  ✓ Removed: $1" || echo "  ✗ Failed to remove: $1"
    fi
}

# Function to remove directories
remove_dir() {
    if [ "$DRY_RUN" = true ]; then
        echo "  Would remove: $1/"
    else
        rm -rf "$1" 2>/dev/null && echo "  ✓ Removed: $1/" || echo "  ✗ Failed to remove: $1/"
    fi
}

# 1. Clean up log files
echo -e "${YELLOW}Cleaning log files...${NC}"
remove_file "build_clean.log"
remove_file "build.log"
remove_file "serial_output.log"
remove_file "lcd_cam_output.log"
remove_file "build_test.log"
remove_file "full_build.log"
remove_file "fast_build.log"
remove_file "build_debug.log"
remove_file "quick_build.log"
remove_file "build_output.log"
remove_file "test_build.log"
remove_file "boot_log.txt"

# 2. Clean up temporary/test files
echo -e "\n${YELLOW}Cleaning temporary files...${NC}"
remove_file "compile_output.txt"
remove_file "test.rs"
remove_file "espup"

# 3. Clean up test directories
echo -e "\n${YELLOW}Cleaning test directories...${NC}"
remove_dir "test-minimal"
remove_dir "test-rust"

# 4. Clean up redundant monitor scripts (keeping only scripts/ versions)
echo -e "\n${YELLOW}Cleaning redundant monitor scripts...${NC}"
echo -e "${BLUE}Note: Keeping monitor scripts in scripts/ directory${NC}"
remove_file "monitor.sh"
remove_file "monitor.py"
remove_file "monitor_fps.sh"
remove_file "monitor_metrics.py"
remove_file "serial_monitor.sh"
remove_file "simple_monitor.sh"
remove_file "read_serial.py"
remove_file "capture_metrics.sh"

# 5. Clean Python cache
echo -e "\n${YELLOW}Cleaning Python cache...${NC}"
remove_dir "scripts/__pycache__"
remove_dir ".mypy_cache"

# 6. Optional: Clean OTA tool build (ask user)
if [ -d "tools/ota/target" ]; then
    echo -e "\n${YELLOW}Found OTA tool build artifacts (54MB)${NC}"
    if [ "$DRY_RUN" = false ]; then
        read -p "Remove tools/ota/target/? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            remove_dir "tools/ota/target"
        fi
    else
        echo "  Would prompt to remove: tools/ota/target/"
    fi
fi

# 7. Clean up lock files
echo -e "\n${YELLOW}Cleaning lock files...${NC}"
remove_file "components_esp32s3.lock"

# 8. Clean up downloaded archives
echo -e "\n${YELLOW}Cleaning downloaded archives...${NC}"
remove_file "~/rust-1.87.0.0-aarch64-apple-darwin.tar.xz"

# Summary
echo -e "\n${GREEN}Cleanup complete!${NC}"
if [ "$DRY_RUN" = true ]; then
    echo -e "\n${YELLOW}This was a DRY RUN. To actually delete files, run:${NC}"
    echo -e "  $0 --force"
fi

# Show remaining space
echo -e "\n${BLUE}Current directory size:${NC}"
du -sh .

echo -e "\n${BLUE}Build artifacts (not cleaned):${NC}"
du -sh target 2>/dev/null || echo "  target/ - Not found"
du -sh .embuild 2>/dev/null || echo "  .embuild/ - Not found"