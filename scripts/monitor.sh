#!/usr/bin/env bash
# Monitor script for ESP32-S3

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "Monitoring ESP32-S3 on port: $PORT"
echo "Press Ctrl+C to exit"
echo "----------------------------------------"

# Trap Ctrl+C to ensure clean exit
trap 'echo -e "\nMonitor stopped"; exit 0' INT

# Simple direct read from serial port
# Set baud rate and read continuously
stty -f "$PORT" 115200 cs8 -cstopb -parenb 2>/dev/null || true
cat "$PORT"