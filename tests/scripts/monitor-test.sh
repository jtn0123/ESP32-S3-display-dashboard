#!/usr/bin/env bash
# Monitor script that works in non-interactive mode

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "Monitoring on port: $PORT at 115200 baud"
echo "Reading for 5 seconds..."
echo "----------------------------------------"

# Set serial parameters
stty -f "$PORT" 115200 cs8 -cstopb -parenb 2>/dev/null || true

# Read for 5 seconds
timeout 5 cat "$PORT" 2>&1 || echo -e "\n----------------------------------------\nMonitor stopped"