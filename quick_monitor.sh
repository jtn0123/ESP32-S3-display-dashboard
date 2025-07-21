#!/bin/bash
echo "Monitoring ESP32-S3 - Press RESET button to see boot output"
echo "Press Ctrl+C to exit"
echo "----------------------------------------"

# Simple serial monitor
stty -F /dev/cu.usbmodem101 115200 raw -echo 2>/dev/null || true
cat /dev/cu.usbmodem101