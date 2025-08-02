#!/bin/bash
# ESP32-S3 Dashboard Development Aliases
# Source this file to get helpful aliases for development
# Usage: source scripts/esp32-aliases.sh

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Alias for espflash that includes flash size
alias espflash-s3="$SCRIPT_DIR/espflash-wrapper.sh"

# Quick development commands
alias esp32-build="cd '$PROJECT_ROOT' && ./compile.sh"
alias esp32-flash="cd '$PROJECT_ROOT' && ./scripts/flash.sh"
alias esp32-monitor="espflash monitor"
alias esp32-quick="cd '$PROJECT_ROOT' && ./scripts/quick-flash.sh"
alias esp32-ota="cd '$PROJECT_ROOT' && ./scripts/ota.sh"
alias esp32-telnet="cd '$PROJECT_ROOT' && ./scripts/monitor-telnet.py"
alias esp32-control="cd '$PROJECT_ROOT' && ./scripts/telnet-control.py"

# Show available commands
echo "ESP32-S3 Dashboard aliases loaded!"
echo ""
echo "Available commands:"
echo "  espflash-s3     - espflash with automatic --flash-size 16mb"
echo "  esp32-build     - Build the project"
echo "  esp32-flash     - Flash via USB"
echo "  esp32-monitor   - Monitor serial output"
echo "  esp32-quick     - Build, flash, and monitor"
echo "  esp32-ota       - OTA update tool"
echo "  esp32-telnet    - Telnet log monitor"
echo "  esp32-control   - Telnet control with commands"
echo ""
echo "Example: espflash-s3 flash --port /dev/cu.usbmodem101 target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"