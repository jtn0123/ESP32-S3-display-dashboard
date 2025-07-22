#!/bin/bash

# Simple serial monitor for ESP LCD test output
PORT="/dev/cu.usbmodem101"
BAUD="115200"

echo "Monitoring ESP LCD test output on $PORT..."
echo "Press Ctrl+C to exit"
echo "==============================================="

# Configure serial port
stty -f $PORT $BAUD cs8 -cstopb -parenb

# Read from serial port and highlight LCD-related messages
cat $PORT | while IFS= read -r line; do
    if echo "$line" | grep -qE "(ESP_LCD|LCD|display|power|backlight|init|Power pins|gap|viewport|ST7789|panel|I80|draw|color|test)"; then
        echo -e "\033[1;32m$line\033[0m"  # Green for LCD messages
    elif echo "$line" | grep -qE "(ERROR|error|fail|Failed)"; then
        echo -e "\033[1;31m$line\033[0m"  # Red for errors
    else
        echo "$line"
    fi
done