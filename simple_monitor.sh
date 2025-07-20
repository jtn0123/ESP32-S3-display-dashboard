#!/bin/bash
# Simple serial monitor using cat

PORT="/dev/cu.usbmodem101"
BAUD="115200"

echo "Monitoring ESP32 on $PORT at $BAUD baud..."
echo "Press Ctrl+C to exit"
echo "----------------------------------------"

# Use stty to configure the port, then cat to read
stty -f $PORT $BAUD cs8 -cstopb -parenb
cat $PORT