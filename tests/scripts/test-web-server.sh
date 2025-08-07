#!/bin/bash
# Quick test script for web server connectivity

DEVICE_IP="${1:-}"

if [ -z "$DEVICE_IP" ]; then
    # Try to find device via mDNS
    echo "No IP provided, searching for ESP32 via mDNS..."
    
    if command -v avahi-resolve >/dev/null 2>&1; then
        DEVICE_IP=$(avahi-resolve -n esp32.local 2>/dev/null | awk '{print $2}')
    elif command -v dig >/dev/null 2>&1; then
        DEVICE_IP=$(dig +short esp32.local)
    fi
    
    if [ -z "$DEVICE_IP" ]; then
        echo "Usage: $0 <device-ip>"
        echo "Could not auto-discover device IP"
        exit 1
    fi
    
    echo "Found device at: $DEVICE_IP"
fi

echo "Testing ESP32 Web Server at $DEVICE_IP"
echo "======================================"

# Function to test endpoint
test_endpoint() {
    local endpoint=$1
    local expected=$2
    local method=${3:-GET}
    
    echo -n "Testing $method $endpoint... "
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "http://$DEVICE_IP$endpoint" 2>/dev/null | tail -1)
    else
        response=$(curl -s -X $method -w "\n%{http_code}" "http://$DEVICE_IP$endpoint" 2>/dev/null | tail -1)
    fi
    
    if [ "$response" = "$expected" ]; then
        echo "✓ OK ($response)"
    else
        echo "✗ FAIL (got $response, expected $expected)"
    fi
}

# Test basic connectivity
echo -n "1. Network connectivity... "
if ping -c 1 -W 2 "$DEVICE_IP" > /dev/null 2>&1; then
    echo "✓ OK"
else
    echo "✗ FAIL"
    exit 1
fi

# Test endpoints
echo -e "\n2. Testing HTTP endpoints:"
test_endpoint "/" "200"
test_endpoint "/health" "200"
test_endpoint "/metrics" "200"
test_endpoint "/api/system" "200"
test_endpoint "/ota" "200"
test_endpoint "/api/config" "200"

echo -e "\n3. Testing protected endpoints:"
test_endpoint "/restart" "405" "GET"  # Should fail - GET not allowed
test_endpoint "/restart" "403" "POST" # Should fail - no auth
test_endpoint "/api/restart" "403" "POST" # Should fail - no auth

echo -e "\n4. Testing with authentication:"
echo -n "Testing POST /restart with auth... "
response=$(curl -s -X POST -H "X-Restart-Token: esp32-restart" -w "\n%{http_code}" "http://$DEVICE_IP/restart" 2>/dev/null | tail -1)
if [ "$response" = "200" ]; then
    echo "✓ OK (200) - Device will restart!"
else
    echo "✗ Unexpected response: $response"
fi

echo -e "\n5. Quick functionality test:"
echo -n "Fetching metrics data... "
metrics=$(curl -s "http://$DEVICE_IP/api/metrics" 2>/dev/null)
if echo "$metrics" | grep -q "uptime"; then
    echo "✓ OK"
    echo "  Uptime: $(echo "$metrics" | grep -o '"uptime":[0-9]*' | cut -d: -f2) seconds"
    echo "  Heap: $(echo "$metrics" | grep -o '"heap_free":[0-9]*' | cut -d: -f2) bytes"
else
    echo "✗ FAIL"
fi

echo -e "\n======================================"
echo "Test Summary:"
echo "- If all tests pass: Web server is working correctly!"
echo "- If tests fail: Run ./scripts/debug-web-server.sh for details"
echo "- Check serial output: espflash monitor"