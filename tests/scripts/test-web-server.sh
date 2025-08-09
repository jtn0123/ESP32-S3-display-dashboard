#!/bin/bash
# Quick test script for web server connectivity

DEVICE_IP="${1:-}"
RESTART_TOKEN_HEADER="X-Restart-Token"
RESTART_TOKEN_VALUE="${RESTART_TOKEN:-esp32-restart}"
CURL="curl -sS --max-time 5"
PASS=0; FAIL=0

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
    local extra_args=${4:-}

    echo -n "Testing $method $endpoint... "

    local code
    if [ "$method" = "GET" ]; then
        code=$($CURL -w "%{http_code}" "http://$DEVICE_IP$endpoint" $extra_args -o /dev/null 2>/dev/null)
    else
        code=$($CURL -X "$method" -w "%{http_code}" "http://$DEVICE_IP$endpoint" $extra_args -o /dev/null 2>/dev/null)
    fi

    if [ "$code" = "$expected" ]; then
        echo "✓ OK ($code)"; PASS=$((PASS+1))
    else
        echo "✗ FAIL (got $code, expected $expected)"; FAIL=$((FAIL+1))
    fi
}

# Test basic connectivity
echo -n "1. Network connectivity... "
if ping -c 1 -W 2 "$DEVICE_IP" > /dev/null 2>&1; then
    echo "✓ OK"; PASS=$((PASS+1))
else
    echo "✗ FAIL"; FAIL=$((FAIL+1))
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

# Some builds return 400 (bad request) vs 403 (forbidden) when auth header missing
echo -n "Testing POST /restart unauth... "
code=$($CURL -X POST -w "%{http_code}" "http://$DEVICE_IP/restart" -o /dev/null 2>/dev/null)
if [ "$code" = "403" ] || [ "$code" = "400" ]; then
    echo "✓ OK ($code)"; PASS=$((PASS+1))
else
    echo "✗ FAIL (got $code, expected 400/403)"; FAIL=$((FAIL+1))
fi

test_endpoint "/api/restart" "403" "POST"

echo -e "\n4. Testing with authentication (optional):"
if [ "${RUN_RESTART:-0}" = "1" ]; then
    echo -n "Testing POST /restart with auth... "
    code=$($CURL -X POST -H "$RESTART_TOKEN_HEADER: $RESTART_TOKEN_VALUE" -w "%{http_code}" "http://$DEVICE_IP/restart" -o /dev/null 2>/dev/null)
    if [ "$code" = "200" ]; then
        echo "✓ OK (200) - Device will restart!"; PASS=$((PASS+1))
    else
        echo "✗ Unexpected response: $code"; FAIL=$((FAIL+1))
    fi
else
    echo "Skipping restart test (set RUN_RESTART=1 to enable)"; PASS=$((PASS+1))
fi

echo -e "\n5. Quick functionality test:"
echo -n "Fetching metrics data... "
metrics=$($CURL "http://$DEVICE_IP/api/metrics" 2>/dev/null)
if echo "$metrics" | grep -q '"uptime"'; then
    echo "✓ OK"; PASS=$((PASS+1))
    echo "  Uptime: $(echo "$metrics" | grep -o '"uptime":[0-9]*' | head -1 | cut -d: -f2) seconds"
    echo "  Heap: $(echo "$metrics" | grep -o '"heap_free":[0-9]*' | head -1 | cut -d: -f2) bytes"
else
    echo "✗ FAIL"; FAIL=$((FAIL+1))
fi

echo -e "\n6. SSE stream check (/api/events):"
echo -n "Sampling 3 events... "
sse_sample=$($CURL -N "http://$DEVICE_IP/api/events" 2>/dev/null | sed -n '1,10p')
if echo "$sse_sample" | grep -q '"type":"metrics"'; then
    echo "✓ OK"; PASS=$((PASS+1))
else
    echo "✗ FAIL"; FAIL=$((FAIL+1))
fi

echo -e "\n======================================"
echo "Summary: $PASS passed, $FAIL failed"
echo "- If tests fail: ./scripts/debug-web-server.sh"
echo "- Auth header used: $RESTART_TOKEN_HEADER: $RESTART_TOKEN_VALUE"