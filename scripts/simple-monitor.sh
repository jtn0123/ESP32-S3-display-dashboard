#\!/bin/bash

# Simple serial monitor that captures everything
PORT="/dev/cu.usbmodem101"
BAUD="115200"

echo "Simple serial monitor on $PORT..."
echo "==============================================="

# Configure serial port
stty -f $PORT $BAUD cs8 -cstopb -parenb

# Just cat the output
cat $PORT
EOF < /dev/null