#!/usr/bin/env bash
# Simple working monitor using cu

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "Monitoring ESP32-S3 on port: $PORT"
echo "To exit: Type ~. (tilde followed by period)"
echo "----------------------------------------"

# Use cu which works well with ESP32
exec cu -l "$PORT" -s 115200