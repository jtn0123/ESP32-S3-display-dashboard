#!/bin/bash
# Script to run Rust tests for ESP32-S3 Dashboard

set -e

echo "================================================================"
echo "ESP32-S3 Dashboard - Test Runner"
echo "================================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
RUN_HOST_TESTS=false
RUN_PYTHON_TESTS=false
VERBOSE=false
DEVICE_IP="10.27.27.201"

while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            RUN_HOST_TESTS=true
            shift
            ;;
        --python)
            RUN_PYTHON_TESTS=true
            shift
            ;;
        --all)
            RUN_HOST_TESTS=true
            RUN_PYTHON_TESTS=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --device-ip)
            DEVICE_IP="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --host          Run host-based Rust tests"
            echo "  --python        Run Python integration tests"
            echo "  --all           Run all tests (host + python)"
            echo "  --device-ip IP  Device IP for Python tests (default: 10.27.27.201)"
            echo "  --verbose, -v   Verbose output"
            echo "  --help, -h      Show this help"
            echo ""
            echo "Examples:"
            echo "  $0 --all                    # Run all tests"
            echo "  $0 --host                   # Run only Rust tests"
            echo "  $0 --python --device-ip 192.168.1.100  # Run Python tests with custom IP"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Default to running host tests if nothing specified
if [ "$RUN_HOST_TESTS" = false ] && [ "$RUN_PYTHON_TESTS" = false ]; then
    RUN_HOST_TESTS=true
fi

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

# Run host-based Rust tests
if [ "$RUN_HOST_TESTS" = true ]; then
    echo -e "\n${YELLOW}Running Host-Based Rust Tests${NC}"
    echo "================================"
    
    # Check if host-tests directory exists
    if [ -d "host-tests" ]; then
        cd host-tests
        
        # Run cargo test with stable toolchain and correct target
        # Detect the host target
        HOST_TARGET=$(rustc +stable --version --verbose | grep "host:" | cut -d' ' -f2)
        if run_command "cargo +stable test --target $HOST_TARGET" "Library tests"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
        
        cd ..
    else
        echo -e "${YELLOW}Note: host-tests directory not found. Creating it...${NC}"
        
        # Create host-tests directory and basic structure
        mkdir -p host-tests/src
        
        # Create a basic Cargo.toml for host tests
        cat > host-tests/Cargo.toml << 'EOF'
[package]
name = "esp32-dashboard-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
# Add test dependencies here

[dev-dependencies]
# Test-specific dependencies
EOF
        
        # Create a basic lib.rs
        cat > host-tests/src/lib.rs << 'EOF'
//! Host-based tests for ESP32-S3 Dashboard

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert_eq!(2 + 2, 4);
    }
}
EOF
        
        echo -e "${GREEN}Created host-tests structure. Re-run to execute tests.${NC}"
    fi
    
    # Run unit tests in main codebase (if ESP toolchain supports it)
    echo -e "\n${YELLOW}Checking for unit tests in main codebase...${NC}"
    
    # Count test modules
    TEST_COUNT=$(find src -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | wc -l)
    echo "Found $TEST_COUNT files with test modules"
    
    if [ $TEST_COUNT -gt 0 ]; then
        echo -e "${YELLOW}Note: ESP32 target tests must be run on device or with emulator${NC}"
    fi
fi

# Run Python integration tests
if [ "$RUN_PYTHON_TESTS" = true ]; then
    echo -e "\n${YELLOW}Running Python Integration Tests${NC}"
    echo "===================================="
    echo "Device IP: $DEVICE_IP"
    
    cd tests/python
    
    # Check if device is reachable
    if run_command "curl -s -f http://$DEVICE_IP/health" "Device connectivity check"; then
        ((TESTS_PASSED++))
        
        # Run simple test runner
        if [ -f "run_tests_simple.py" ]; then
            if run_command "python3 run_tests_simple.py $DEVICE_IP" "Python integration tests"; then
                ((TESTS_PASSED++))
            else
                ((TESTS_FAILED++))
            fi
        else
            echo -e "${RED}run_tests_simple.py not found${NC}"
            ((TESTS_FAILED++))
        fi
    else
        echo -e "${RED}Device not reachable at $DEVICE_IP${NC}"
        ((TESTS_FAILED++))
    fi
    
    cd ../..
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