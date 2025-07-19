#\!/bin/bash
stty -f /dev/cu.usbmodem101 115200 cs8 -cstopb -parenb raw
exec 3</dev/cu.usbmodem101
(cat <&3 | while IFS= read -r line || [ -n "$line" ]; do
    echo "$line"
    if [[ "$line" == *"LCD_CAM"* ]] || [[ "$line" == *"Register"* ]]; then
        echo ">>> $line" >&2
    fi
done) &
pid=$\!
sleep 15
kill $pid 2>/dev/null
exec 3<&-
