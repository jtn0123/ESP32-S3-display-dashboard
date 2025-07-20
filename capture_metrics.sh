#!/bin/bash

echo "Capturing metrics from ESP32-S3..."
echo "Press Ctrl+C to stop"
echo "-----------------------------------"

# Use stty to configure serial port
stty -f /dev/cu.usbmodem101 115200 raw -echo

# Read from serial port and filter for metrics
cat /dev/cu.usbmodem101 | while IFS= read -r line; do
    # Check for performance metrics
    if [[ "$line" == *"[DISPLAY PERF]"* ]] || \
       [[ "$line" == *"[DISPLAY OPS]"* ]] || \
       [[ "$line" == *"[DISPLAY TIME]"* ]] || \
       [[ "$line" == *"[DISPLAY EFF]"* ]] || \
       [[ "$line" == *"[PERF]"* ]] || \
       [[ "$line" == *"[CORES]"* ]] || \
       [[ "$line" == *"ESP32-S3 Dashboard"* ]] || \
       [[ "$line" == *"Free heap"* ]]; then
        echo "$(date +%H:%M:%S) | $line"
    fi
done