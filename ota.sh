#!/bin/bash
# ESP32-S3 Dashboard OTA Update Script
# Simple OTA update using curl (no Python dependencies needed)

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
FIRMWARE="${FIRMWARE:-target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard}"
PORT="${PORT:-80}"  # OTA endpoint is on main server port 80

# Function to print colored output
print_color() {
    local color=$1
    shift
    echo -e "${color}$@${NC}"
}

# Function to check if device is reachable and is an ESP32
check_device() {
    local ip=$1
    local response=$(curl -s --connect-timeout 1 "http://${ip}:${PORT}/api/system" 2>/dev/null)
    # Check if response contains ESP32 identifiers
    echo "$response" | grep -q '"version"' && echo "$response" | grep -q '"free_heap"'
}

# Function to get device info
get_device_info() {
    local ip=$1
    curl -s "http://${ip}:${PORT}/api/system" 2>/dev/null | grep -o '"version":"[^"]*"' | cut -d'"' -f4
}

# Function to upload firmware
upload_firmware() {
    local ip=$1
    local firmware=$2
    
    if [ ! -f "$firmware" ]; then
        print_color "$RED" "❌ Firmware not found: $firmware"
        print_color "$YELLOW" "   Run ./compile.sh --release to build firmware"
        return 1
    fi
    
    local size=$(stat -f%z "$firmware" 2>/dev/null || stat -c%s "$firmware" 2>/dev/null)
    local size_mb=$(echo "scale=2; $size / 1024 / 1024" | bc)
    
    print_color "$BLUE" "📤 Updating device at $ip"
    echo "   Firmware: $size bytes (${size_mb} MB)"
    echo -n "   Uploading..."
    
    # Upload with curl and capture response
    response=$(curl -X POST \
        -H "Content-Length: $size" \
        --data-binary "@$firmware" \
        --connect-timeout 5 \
        --max-time 60 \
        -w "\n|||HTTP_CODE:%{http_code}|||" \
        -s \
        "http://${ip}:${PORT}/ota/update" 2>&1)
    
    http_code=$(echo "$response" | grep -o "|||HTTP_CODE:[0-9]*|||" | sed 's/|||HTTP_CODE://g' | sed 's/|||//g')
    body=$(echo "$response" | sed 's/|||HTTP_CODE:[0-9]*|||//g')
    
    if [ "$http_code" = "200" ]; then
        print_color "$GREEN" "\r   ✅ Upload successful! Device will restart."
        return 0
    else
        print_color "$RED" "\r   ❌ Upload failed! (HTTP $http_code)"
        if [ "$http_code" = "503" ]; then
            echo "   Device is running from factory partition."
            echo "   Flash one more time via USB to enable OTA updates."
            echo "   After that, wireless updates will work!"
        elif [ "$http_code" = "500" ]; then
            echo "   Server error: $body"
            echo "   This often means the firmware is too large for the partition."
            echo "   Current firmware size: ${size_mb} MB"
        elif [ "$http_code" = "404" ]; then
            echo "   OTA endpoint not found. Update firmware via USB."
        elif [ "$http_code" = "" ]; then
            echo "   Connection failed. Check if device is online."
        fi
        return 1
    fi
}

# Function to scan network
scan_network() {
    local subnet=${1:-$(ifconfig | grep "inet " | grep -v 127.0.0.1 | head -1 | awk '{print $2}' | cut -d. -f1-3)}
    
    print_color "$BLUE" "🔍 Scanning for ESP32 devices on ${subnet}.0/24..."
    echo "   This may take a moment..."
    
    # Create temp file for results
    tmpfile=$(mktemp)
    
    # Scan in parallel
    for i in {1..254}; do
        (
            if check_device "${subnet}.${i}"; then
                local version=$(get_device_info "${subnet}.${i}")
                echo "${subnet}.${i}|${version}" >> "$tmpfile"
                print_color "$GREEN" "  ✓ Found ESP32: ${subnet}.${i} (v${version:-unknown})"
            fi
        ) &
        
        # Limit concurrent jobs (macOS compatible)
        while (( $(jobs -r | wc -l) >= 20 )); do
            sleep 0.1
        done
    done
    
    # Wait for all jobs
    wait
    
    # Count results
    local found=0
    if [ -f "$tmpfile" ]; then
        found=$(wc -l < "$tmpfile")
        rm -f "$tmpfile"
    fi
    
    if [ $found -eq 0 ]; then
        print_color "$RED" "❌ No ESP32 devices found"
        echo ""
        echo "Troubleshooting:"
        echo "  1. Ensure your ESP32 is powered on and connected to WiFi"
        echo "  2. Check that your computer is on the same network" 
        echo "  3. Your device IP from earlier was 10.27.27.201"
        echo "  4. Try: ./ota.sh 10.27.27.201"
    else
        print_color "$BLUE" "\n📱 Found $found ESP32 device(s)"
    fi
}

