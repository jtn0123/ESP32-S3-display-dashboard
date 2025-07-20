#!/bin/bash

echo "Monitoring FPS and performance metrics from ESP32-S3..."
echo "Press Ctrl+C to stop"
echo "-----------------------------------"

# Configure serial port
stty -f /dev/cu.usbmodem101 115200 raw -echo 2>/dev/null

# Read from serial port and filter for performance metrics
cat /dev/cu.usbmodem101 | while IFS= read -r line; do
    # Only show lines with performance data
    if [[ "$line" == *"[PERF]"* ]] || [[ "$line" == *"[CORES]"* ]]; then
        echo "$(date +%H:%M:%S) | $line"
    fi
done