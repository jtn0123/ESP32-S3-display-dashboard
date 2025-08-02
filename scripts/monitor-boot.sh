#!/usr/bin/env bash
# Monitor script that resets device and captures boot sequence

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"
ESPTOOL=".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "Resetting device and monitoring boot sequence..."
echo "Port: $PORT"
echo "========================================"

# Reset the device
$ESPTOOL --chip esp32s3 --port "$PORT" --no-stub run >/dev/null 2>&1

# Give it a moment to start booting
sleep 0.5

# Monitor with timestamps
stty -f "$PORT" 115200 cs8 -cstopb -parenb 2>/dev/null || true
cat "$PORT" | while IFS= read -r line; do
    echo "$(date '+%H:%M:%S') | $line"
done