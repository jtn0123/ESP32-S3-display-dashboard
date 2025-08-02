#!/bin/bash
# Performance profiling script for ESP32-S3 Dashboard
# Monitors real-time metrics and generates performance report

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DURATION=${1:-60}  # Default 60 seconds
OUTPUT_FILE="performance-report-$(date +%Y%m%d-%H%M%S).txt"
METRICS_INTERVAL=1
ESP_IP="esp32.local"

echo -e "${GREEN}ESP32-S3 Performance Profiler${NC}"
echo "================================="
echo -e "Duration: ${YELLOW}${DURATION}${NC} seconds"
echo -e "Output: ${YELLOW}${OUTPUT_FILE}${NC}"
echo ""

# Check if device is reachable
echo -n "Checking device connectivity... "
if ping -c 1 -W 2 "$ESP_IP" >/dev/null 2>&1; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "Device not reachable at $ESP_IP"
    exit 1
fi

# Initialize report
cat > "$OUTPUT_FILE" << EOF
ESP32-S3 Dashboard Performance Report
Generated: $(date)
Duration: ${DURATION} seconds
=====================================

EOF

# Function to collect metrics
collect_metrics() {
    local timestamp=$(date +%s)
    
    # Fetch metrics from device
    local metrics=$(curl -s "http://$ESP_IP/api/metrics" 2>/dev/null || echo "{}")
    
    # Parse key metrics
    local fps=$(echo "$metrics" | jq -r '.fps_actual // 0')
    local cpu0=$(echo "$metrics" | jq -r '.cpu0_usage // 0')
    local cpu1=$(echo "$metrics" | jq -r '.cpu1_usage // 0')
    local heap=$(echo "$metrics" | jq -r '.heap_free // 0')
    local temp=$(echo "$metrics" | jq -r '.temperature // 0')
    local render_time=$(echo "$metrics" | jq -r '.render_time_ms // 0')
    local skip_rate=$(echo "$metrics" | jq -r '.skip_rate // 0')
    
    # Output to console
    printf "\r${BLUE}FPS:${NC} %.1f | ${BLUE}CPU:${NC} %d%%/%d%% | ${BLUE}Heap:${NC} %dk | ${BLUE}Temp:${NC} %.1f°C | ${BLUE}Render:${NC} %dms" \
        "$fps" "$cpu0" "$cpu1" "$((heap/1024))" "$temp" "$render_time"
    
    # Log to file
    echo "$timestamp,$fps,$cpu0,$cpu1,$heap,$temp,$render_time,$skip_rate" >> "$OUTPUT_FILE.csv"
}

# Create CSV header
echo "timestamp,fps,cpu0,cpu1,heap,temp,render_ms,skip_rate" > "$OUTPUT_FILE.csv"

# Start monitoring
echo -e "\n${YELLOW}Collecting performance data...${NC}"
echo "(Press Ctrl+C to stop early)"
echo ""

START_TIME=$(date +%s)
while true; do
    collect_metrics
    
    # Check if duration exceeded
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))
    if [ $ELAPSED -ge $DURATION ]; then
        break
    fi
    
    sleep $METRICS_INTERVAL
done

echo -e "\n\n${GREEN}Data collection complete!${NC}"

# Analyze results
echo -e "\n${YELLOW}Analyzing performance data...${NC}"

# Calculate statistics using awk
awk -F',' 'NR>1 {
    fps_sum += $2; fps_count++
    if ($2 > fps_max || NR==2) fps_max = $2
    if ($2 < fps_min || NR==2) fps_min = $2
    
    cpu0_sum += $3; cpu0_count++
    if ($3 > cpu0_max || NR==2) cpu0_max = $3
    
    cpu1_sum += $4; cpu1_count++
    if ($4 > cpu1_max || NR==2) cpu1_max = $4
    
    heap_sum += $5; heap_count++
    if ($5 < heap_min || NR==2) heap_min = $5
    
    temp_sum += $6; temp_count++
    if ($6 > temp_max || NR==2) temp_max = $6
    
    render_sum += $7; render_count++
    if ($7 > render_max || NR==2) render_max = $7
    
    skip_sum += $8; skip_count++
}
END {
    printf "\nPerformance Summary\n"
    printf "===================\n"
    printf "FPS:        Avg: %.1f, Min: %.1f, Max: %.1f\n", fps_sum/fps_count, fps_min, fps_max
    printf "CPU Core 0: Avg: %.1f%%, Max: %.1f%%\n", cpu0_sum/cpu0_count, cpu0_max
    printf "CPU Core 1: Avg: %.1f%%, Max: %.1f%%\n", cpu1_sum/cpu1_count, cpu1_max
    printf "Free Heap:  Avg: %.1f KB, Min: %.1f KB\n", heap_sum/heap_count/1024, heap_min/1024
    printf "Temperature: Avg: %.1f°C, Max: %.1f°C\n", temp_sum/temp_count, temp_max
    printf "Render Time: Avg: %.1f ms, Max: %.1f ms\n", render_sum/render_count, render_max
    printf "Skip Rate:   Avg: %.1f%%\n", skip_sum/skip_count
}' "$OUTPUT_FILE.csv" | tee -a "$OUTPUT_FILE"

