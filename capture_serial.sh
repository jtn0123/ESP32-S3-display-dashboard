#\!/bin/bash
# Simple serial capture script
stty -f /dev/cu.usbmodem101 115200 cs8 -cstopb -parenb
cat /dev/cu.usbmodem101 | tee lcd_enhanced_debug.log
