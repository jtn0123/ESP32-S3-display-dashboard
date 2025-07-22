#\!/bin/bash
echo "Starting serial monitor on /dev/cu.usbmodem101..."
stty -f /dev/cu.usbmodem101 115200 cs8 -cstopb -parenb raw
exec 3</dev/cu.usbmodem101
cat <&3
