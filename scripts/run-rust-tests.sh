#!/bin/bash
# Script to run Rust tests (host-based unit tests)

set -e

echo "================================================================"
echo "ESP32-S3 Dashboard - Rust Test Runner"
echo "================================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --verbose, -v   Verbose output"
            echo "  --help, -h      Show this help"
            echo ""
            echo "This script runs host-based Rust unit tests."
            echo "These tests run on your development machine, not on the ESP32."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

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

echo -e "\n${YELLOW}Running Host-Based Rust Tests${NC}"
echo "================================"

# Check if host-tests directory exists
if [ -d "host-tests" ]; then
    cd host-tests
    
    # Detect the host target
    HOST_TARGET=$(rustc +stable --version --verbose 2>/dev/null | grep "host:" | cut -d' ' -f2)
    
    if [ -z "$HOST_TARGET" ]; then
        echo -e "${YELLOW}Warning: Could not detect host target, using default${NC}"
        if run_command "cargo test" "Host unit tests"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
    else
        if run_command "cargo +stable test --target $HOST_TARGET" "Host unit tests"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
    fi
    
    cd ..
else
    echo -e "${RED}Error: host-tests directory not found${NC}"
    echo "Please run from the project root directory"
    exit 1
fi

# Count unit tests in main codebase
echo -e "\n${YELLOW}Checking for unit tests in main codebase...${NC}"
TEST_COUNT=$(find src -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | wc -l)
echo "Found $TEST_COUNT files with test modules"

if [ $TEST_COUNT -gt 0 ]; then
    echo -e "${YELLOW}Note: ESP32 target tests must be run on device or with emulator${NC}"
    echo "Files with tests:"
    find src -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | sort
fi

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