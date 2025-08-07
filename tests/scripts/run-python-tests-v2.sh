#!/bin/bash
# Enhanced Python test runner with environment validation and pre-flight checks

set -e

echo "================================================================"
echo "ESP32-S3 Dashboard - Python Test Runner v2"
echo "================================================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DEVICE_IP="10.27.27.201"
VERBOSE=false
TEST_SUITE="smoke"  # smoke, integration, full
SKIP_PREFLIGHT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --device-ip|--ip)
            DEVICE_IP="$2"
            shift 2
            ;;
        --suite)
            TEST_SUITE="$2"
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
            echo "  --suite TYPE        Test suite: smoke, integration, full (default: smoke)"
            echo "  --skip-preflight    Skip pre-flight checks"
            echo "  --verbose, -v       Verbose output"
            echo "  --help, -h          Show this help"
            echo ""
            echo "Test Suites:"
            echo "  smoke       - Basic connectivity and health checks (fast)"
            echo "  integration - API and functionality tests (medium)"
            echo "  full        - All tests including stress tests (slow)"
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

# Function to check requirement
check_requirement() {
    local cmd="$1"
    local name="$2"
    local install_hint="$3"
    
    if command -v "$cmd" &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} $name found: $(command -v $cmd)"
        return 0
    else
        echo -e "  ${RED}✗${NC} $name not found"
        if [ -n "$install_hint" ]; then
            echo -e "    ${YELLOW}→ Install with: $install_hint${NC}"
        fi
        return 1
    fi
}

# Track results
PREFLIGHT_PASSED=0
PREFLIGHT_FAILED=0
TESTS_PASSED=0
TESTS_FAILED=0

# Change to test directory
cd tests/python

# ========== ENVIRONMENT VALIDATION ==========
print_section "Environment Validation"

# Check Python version
PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)

echo -e "Python version: $PYTHON_VERSION"
if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 8 ]; then
    echo -e "  ${GREEN}✓${NC} Python version OK (3.8+)"
    ((PREFLIGHT_PASSED++))
else
    echo -e "  ${RED}✗${NC} Python 3.8+ required"
    ((PREFLIGHT_FAILED++))
fi

# Check virtual environment
if [ -d "venv" ]; then
    echo -e "  ${GREEN}✓${NC} Virtual environment found"
    source venv/bin/activate 2>/dev/null || true
    ((PREFLIGHT_PASSED++))
else
    echo -e "  ${YELLOW}!${NC} No virtual environment found"
    echo -e "    ${YELLOW}→ Create with: python3 -m venv venv${NC}"
fi

# Check required tools
echo -e "\nChecking required tools:"
check_requirement "pytest" "pytest" "pip install pytest" && ((PREFLIGHT_PASSED++)) || ((PREFLIGHT_FAILED++))
check_requirement "curl" "curl" "apt/brew install curl" && ((PREFLIGHT_PASSED++)) || ((PREFLIGHT_FAILED++))
check_requirement "ping" "ping" "" && ((PREFLIGHT_PASSED++)) || ((PREFLIGHT_FAILED++))

# Check pytest plugins
echo -e "\nChecking pytest plugins:"
if command -v pytest &> /dev/null; then
    PYTEST_PLUGINS=$(pytest --version 2>&1 | grep -E "pytest-timeout|pytest-asyncio" || true)
    if echo "$PYTEST_PLUGINS" | grep -q "timeout"; then
        echo -e "  ${GREEN}✓${NC} pytest-timeout installed"
        ((PREFLIGHT_PASSED++))
    else
        echo -e "  ${YELLOW}!${NC} pytest-timeout not found (optional)"
        echo -e "    ${YELLOW}→ Install with: pip install pytest-timeout${NC}"
    fi
fi

# ========== DEVICE CONNECTIVITY ==========
print_section "Device Connectivity Tests"

echo "Target device: $DEVICE_IP"

# Test 1: Ping
echo -e "\n1. Network connectivity (ping):"
if ping -c 1 -W 2 "$DEVICE_IP" > /dev/null 2>&1; then
    RTT=$(ping -c 1 -W 2 "$DEVICE_IP" 2>/dev/null | grep -oE 'time=[0-9.]+ ms' | grep -oE '[0-9.]+' || echo "N/A")
    echo -e "  ${GREEN}✓${NC} Device reachable (RTT: ${RTT}ms)"
    ((PREFLIGHT_PASSED++))
else
    echo -e "  ${RED}✗${NC} Device not reachable via ping"
    ((PREFLIGHT_FAILED++))
fi

