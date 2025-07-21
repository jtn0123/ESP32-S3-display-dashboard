#!/usr/bin/env bash
# Capture serial output after reset

PORT="${1:-$(ls /dev/cu.usbmodem* /dev/tty.usbmodem* 2>/dev/null | head -1)}"

if [ -z "$PORT" ]; then
    echo "No device found"
    exit 1
fi

echo "ESP32-S3 Serial Capture"
echo "======================="
echo "Port: $PORT"
echo ""
echo ">>> PRESS RESET BUTTON ON ESP32-S3 NOW <<<"
echo ""
echo "Waiting for boot output..."
echo "----------------------------------------"

# Set serial parameters
stty -f "$PORT" 115200 cs8 -cstopb -parenb 2>/dev/null || true

# Read for 20 seconds to capture full boot sequence
timeout 20 cat "$PORT" 2>&1 | while IFS= read -r line; do
    echo "$line"
    
    # Check for key indicators
    if [[ "$line" == *"ESP-IDF v5.3"* ]]; then
        echo "[DETECTED: v5.3 Bootloader]"
    fi
    if [[ "$line" == *"multiple DROM"* ]]; then
        echo "[ERROR: Multiple DROM segments!]"
    fi
    if [[ "$line" == *"v5.52-finalFix"* ]]; then
        echo "[DETECTED: App version v5.52-finalFix]"
    fi
    if [[ "$line" == *"ESP_LCD Display Manager"* ]]; then
        echo "[DETECTED: ESP_LCD initialization]"
    fi
    if [[ "$line" == *"FPS:"* ]]; then
        echo "[SUCCESS: Performance metrics detected]"
    fi
done

echo "----------------------------------------"
echo "Capture complete"