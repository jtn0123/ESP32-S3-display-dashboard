#!/bin/bash
# Debug script for web server connectivity issues

DEVICE_IP="${1:-}"

if [ -z "$DEVICE_IP" ]; then
    echo "Usage: $0 <device-ip>"
    echo "This script helps debug web server connectivity issues"
    exit 1
fi

echo "=== ESP32 Web Server Debug Script ==="
echo "Testing device at: $DEVICE_IP"
echo

# Test 1: Basic connectivity
echo "1. Testing network connectivity..."
if ping -c 3 -W 2 "$DEVICE_IP" > /dev/null 2>&1; then
    echo "✓ Device is reachable on network"
else
    echo "✗ Cannot reach device - check WiFi connection"
    exit 1
fi

# Test 2: Telnet connectivity
echo -e "\n2. Testing telnet server (port 23)..."
if timeout 2 bash -c "echo 'test' | nc -w 1 $DEVICE_IP 23" > /dev/null 2>&1; then
    echo "✓ Telnet server is responding"
else
    echo "✗ Telnet server not responding"
fi

# Test 3: Web server ports
echo -e "\n3. Testing web server ports..."
for PORT in 80 8080; do
    echo -n "   Port $PORT: "
    if timeout 2 bash -c "echo -e 'GET / HTTP/1.0\r\n\r\n' | nc -w 1 $DEVICE_IP $PORT" > /dev/null 2>&1; then
        echo "✓ Responding"
    else
        echo "✗ Not responding"
    fi
done

# Test 4: HTTP endpoints
echo -e "\n4. Testing HTTP endpoints..."
test_endpoint() {
    local path=$1
    local desc=$2
    echo -n "   $desc: "
    response=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 2 "http://$DEVICE_IP$path" 2>/dev/null)
    if [ "$response" = "200" ]; then
        echo "✓ OK (200)"
    elif [ "$response" = "401" ]; then
        echo "⚠ Auth required (401)"
    elif [ "$response" = "404" ]; then
        echo "✗ Not found (404)"
    else
        echo "✗ Error ($response)"
    fi
}

test_endpoint "/" "Home page"
test_endpoint "/health" "Health check"
test_endpoint "/metrics" "Metrics"
test_endpoint "/api/system" "System API"
test_endpoint "/ota" "OTA page"

# Test 5: mDNS discovery
echo -e "\n5. Testing mDNS discovery..."
if command -v avahi-browse >/dev/null 2>&1; then
    echo "   Searching for ESP32 services..."
    timeout 3 avahi-browse -at 2>/dev/null | grep -E "esp32|_http|_esp32-ota" || echo "   No mDNS services found"
elif command -v dns-sd >/dev/null 2>&1; then
    echo "   Searching for ESP32 services..."
    timeout 3 dns-sd -B _http._tcp 2>/dev/null | grep esp32 || echo "   No mDNS services found"
else
    echo "   ⚠ No mDNS tools available (install avahi-utils or dns-sd)"
fi

# Test 6: Detailed curl test
echo -e "\n6. Detailed connection test..."
echo "   Full curl output for home page:"
curl -v --connect-timeout 5 "http://$DEVICE_IP/" 2>&1 | grep -E "^(\*|<|>)" | head -20

echo -e "\n=== Diagnosis Summary ==="
echo "If web server is not responding but telnet works:"
echo "1. Web server may not be starting due to WiFi timing issues"
echo "2. Authentication middleware may be blocking requests"
echo "3. Port configuration may be incorrect"
echo ""
echo "Try these fixes:"
echo "- Flash with --no-erase to preserve NVS settings"
echo "- Check serial output: espflash monitor"
echo "- Use recovery mode if implemented"