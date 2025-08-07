#!/bin/bash
# Simple sequential test runner for ESP32-S3 Dashboard

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
DEVICE_IP="${1:-10.27.27.201}"
DELAY_BETWEEN_TESTS="${2:-3}"
TEST_FILE="tests/test_web_comprehensive.py"

echo "================================================================"
echo "ESP32-S3 Dashboard - Simple Sequential Test Runner"
echo "================================================================"
echo "Device IP: $DEVICE_IP"
echo "Delay between tests: ${DELAY_BETWEEN_TESTS}s"
echo ""

# Change to test directory
cd tests/python

# Activate virtual environment
source venv/bin/activate

# List of tests to run (excluding problematic ones)
TESTS=(
    "test_all_api_endpoints"
    "test_metrics_accuracy"
    "test_concurrent_requests"
    "test_error_handling"
    "test_response_headers"
    "test_websocket_support"
    "test_cors_headers"
    "test_compression_support"
    "test_rate_limiting"
    "test_authentication"
)

# Track results
PASSED=0
FAILED=0
TOTAL=${#TESTS[@]}

echo "Running $TOTAL tests sequentially..."
echo ""

# Run each test
for i in "${!TESTS[@]}"; do
    TEST="${TESTS[$i]}"
    TEST_NUM=$((i + 1))
    
    echo -e "${BLUE}[$TEST_NUM/$TOTAL]${NC} Running $TEST..."
    
    # Run the test
    if pytest -v --tb=short --device-ip $DEVICE_IP ${TEST_FILE}::TestWebComprehensive::${TEST} > /tmp/test_output.log 2>&1; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ FAILED${NC}"
        FAILED=$((FAILED + 1))
        
        # Show brief error info
        echo -e "${RED}Error:${NC}"
        grep -A 3 "FAILED\|ERROR\|AssertionError" /tmp/test_output.log | head -10 || true
    fi
    
    # Check device health
    if ! curl -s -m 5 "http://$DEVICE_IP/health" > /dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  Device not responding! Waiting 30s...${NC}"
        sleep 30
        
        # Retry health check
        if ! curl -s -m 5 "http://$DEVICE_IP/health" > /dev/null 2>&1; then
            echo -e "${RED}Device still not responding. Aborting tests.${NC}"
            break
        fi
        echo -e "${GREEN}Device recovered.${NC}"
    fi
    
    # Delay between tests
    if [ $TEST_NUM -lt $TOTAL ]; then
        echo -e "${YELLOW}Waiting ${DELAY_BETWEEN_TESTS}s...${NC}"
        sleep $DELAY_BETWEEN_TESTS
    fi
    echo ""
done

# Summary
echo "================================================================"
echo "Test Summary"
echo "================================================================"
echo -e "Total: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

# Clean up
rm -f /tmp/test_output.log
deactivate

cd ../..

# Exit code
if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed.${NC}"
    exit 1
fi