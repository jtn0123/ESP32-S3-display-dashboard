#!/usr/bin/env bash
# Simple serial monitor using screen or picocom

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "Monitoring on port: $PORT at 115200 baud"
echo "Exit with: Ctrl+A then K (screen) or Ctrl+A Ctrl+X (picocom)"
echo ""

# Try screen first (usually installed on macOS)
if command -v screen >/dev/null 2>&1; then
    exec screen "$PORT" 115200
elif command -v picocom >/dev/null 2>&1; then
    exec picocom -b 115200 "$PORT"
else
    echo "Neither screen nor picocom found. Using cat (no input, Ctrl+C to exit):"
    stty -f "$PORT" 115200 cs8 -cstopb -parenb
    exec cat "$PORT"
fi