#!/bin/bash

# Test script for ESP_LCD driver migration
# This script helps test and compare both display driver implementations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "ESP32-S3 Display Dashboard - ESP_LCD Test Script"
echo "=============================================="
echo

# Function to test minimal esp_lcd
test_minimal() {
    echo "1. Testing minimal ESP_LCD implementation..."
    echo "   This will show a red screen if successful"
    echo
    
    LCD_CAM_TEST=1 ./compile.sh
    if [ $? -eq 0 ]; then
        echo "✓ Compilation successful"
        echo
        echo "Flashing device..."
        LCD_CAM_TEST=1 ./scripts/flash.sh
    else
        echo "✗ Compilation failed"
        exit 1
    fi
}

# Function to test with GPIO driver
test_gpio_driver() {
    echo "2. Building with GPIO bit-bang driver (default)..."
    echo
    
    ./compile.sh --clean
    if [ $? -eq 0 ]; then
        echo "✓ GPIO driver compilation successful"
        echo "Size: $(du -h target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard | cut -f1)"
    else
        echo "✗ GPIO driver compilation failed"
        exit 1
    fi
}

# Function to test with ESP_LCD driver
test_esp_lcd_driver() {
    echo "3. Building with ESP_LCD DMA driver..."
    echo
    
    # Note: We need to modify compile.sh to pass features, or use cargo directly
    export CARGO_BUILD_TARGET="xtensa-esp32s3-espidf"
    export CARGO_BUILD_TARGET_DIR="target"
    
    cargo build --release --features esp_lcd_driver
    if [ $? -eq 0 ]; then
        echo "✓ ESP_LCD driver compilation successful"
        echo "Size: $(du -h target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard | cut -f1)"
    else
        echo "✗ ESP_LCD driver compilation failed"
        exit 1
    fi
}

# Main menu
echo "Select test option:"
echo "1) Test minimal ESP_LCD (red screen test)"
echo "2) Build with GPIO driver (default)"
echo "3) Build with ESP_LCD driver"
echo "4) Run all tests"
echo

read -p "Enter choice (1-4): " choice

case $choice in
    1)
        test_minimal
        ;;
    2)
        test_gpio_driver
        ;;
    3)
        test_esp_lcd_driver
        ;;
    4)
        echo "Running all tests..."
        echo
        test_minimal
        echo
        echo "Press any key to continue with driver tests..."
        read -n 1
        echo
        test_gpio_driver
        echo
        test_esp_lcd_driver
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

echo
echo "Test complete!"