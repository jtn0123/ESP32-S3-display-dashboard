#!/bin/bash

# Simple serial monitor for ESP32
PORT=${1:-/dev/cu.usbmodem101}
BAUD=${2:-115200}

echo "Monitoring $PORT at $BAUD baud..."
echo "Press Ctrl+A then K to exit"

# Use cat to read from serial port
stty -f $PORT $BAUD cs8 -parenb -cstopb
cat $PORT