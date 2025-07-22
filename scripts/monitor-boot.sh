#!/bin/bash

# Monitor from boot with enhanced filtering
PORT="/dev/cu.usbmodem101"
BAUD="115200"
LOG_FILE="boot_debug_$(date +%Y%m%d_%H%M%S).log"

echo "Monitoring ESP LCD boot sequence on $PORT..."
echo "Logging to: $LOG_FILE"
echo "==============================================="

# Configure serial port
stty -f $PORT $BAUD cs8 -cstopb -parenb

# Monitor and filter for our debug output
cat $PORT | tee "$LOG_FILE" | while IFS= read -r line; do
    # Highlight our debug messages
    if echo "$line" | grep -qE "(=== ESP LCD|=== GPIO|=== ST7789|=== COMPREHENSIVE|✓|✗|ERROR|Failed)"; then
        echo -e "\033[1;35m$line\033[0m"  # Magenta for key messages
    elif echo "$line" | grep -qE "\[ST7789\]"; then
        echo -e "\033[1;36m$line\033[0m"  # Cyan for ST7789 commands
    elif echo "$line" | grep -qE "(Power pins|GPIO[0-9]+|HIGH|LOW)"; then
        echo -e "\033[1;32m$line\033[0m"  # Green for GPIO states
    elif echo "$line" | grep -qE "(Test [0-9]+:|Result:)"; then
        echo -e "\033[1;33m$line\033[0m"  # Yellow for test results
    elif echo "$line" | grep -qE "(Time elapsed:|panel|init|gap)"; then
        echo -e "\033[1;34m$line\033[0m"  # Blue for timing/init
    else
        echo "$line"
    fi
done