# Performance analysis
cat >> "$OUTPUT_FILE" << EOF

Performance Analysis
====================
EOF

# Check for issues
if awk -F',' 'NR>1 && $2 < 30 {found=1} END {exit !found}' "$OUTPUT_FILE.csv"; then
    echo "⚠️  WARNING: FPS dropped below 30 during test" | tee -a "$OUTPUT_FILE"
fi

if awk -F',' 'NR>1 && ($3 > 80 || $4 > 80) {found=1} END {exit !found}' "$OUTPUT_FILE.csv"; then
    echo "⚠️  WARNING: CPU usage exceeded 80% during test" | tee -a "$OUTPUT_FILE"
fi

if awk -F',' 'NR>1 && $5 < 51200 {found=1} END {exit !found}' "$OUTPUT_FILE.csv"; then
    echo "⚠️  WARNING: Free heap dropped below 50KB during test" | tee -a "$OUTPUT_FILE"
fi

if awk -F',' 'NR>1 && $6 > 70 {found=1} END {exit !found}' "$OUTPUT_FILE.csv"; then
    echo "⚠️  WARNING: Temperature exceeded 70°C during test" | tee -a "$OUTPUT_FILE"
fi

# Generate recommendations
cat >> "$OUTPUT_FILE" << EOF

Recommendations
===============
EOF

# FPS recommendations
AVG_FPS=$(awk -F',' 'NR>1 {sum+=$2; count++} END {print sum/count}' "$OUTPUT_FILE.csv")
if (( $(echo "$AVG_FPS < 50" | bc -l) )); then
    echo "- Consider reducing display update frequency" | tee -a "$OUTPUT_FILE"
    echo "- Check for blocking operations in render loop" | tee -a "$OUTPUT_FILE"
fi

# Memory recommendations
MIN_HEAP=$(awk -F',' 'NR>1 {if ($5 < min || NR==2) min=$5} END {print min}' "$OUTPUT_FILE.csv")
if [ "$MIN_HEAP" -lt 102400 ]; then
    echo "- Memory usage is high, consider optimizing allocations" | tee -a "$OUTPUT_FILE"
    echo "- Review sensor history buffer sizes" | tee -a "$OUTPUT_FILE"
fi

echo -e "\n${GREEN}Performance report saved to: ${OUTPUT_FILE}${NC}"
echo -e "${GREEN}Raw data saved to: ${OUTPUT_FILE}.csv${NC}"

# Optional: Plot graph if gnuplot is available
if command -v gnuplot &> /dev/null; then
    echo -e "\n${YELLOW}Generating performance graph...${NC}"
    
    gnuplot << EOF
set terminal png size 1200,800
set output "performance-graph-$(date +%Y%m%d-%H%M%S).png"
set title "ESP32-S3 Dashboard Performance"
set xlabel "Time (seconds)"
set ylabel "Value"
set grid
set key left top

plot "$OUTPUT_FILE.csv" using 0:2 with lines title "FPS" lw 2, \
     "$OUTPUT_FILE.csv" using 0:3 with lines title "CPU0 %" lw 2, \
     "$OUTPUT_FILE.csv" using 0:4 with lines title "CPU1 %" lw 2, \
     "$OUTPUT_FILE.csv" using 0:(\$5/1024) with lines title "Heap KB" lw 2, \
     "$OUTPUT_FILE.csv" using 0:6 with lines title "Temp °C" lw 2
EOF
    
    echo -e "${GREEN}Performance graph generated!${NC}"
fi