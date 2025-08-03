#!/bin/bash
# Script to run minimal Python integration tests against ESP32 device
# This is a safe subset that won't crash the device

set -e

echo "================================================================"
echo "ESP32-S3 Dashboard - Minimal Test Runner"
echo "================================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
DEVICE_IP="10.27.27.201"
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --device-ip|--ip)
            DEVICE_IP="$2"
            shift 2
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
            echo "  --verbose, -v   Verbose output"
            echo "  --help, -h      Show this help"
            echo ""
            echo "This script runs a minimal set of safe tests that won't crash the device."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "Device IP: $DEVICE_IP"

# Check Python environment
echo -e "\n${YELLOW}Checking Python environment...${NC}"
python3 --version

# Change to test directory
cd tests/python

# Activate virtual environment if available
if [ -d "venv" ]; then
    echo -e "${GREEN}✓ Virtual environment found${NC}"
    source venv/bin/activate
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
if curl -s -f -m 5 http://$DEVICE_IP/health > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Device is reachable${NC}"
else
    echo -e "${RED}✗ Device not reachable at $DEVICE_IP${NC}"
    exit 1
fi

# Run minimal test suite
echo -e "\n${YELLOW}Running minimal test suite...${NC}"

PYTEST_ARGS="-v --tb=short --device-ip $DEVICE_IP"
if [ "$VERBOSE" = true ]; then
    PYTEST_ARGS="$PYTEST_ARGS -s"
fi

# Run only the minimal test file
pytest $PYTEST_ARGS tests/test_web_server_minimal.py

# Deactivate virtual environment if it was activated
deactivate 2>/dev/null || true

cd ../..

echo -e "\n${GREEN}✓ Minimal tests completed${NC}"