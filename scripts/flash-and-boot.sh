#!/usr/bin/env bash
# Flash script that ensures device boots properly

set -euo pipefail

PORT="${PORT:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "ESP32-S3 Flash & Boot"
echo "===================="

# Use espflash which handles ESP32-S3 USB-JTAG better
echo "Flashing with espflash..."
espflash flash \
    --port "$PORT" \
    --chip esp32s3 \
    --flash-size 16mb \
    --flash-freq 40mhz \
    --flash-mode dio \
    target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard

echo ""
echo "Flash complete. The device should boot automatically."
echo ""

# Option to monitor
if [[ "${1:-}" == "--monitor" ]]; then
    echo "Starting monitor..."
    echo "If screen is black, press RESET button on device"
    echo "Press Ctrl+] to exit"
    exec espflash monitor --port "$PORT"
fi