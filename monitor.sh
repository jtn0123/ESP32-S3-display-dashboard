#\!/bin/bash
# Simple monitor script for ESP32-S3

PORT="${1:-/dev/cu.usbmodem101}"
echo "Monitoring $PORT at 115200 baud..."
echo "Press Ctrl+C to exit"
echo "================================"

# Use cat to read from serial port
stty -f $PORT 115200 raw -echo
cat $PORT
