#!/bin/bash
# Baseline Performance Measurement Script
# Records current GPIO implementation performance metrics

echo "ESP32-S3 Display Dashboard - Baseline Performance Test"
echo "====================================================="
echo "Date: $(date)"
echo "Branch: $(git branch --show-current)"
echo "Version: $(grep DISPLAY_VERSION src/version.rs | cut -d'"' -f2)"
echo ""

# Output file for baseline metrics
OUTPUT_FILE="target/baseline-metrics.txt"

# Build in release mode
echo "Building release binary..."
./compile.sh --release || exit 1

echo ""
echo "Flash the device and monitor serial output for 30 seconds"
echo "to capture performance metrics."
echo ""
echo "Press Enter when device is ready..."
read

# Create output file with header
cat > "$OUTPUT_FILE" << EOF
ESP32-S3 Display Dashboard - Baseline Performance Metrics
========================================================
Date: $(date)
Branch: $(git branch --show-current)
Version: $(grep DISPLAY_VERSION src/version.rs | cut -d'"' -f2)
Build: Release Mode

Performance Metrics (30 second sample):
EOF

echo ""
echo "Monitoring performance for 30 seconds..."
echo "Look for [PERF] lines in serial output"
echo ""

# Monitor for 30 seconds and capture PERF lines
timeout 30 espflash monitor | grep "\[PERF\]" | tail -10 >> "$OUTPUT_FILE"

echo ""
echo "Baseline metrics saved to: $OUTPUT_FILE"
echo ""
echo "Summary:"
tail -5 "$OUTPUT_FILE"