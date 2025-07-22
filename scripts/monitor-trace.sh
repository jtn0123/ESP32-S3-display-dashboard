#!/bin/bash

# Enhanced serial monitor with ST7789 command trace capture
PORT="/dev/cu.usbmodem101"
BAUD="115200"
LOG_FILE="lcd_trace_$(date +%Y%m%d_%H%M%S).log"

echo "Monitoring ESP LCD command trace on $PORT..."
echo "Logging to: $LOG_FILE"
echo "Press Ctrl+C to exit"
echo "==============================================="

# Configure serial port
stty -f $PORT $BAUD cs8 -cstopb -parenb

# Read from serial port and capture LCD commands
cat $PORT | tee "$LOG_FILE" | while IFS= read -r line; do
    # Highlight ST7789 command traces
    if echo "$line" | grep -qE "\[ST7789\]"; then
        echo -e "\033[1;36m$line\033[0m"  # Cyan for ST7789 commands
    elif echo "$line" | grep -qE "(Power pins|display|test|gap|viewport|init)"; then
        echo -e "\033[1;32m$line\033[0m"  # Green for important messages
    elif echo "$line" | grep -qE "(ERROR|error|fail|Failed)"; then
        echo -e "\033[1;31m$line\033[0m"  # Red for errors
    elif echo "$line" | grep -qE "(panel|I80|LCD)"; then
        echo -e "\033[1;33m$line\033[0m"  # Yellow for LCD messages
    else
        echo "$line"
    fi
done