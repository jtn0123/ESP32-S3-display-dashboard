#!/bin/bash
# Simple network monitor for ESP32 logs

IP="${1:-10.27.27.201}"
PORT="${2:-23}"

echo "Connecting to ESP32 at $IP:$PORT"
echo "Press Ctrl+C to exit"
echo "========================================="

# Use nc (netcat) which is built into macOS
nc $IP $PORT