# Main script
print_color "$BLUE" "ESP32-S3 Dashboard OTA Tool"
echo "==========================="

case "${1:-help}" in
    find)
        # Quick find - tries common IPs first
        print_color "$BLUE" "🔍 Quick scan for ESP32 devices..."
        
        # Try common device IPs first
        for ip in "10.27.27.201" "192.168.1.201" "192.168.0.201" "10.0.0.201"; do
            if check_device "$ip"; then
                version=$(get_device_info "$ip")
                print_color "$GREEN" "✓ Found device at: $ip (v${version:-unknown})"
                echo "  To update: $0 $ip"
                exit 0
            fi
        done
        
        # If not found, scan current subnet
        subnet=$(ifconfig | grep "inet " | grep -v 127.0.0.1 | head -1 | awk '{print $2}' | cut -d. -f1-3)
        print_color "$YELLOW" "Scanning subnet ${subnet}.0/24..."
        
        for i in {1..254}; do
            if check_device "${subnet}.${i}"; then
                version=$(get_device_info "${subnet}.${i}")
                print_color "$GREEN" "✓ Found device at: ${subnet}.${i} (v${version:-unknown})"
                echo "  To update: $0 ${subnet}.${i}"
                exit 0
            fi
        done
        
        print_color "$RED" "❌ No ESP32 devices found"
        echo "Make sure your device is powered on and connected to the same network"
        ;;
        
    scan)
        scan_network "$2"
        ;;
    
    auto)
        # Auto-discover and update
        print_color "$BLUE" "🔍 Finding ESP32 devices..."
        
        devices=()
        
        # First, try common known IPs for fast discovery
        for ip in "10.27.27.201" "192.168.1.201" "192.168.0.201" "10.0.0.201"; do
            if check_device "$ip"; then
                devices+=("$ip")
                print_color "$GREEN" "  ✓ Found ESP32: $ip"
            fi
        done
        
        # If no devices found, do subnet scan
        if [ ${#devices[@]} -eq 0 ]; then
            subnet=${2:-$(ifconfig | grep "inet " | grep -v 127.0.0.1 | head -1 | awk '{print $2}' | cut -d. -f1-3)}
            print_color "$YELLOW" "  Scanning subnet ${subnet}.0/24..."
            
            # Create temp file for results
            tmpfile=$(mktemp)
            
            # Scan in parallel using background jobs
            for i in {1..254}; do
                (
                    if check_device "${subnet}.${i}"; then
                        echo "${subnet}.${i}" >> "$tmpfile"
                        print_color "$GREEN" "  ✓ Found ESP32: ${subnet}.${i}"
                    fi
                ) &
                
                # Limit concurrent jobs (macOS compatible)
                while (( $(jobs -r | wc -l) >= 20 )); do
                    sleep 0.1
                done
            done
            
            # Wait for all background jobs
            wait
            
            # Read found devices
            if [ -f "$tmpfile" ]; then
                while IFS= read -r device; do
                    devices+=("$device")
                done < "$tmpfile"
                rm -f "$tmpfile"
            fi
        fi
        
        if [ ${#devices[@]} -eq 0 ]; then
            print_color "$RED" "\n❌ No devices found"
            exit 1
        fi
        
        print_color "$YELLOW" "\nUpdate all ${#devices[@]} device(s)? (y/N): "
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            echo "Update cancelled"
            exit 0
        fi
        
        success=0
        for device in "${devices[@]}"; do
            if upload_firmware "$device" "$FIRMWARE"; then
                ((success++))
            fi
        done
        
        print_color "$GREEN" "\n✨ Update complete: $success/${#devices[@]} successful"
        ;;
    
    help|--help|-h)
        echo "Usage: $0 [COMMAND] [OPTIONS]"
        echo ""
        echo "Commands:"
        echo "  <IP>          Update specific device"
        echo "  find          Quick find first ESP32 device"
        echo "  scan [subnet] Scan network for all devices"  
        echo "  auto [subnet] Auto-discover and update all devices"
        echo "  help          Show this help"
        echo ""
        echo "Examples:"
        echo "  $0 find                   # Find first device"
        echo "  $0 192.168.1.100         # Update specific device"
        echo "  $0 scan                  # List all devices"
        echo "  $0 auto                  # Update all devices"
        echo ""
        echo "Environment variables:"
        echo "  FIRMWARE  Path to firmware file (default: target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard)"
        echo "  PORT      Device HTTP port (default: 80)"
        ;;
    
    *)
        # Assume it's an IP address
        if [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            if upload_firmware "$1" "$FIRMWARE"; then
                print_color "$GREEN" "\n✨ OTA update completed successfully!"
            else
                print_color "$RED" "\n❌ OTA update failed!"
                exit 1
            fi
        else
            print_color "$RED" "Invalid command or IP address: $1"
            echo "Run '$0 help' for usage"
            exit 1
        fi
        ;;
esac