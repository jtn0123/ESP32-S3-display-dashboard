#!/bin/bash
# Sequential Python test runner for ESP32-S3 Dashboard
# Runs tests one by one with delays to prevent device crashes

set -e

echo "================================================================"
echo "ESP32-S3 Dashboard - Sequential Python Test Runner"
echo "================================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DEVICE_IP="10.27.27.201"
DELAY_BETWEEN_TESTS=3  # seconds
HEALTH_CHECK_RETRIES=5
VERBOSE=false
TEST_PATTERN=""
SKIP_PREFLIGHT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --device-ip|--ip)
            DEVICE_IP="$2"
            shift 2
            ;;
        --delay)
            DELAY_BETWEEN_TESTS="$2"
            shift 2
            ;;
        --pattern|-k)
            TEST_PATTERN="$2"
            shift 2
            ;;
        --skip-preflight)
            SKIP_PREFLIGHT=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --device-ip IP      Device IP address (default: 10.27.27.201)"
            echo "  --delay SECONDS     Delay between tests (default: 3)"
            echo "  --pattern PATTERN   Run only tests matching pattern"
            echo "  --skip-preflight    Skip pre-flight device checks"
            echo "  --verbose, -v       Verbose output"
            echo "  --help, -h          Show this help"
            echo ""
            echo "Examples:"
            echo "  $0                              # Run all tests"
            echo "  $0 --delay 5                    # 5 second delay between tests"
            echo "  $0 --pattern test_metrics       # Run only metrics tests"
            echo "  $0 -k 'not test_config'         # Skip config tests"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Function to print section headers
print_section() {
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}▶ $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Function to check device health
check_device_health() {
    local retries=0
    while [ $retries -lt $HEALTH_CHECK_RETRIES ]; do
        if curl -s -m 5 "http://$DEVICE_IP/health" > /dev/null 2>&1; then
            return 0
        fi
        retries=$((retries + 1))
        echo -e "${YELLOW}Device not responding, retry $retries/$HEALTH_CHECK_RETRIES...${NC}"
        sleep 5
    done
    return 1
}

# Function to get device uptime
get_device_uptime() {
    local uptime=$(curl -s -m 5 "http://$DEVICE_IP/health" 2>/dev/null | grep -o '"uptime_seconds":[0-9]*' | cut -d: -f2)
    echo ${uptime:-0}
}

# Change to test directory
cd tests/python

# Activate virtual environment
if [ -d "venv" ]; then
    source venv/bin/activate
else
    echo -e "${RED}Virtual environment not found!${NC}"
    exit 1
fi

# ========== PRE-FLIGHT CHECKS ==========
if [ "$SKIP_PREFLIGHT" = false ]; then
    print_section "Pre-flight Device Check"
    
    echo "Checking device at $DEVICE_IP..."
    if check_device_health; then
        INITIAL_UPTIME=$(get_device_uptime)
        echo -e "${GREEN}✓${NC} Device is healthy (uptime: ${INITIAL_UPTIME}s)"
    else
        echo -e "${RED}✗${NC} Device is not responding!"
        exit 1
    fi
fi

# ========== COLLECT TESTS ==========
print_section "Collecting Tests"

# Build pytest collection command
COLLECT_CMD="pytest --collect-only -q"
if [ -n "$TEST_PATTERN" ]; then
    COLLECT_CMD="$COLLECT_CMD -k '$TEST_PATTERN'"
fi
COLLECT_CMD="$COLLECT_CMD tests/"

# Collect all test names
echo "Collecting tests..."
# Use different approach - list tests with their full path
TEST_LIST=$(eval $COLLECT_CMD --no-header 2>/dev/null | grep "<Function" | sed 's/.*<Function //' | sed 's/>//' | while read test; do echo "tests/test_web_comprehensive.py::TestWebComprehensive::$test"; done || true)

if [ -z "$TEST_LIST" ]; then
    echo -e "${RED}No tests found!${NC}"
    exit 1
fi

TEST_COUNT=$(echo "$TEST_LIST" | wc -l | tr -d ' ')
echo -e "${GREEN}Found $TEST_COUNT tests to run${NC}"

# ========== RUN TESTS SEQUENTIALLY ==========
print_section "Running Tests Sequentially"

echo -e "Delay between tests: ${DELAY_BETWEEN_TESTS}s\n"

# Track results
PASSED=0
FAILED=0
SKIPPED=0
FAILED_TESTS=""

# Run each test individually
TEST_NUM=0
while IFS= read -r test; do
    TEST_NUM=$((TEST_NUM + 1))
    
    # Extract test name for display
    TEST_NAME=$(echo "$test" | sed 's/.*:://')
    echo -e "\n${BLUE}[$TEST_NUM/$TEST_COUNT]${NC} Running: $TEST_NAME"
    
    # Check device health before each test
    UPTIME_BEFORE=$(get_device_uptime)
    
    # Build pytest command
    PYTEST_CMD="pytest -v --tb=short --device-ip $DEVICE_IP"
    if [ "$VERBOSE" = true ]; then
        PYTEST_CMD="$PYTEST_CMD -s"
    fi
    PYTEST_CMD="$PYTEST_CMD $test"
    
    # Run the test
    if eval $PYTEST_CMD > test_output.tmp 2>&1; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        # Check if it was skipped
        if grep -q "SKIPPED" test_output.tmp; then
            echo -e "${YELLOW}⊘ SKIPPED${NC}"
            SKIPPED=$((SKIPPED + 1))
        else
            echo -e "${RED}✗ FAILED${NC}"
            FAILED=$((FAILED + 1))
            FAILED_TESTS="$FAILED_TESTS\n  - $TEST_NAME"
            
            # Show failure reason if verbose
            if [ "$VERBOSE" = true ]; then
                echo -e "${RED}Failure output:${NC}"
                grep -A 5 "FAILED\|ERROR\|AssertionError" test_output.tmp || true
            fi
        fi
    fi
    
    # Check if device rebooted
    UPTIME_AFTER=$(get_device_uptime)
    if [ $UPTIME_AFTER -lt $UPTIME_BEFORE ]; then
        echo -e "${YELLOW}⚠️  Device rebooted during test!${NC}"
        echo "Waiting for device to stabilize..."
        sleep 10
        check_device_health || exit 1
    fi
    
    # Delay between tests (except after last test)
    if [ $TEST_NUM -lt $TEST_COUNT ]; then
        echo -e "${YELLOW}Waiting ${DELAY_BETWEEN_TESTS}s before next test...${NC}"
        sleep $DELAY_BETWEEN_TESTS
    fi
    
done <<< "$TEST_LIST"

# Clean up
rm -f test_output.tmp

# ========== FINAL SUMMARY ==========
print_section "Test Summary"

echo -e "Total tests: $TEST_COUNT"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo -e "${YELLOW}Skipped: $SKIPPED${NC}"

if [ $FAILED -gt 0 ]; then
    echo -e "\n${RED}Failed tests:${NC}$FAILED_TESTS"
fi

# Check final device health
echo -e "\nFinal device check..."
if check_device_health; then
    FINAL_UPTIME=$(get_device_uptime)
    echo -e "${GREEN}✓${NC} Device is still healthy (uptime: ${FINAL_UPTIME}s)"
else
    echo -e "${RED}✗${NC} Device is not responding!"
fi

# Deactivate virtual environment
deactivate 2>/dev/null || true

cd ../..

# Exit with appropriate code
if [ $FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi