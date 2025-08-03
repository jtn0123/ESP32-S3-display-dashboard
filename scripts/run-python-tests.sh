#!/bin/bash
# Script to run Python integration tests against ESP32 device

set -e

echo "================================================================"
echo "ESP32-S3 Dashboard - Python Integration Test Runner"
echo "================================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
DEVICE_IP="10.27.27.201"
VERBOSE=false
TEST_SUITE="simple"  # simple or full

while [[ $# -gt 0 ]]; do
    case $1 in
        --device-ip|--ip)
            DEVICE_IP="$2"
            shift 2
            ;;
        --full)
            TEST_SUITE="full"
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
            echo "  --device-ip IP  Device IP address (default: 10.27.27.201)"
            echo "  --full          Run full test suite (default: simple suite)"
            echo "  --verbose, -v   Verbose output"
            echo "  --help, -h      Show this help"
            echo ""
            echo "This script runs Python integration tests against a live ESP32 device."
            echo "Make sure your device is running and accessible on the network."
            echo ""
            echo "Examples:"
            echo "  $0                                  # Run simple tests on default IP"
            echo "  $0 --device-ip 192.168.1.100       # Run on custom IP"
            echo "  $0 --full --verbose                # Run full test suite with verbose output"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "Device IP: $DEVICE_IP"
echo "Test Suite: $TEST_SUITE"

# Function to run command with proper error handling
run_command() {
    local cmd="$1"
    local description="$2"
    
    echo -e "\n${YELLOW}→ ${description}${NC}"
    
    if [ "$VERBOSE" = true ]; then
        eval "$cmd"
    else
        eval "$cmd" > /tmp/test_output.log 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ ${description} passed${NC}"
        return 0
    else
        echo -e "${RED}✗ ${description} failed${NC}"
        if [ "$VERBOSE" = false ]; then
            echo "Output:"
            tail -20 /tmp/test_output.log
        fi
        return 1
    fi
}

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Check Python environment
echo -e "\n${YELLOW}Checking Python environment...${NC}"
python3 --version

# Change to test directory
cd tests/python

# Check if virtual environment exists
if [ -d "venv" ]; then
    echo -e "${GREEN}✓ Virtual environment found${NC}"
    source venv/bin/activate 2>/dev/null || true
else
    echo -e "${YELLOW}Note: No virtual environment found. Using system Python.${NC}"
fi

# Check if pytest is available
if ! command -v pytest &> /dev/null; then
    echo -e "${RED}Error: pytest not found${NC}"
    echo "Please install pytest: pip install pytest pytest-timeout"
    exit 1
fi

# Check device connectivity
echo -e "\n${YELLOW}Checking device connectivity...${NC}"
if run_command "curl -s -f -m 5 http://$DEVICE_IP/health" "Device health check"; then
    ((TESTS_PASSED++))
    
    # Run appropriate test suite
    if [ "$TEST_SUITE" = "simple" ]; then
        # Run basic smoke tests only
        PYTEST_ARGS="-v --tb=short --device-ip $DEVICE_IP -k 'test_health or test_basic' --maxfail=5"
        if run_command "pytest $PYTEST_ARGS tests/" "Basic smoke tests"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
    else
        # Full test suite with all tests
        PYTEST_ARGS="-v --tb=short --device-ip $DEVICE_IP"
        if [ "$VERBOSE" = true ]; then
            PYTEST_ARGS="$PYTEST_ARGS -s"
        fi
        
        # Run pytest with proper arguments
        if run_command "pytest $PYTEST_ARGS tests/" "Full pytest suite"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
    fi
else
    echo -e "${RED}Device not reachable at $DEVICE_IP${NC}"
    echo "Please check:"
    echo "  1. Device is powered on and running"
    echo "  2. Device is connected to the network"
    echo "  3. IP address is correct"
    echo "  4. Firewall is not blocking connection"
    ((TESTS_FAILED++))
fi

# Deactivate virtual environment if it was activated
deactivate 2>/dev/null || true

cd ../..

# Summary
echo -e "\n================================================================"
echo -e "${YELLOW}Test Summary${NC}"
echo "================================================================"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo -e "Total: $((TESTS_PASSED + TESTS_FAILED))"

# Cleanup
rm -f /tmp/test_output.log

# Exit with appropriate code
if [ $TESTS_FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi