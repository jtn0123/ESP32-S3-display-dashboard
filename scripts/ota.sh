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
    
    # Get device info before update
    local old_version=$(get_device_info "$ip")
    local old_uptime=$(curl -s "http://${ip}:${PORT}/api/system" 2>/dev/null | grep -o '"uptime_ms":[0-9]*' | cut -d':' -f2)
    
    # Check if it's an ELF file and convert to binary if needed
    if file "$firmware" | grep -q "ELF"; then
        print_color "$YELLOW" "🔄 Converting ELF to binary format..."
        local binary_firmware="${firmware}.bin"
        
        # Find esptool.py
        if [ -f ".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
            ESPTOOL=".embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
        elif [ -f "$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py" ]; then
            ESPTOOL="$HOME/.espressif/python_env/idf5.3_py3.13_env/bin/esptool.py"
        else
            print_color "$RED" "❌ esptool.py not found for ELF conversion"
            return 1
        fi
        
        # Convert ELF to binary
        $ESPTOOL --chip esp32s3 elf2image --flash_mode dio --flash_freq 40m --flash_size 16MB "$firmware" -o "$binary_firmware" >/dev/null 2>&1
        
        if [ ! -f "$binary_firmware" ]; then
            print_color "$RED" "❌ Failed to convert ELF to binary"
            return 1
        fi
        
        firmware="$binary_firmware"
        print_color "$GREEN" "✅ Binary conversion successful"
    fi
    
    local size=$(stat -f%z "$firmware" 2>/dev/null || stat -c%s "$firmware" 2>/dev/null)
    local size_mb=$(echo "scale=2; $size / 1024 / 1024" | bc)
    
    # Calculate SHA256
    print_color "$BLUE" "🔐 Calculating SHA256..."
    local sha256=$(shasum -a 256 "$firmware" | cut -d' ' -f1)
    
    print_color "$BLUE" "\n📡 OTA Update Process"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📍 Target device: $ip"
    echo "📦 Firmware size: ${size_mb} MB ($size bytes)"
    echo "🔐 SHA256: ${sha256:0:16}...${sha256: -16}"
    echo "🏷️  Current version: ${old_version:-unknown}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # Start upload with progress indicator
    print_color "$YELLOW" "⬆️  Uploading firmware..."
    
    # Show progress bar during upload
    (
        while true; do
            echo -n "."
            sleep 1
        done
    ) &
    PROGRESS_PID=$!
    
    # Upload with curl and capture response
    response=$(curl -X POST \
        -H "Content-Length: $size" \
        -H "X-OTA-Password: esp32" \
        -H "X-SHA256: $sha256" \
        --data-binary "@$firmware" \
        --connect-timeout 5 \
        --max-time 60 \
        -w "\n|||HTTP_CODE:%{http_code}|||TIME:%{time_total}|||" \
        -s \
        "http://${ip}:${PORT}/ota/update" 2>&1)
    
    # Stop progress indicator
    kill $PROGRESS_PID 2>/dev/null
    echo "" # New line after dots
    
    http_code=$(echo "$response" | grep -o "|||HTTP_CODE:[0-9]*|||" | sed 's/|||HTTP_CODE://g' | sed 's/|||//g')
    upload_time=$(echo "$response" | grep -o "|||TIME:[0-9.]*|||" | sed 's/|||TIME://g' | sed 's/|||//g')
    body=$(echo "$response" | sed 's/|||HTTP_CODE:[0-9]*|||//g' | sed 's/|||TIME:[0-9.]*|||//g')
    
    # Clean up temporary binary if we created one
    if [[ "$firmware" == *.bin ]] && [[ -f "${firmware%.bin}" ]]; then
        rm -f "$firmware"
    fi
    
    if [ "$http_code" = "200" ]; then
        print_color "$GREEN" "✅ Upload successful! (${upload_time}s)"
        print_color "$YELLOW" "\n🔄 Device is restarting..."
        
        # Wait for device to restart
        sleep 3
        
        # Check if device is back online
        print_color "$BLUE" "🔍 Verifying update..."
        local retries=0
        local max_retries=10
        local device_online=false
        
        while [ $retries -lt $max_retries ]; do
            if curl -s --connect-timeout 1 "http://${ip}:${PORT}/api/system" >/dev/null 2>&1; then
                device_online=true
                break
            fi
            ((retries++))
            echo -n "."
            sleep 2
        done
        echo ""
        
        if [ "$device_online" = true ]; then
            # Get new device info
            local new_version=$(get_device_info "$ip")
            local new_uptime=$(curl -s "http://${ip}:${PORT}/api/system" 2>/dev/null | grep -o '"uptime_ms":[0-9]*' | cut -d':' -f2)
            
            print_color "$GREEN" "\n✨ OTA Update Complete!"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "📍 Device: $ip"
            echo "🏷️  Version: ${new_version:-unknown}"
            if [ -n "$new_uptime" ] && [ "$new_uptime" -lt 60000 ]; then
                print_color "$GREEN" "✅ Device successfully restarted"
                echo "⏱️  Uptime: $(($new_uptime / 1000))s (fresh boot)"
            fi
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        else
            print_color "$YELLOW" "⚠️  Device is still restarting or may need manual check"
            echo "   Try accessing: http://${ip}/"
        fi
        
        return 0
    else
        print_color "$RED" "\n❌ Upload failed! (HTTP $http_code)"
        echo ""
        if [ "$http_code" = "503" ]; then
            print_color "$YELLOW" "📋 Diagnosis: OTA Not Available"
            echo "   • Device is running from factory partition"
            echo "   • Flash one more time via USB to enable OTA"
            echo "   • After that, wireless updates will work!"
        elif [ "$http_code" = "500" ]; then
            print_color "$YELLOW" "📋 Diagnosis: Server Error"
            echo "   • Error: $body"
            echo "   • Current firmware size: ${size_mb} MB"
            echo "   • Partition limit: 1.5 MB"
            if (( $(echo "$size_mb > 1.5" | bc -l) )); then
                print_color "$RED" "   ⚠️  Firmware exceeds partition size!"
            fi
        elif [ "$http_code" = "401" ]; then
            print_color "$YELLOW" "📋 Diagnosis: Unauthorized"
            echo "   • Invalid OTA password"
            echo "   • Check X-OTA-Password header in script"
            echo "   • Default password is: esp32"
        elif [ "$http_code" = "404" ]; then
            print_color "$YELLOW" "📋 Diagnosis: Endpoint Not Found"
            echo "   • OTA endpoint not available"
            echo "   • Update firmware via USB"
        elif [ "$http_code" = "" ]; then
            print_color "$YELLOW" "📋 Diagnosis: Connection Failed"
            echo "   • Could not connect to $ip"
            echo "   • Check if device is powered on"
            echo "   • Verify network connectivity"
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
        # Quick find - tries common IPs first, then mDNS, then limited scan
        print_color "$BLUE" "🔍 Quick scan for ESP32 devices..."
        
        # Try common device IPs first (very fast)
        for ip in "10.27.27.201" "192.168.1.201" "192.168.0.201" "10.0.0.201"; do
            if check_device "$ip"; then
                version=$(get_device_info "$ip")
                print_color "$GREEN" "✓ Found device at: $ip (v${version:-unknown})"
                echo "  To update: $0 $ip"
                exit 0
            fi
        done
        
        # Try mDNS discovery (3 second timeout)
        if command -v dns-sd >/dev/null 2>&1; then
            print_color "$YELLOW" "Checking mDNS (3 seconds)..."
            
            # Run dns-sd in background and kill after 3 seconds
            dns-sd -B _http._tcp > /tmp/mdns_scan.txt 2>&1 &
            MDNS_PID=$!
            sleep 3
            kill $MDNS_PID 2>/dev/null || true
            
            # Check for esp32 devices (matches esp32, esp32-dashboard, etc)
            MDNS_DEVICES=$(grep -E "esp32" /tmp/mdns_scan.txt 2>/dev/null | awk '{print $7}' || true)
            rm -f /tmp/mdns_scan.txt
            
            if [ -n "$MDNS_DEVICES" ]; then
                for device in $MDNS_DEVICES; do
                    # Resolve the hostname to IP
                    IP=$(timeout 1 dscacheutil -q host -a name "${device}.local" 2>/dev/null | grep "ip_address" | awk '{print $2}' | head -1 || true)
                    if [ -n "$IP" ]; then
                        version=$(get_device_info "$IP")
                        print_color "$GREEN" "✓ Found via mDNS: $device at $IP (v${version:-unknown})"
                        echo "  To update: $0 $IP"
                        exit 0
                    fi
                done
            fi
        fi
        
        # Limited subnet scan - only scan first 20 IPs and common DHCP ranges
        subnet=$(ifconfig | grep "inet " | grep -v 127.0.0.1 | head -1 | awk '{print $2}' | cut -d. -f1-3)
        print_color "$YELLOW" "Quick scan of common IPs on ${subnet}.0/24..."
        
        # Only scan .1-20 and .100-120 (common DHCP ranges)
        scan_count=0
        for i in {1..20} {100..120}; do
            if timeout 0.2 bash -c "echo > /dev/tcp/${subnet}.${i}/80" 2>/dev/null; then
                if check_device "${subnet}.${i}"; then
                    version=$(get_device_info "${subnet}.${i}")
                    print_color "$GREEN" "✓ Found device at: ${subnet}.${i} (v${version:-unknown})"
                    echo "  To update: $0 ${subnet}.${i}"
                    exit 0
                fi
            fi
            
            # Show progress dots
            ((scan_count++))
            if [ $((scan_count % 10)) -eq 0 ]; then
                echo -n "."
            fi
        done
        echo ""
        
        print_color "$RED" "❌ No ESP32 devices found in quick scan"
        echo "Try:"
        echo "  1. Specify IP directly: $0 192.168.1.x"
        echo "  2. Full network scan: $0 scan"
        echo "  3. Check device is powered on and connected to WiFi"
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
        echo "  $0 192.168.1.100         # Update specific device by IP"
        echo "  $0 esp32.local           # Update using mDNS hostname"
        echo "  $0 scan                  # List all devices"
        echo "  $0 auto                  # Update all devices"
        echo ""
        echo "Environment variables:"
        echo "  FIRMWARE  Path to firmware file (default: target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard)"
        echo "  PORT      Device HTTP port (default: 80)"
        ;;
    
    *)
        # Handle both IP addresses and mDNS hostnames
        TARGET="$1"
        
        # If it's a .local hostname, resolve it
        if [[ $TARGET == *.local ]]; then
            print_color "$BLUE" "🔍 Resolving $TARGET..."
            
            # Try to resolve using different methods
            IP=""
            
            # Method 1: dscacheutil (macOS)
            if command -v dscacheutil >/dev/null 2>&1; then
                IP=$(dscacheutil -q host -a name "$TARGET" 2>/dev/null | grep "ip_address" | awk '{print $2}' | head -1)
            fi
            
            # Method 2: getent (Linux)
            if [ -z "$IP" ] && command -v getent >/dev/null 2>&1; then
                IP=$(getent hosts "$TARGET" 2>/dev/null | awk '{print $1}' | head -1)
            fi
            
            # Method 3: ping (fallback)
            if [ -z "$IP" ]; then
                IP=$(ping -c 1 -t 1 "$TARGET" 2>/dev/null | grep "^PING" | sed -n 's/.*(\([0-9.]*\)).*/\1/p')
            fi
            
            if [ -n "$IP" ]; then
                print_color "$GREEN" "✓ Resolved to: $IP"
                TARGET="$IP"
            else
                print_color "$RED" "❌ Could not resolve $TARGET"
                echo "Make sure the device is online and mDNS is working"
                exit 1
            fi
        fi
        
        # Now upload to the target (IP address)
        if [[ $TARGET =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            if upload_firmware "$TARGET" "$FIRMWARE"; then
                print_color "$GREEN" "\n✨ OTA update completed successfully!"
            else
                print_color "$RED" "\n❌ OTA update failed!"
                exit 1
            fi
        else
            print_color "$RED" "Invalid target: $1"
            echo "Run '$0 help' for usage"
            exit 1
        fi
        ;;
esac