# Test 2: HTTP Health
echo -e "\n2. HTTP connectivity:"
HTTP_RESPONSE=$(curl -s -w "\n%{http_code}" -m 5 "http://$DEVICE_IP/health" 2>/dev/null || echo "FAILED")
HTTP_CODE=$(echo "$HTTP_RESPONSE" | tail -n1)
HTTP_BODY=$(echo "$HTTP_RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ]; then
    echo -e "  ${GREEN}✓${NC} Health endpoint OK"
    
    # Parse JSON response
    if command -v jq &> /dev/null && [ -n "$HTTP_BODY" ]; then
        STATUS=$(echo "$HTTP_BODY" | jq -r '.status' 2>/dev/null || echo "unknown")
        VERSION=$(echo "$HTTP_BODY" | jq -r '.version' 2>/dev/null || echo "unknown")
        HEAP=$(echo "$HTTP_BODY" | jq -r '.free_heap' 2>/dev/null || echo "unknown")
        
        echo -e "    Status: $STATUS"
        echo -e "    Version: $VERSION"
        echo -e "    Free heap: $((HEAP / 1024 / 1024))MB"
    else
        echo -e "    Response: ${HTTP_BODY:0:50}..."
    fi
    ((PREFLIGHT_PASSED++))
else
    echo -e "  ${RED}✗${NC} Health endpoint failed (HTTP $HTTP_CODE)"
    ((PREFLIGHT_FAILED++))
fi

# Test 3: Check key endpoints
echo -e "\n3. Endpoint availability:"
ENDPOINTS=("/" "/api/metrics" "/api/system" "/api/config")
ENDPOINT_OK=0

for endpoint in "${ENDPOINTS[@]}"; do
    CODE=$(curl -s -o /dev/null -w "%{http_code}" -m 2 "http://$DEVICE_IP$endpoint" 2>/dev/null || echo "000")
    if [ "$CODE" = "200" ]; then
        echo -e "  ${GREEN}✓${NC} $endpoint - OK"
        ((ENDPOINT_OK++))
    else
        echo -e "  ${RED}✗${NC} $endpoint - HTTP $CODE"
    fi
done

if [ $ENDPOINT_OK -ge 3 ]; then
    echo -e "  ${GREEN}✓${NC} Most endpoints available ($ENDPOINT_OK/4)"
    ((PREFLIGHT_PASSED++))
else
    echo -e "  ${RED}✗${NC} Too many endpoints unavailable ($ENDPOINT_OK/4)"
    ((PREFLIGHT_FAILED++))
fi

# ========== PRE-FLIGHT SUMMARY ==========
print_section "Pre-flight Check Summary"

echo -e "Pre-flight checks:"
echo -e "  ${GREEN}Passed: $PREFLIGHT_PASSED${NC}"
echo -e "  ${RED}Failed: $PREFLIGHT_FAILED${NC}"

if [ $PREFLIGHT_FAILED -gt 0 ] && [ "$SKIP_PREFLIGHT" = false ]; then
    echo -e "\n${RED}⚠️  Pre-flight checks failed!${NC}"
    echo "Some tests may fail due to missing requirements or connectivity issues."
    echo "Use --skip-preflight to run tests anyway."
    exit 1
fi

# ========== RUN TESTS ==========
print_section "Running Test Suite: $TEST_SUITE"

# Build pytest arguments
PYTEST_ARGS="-v --tb=short --device-ip $DEVICE_IP"

if [ "$VERBOSE" = true ]; then
    PYTEST_ARGS="$PYTEST_ARGS -s"
fi

# Select test markers based on suite
case $TEST_SUITE in
    smoke)
        echo "Running smoke tests (basic connectivity)..."
        PYTEST_ARGS="$PYTEST_ARGS tests/test_basic_connectivity.py tests/test_debug_connection.py"
        ;;
    integration)
        echo "Running integration tests..."
        PYTEST_ARGS="$PYTEST_ARGS -k 'not test_api_versioning' tests/"
        ;;
    full)
        echo "Running full test suite..."
        PYTEST_ARGS="$PYTEST_ARGS"
        ;;
    *)
        echo -e "${RED}Unknown test suite: $TEST_SUITE${NC}"
        exit 1
        ;;
esac

# Run pytest
echo -e "\nExecuting: pytest $PYTEST_ARGS"
echo -e "${BLUE}────────────────────────────────────────────────────────${NC}"

if pytest $PYTEST_ARGS; then
    TESTS_PASSED=1
else
    TESTS_FAILED=1
fi

# ========== FINAL SUMMARY ==========
print_section "Test Execution Summary"

if [ $TESTS_PASSED -eq 1 ]; then
    echo -e "${GREEN}✅ All tests in '$TEST_SUITE' suite passed!${NC}"
else
    echo -e "${RED}❌ Some tests in '$TEST_SUITE' suite failed${NC}"
fi

echo -e "\nEnvironment summary:"
echo -e "  Device IP: $DEVICE_IP"
echo -e "  Test suite: $TEST_SUITE"
echo -e "  Python: $PYTHON_VERSION"

# Deactivate virtual environment
deactivate 2>/dev/null || true

cd ../..

# Exit with appropriate code
if [ $TESTS_FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi