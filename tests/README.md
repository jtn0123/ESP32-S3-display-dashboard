# ESP32-S3 Display Dashboard - Test Suite

This directory contains all tests and testing utilities for the ESP32-S3 Display Dashboard project.

## Directory Structure

```
tests/
├── integration/          # Integration tests requiring device
│   ├── ota/             # OTA update tests
│   ├── stability/       # Stress and stability tests
│   └── web/             # Web server and API tests
├── unit/                # Unit tests (future)
├── python/              # Python test infrastructure
│   ├── tests/           # Individual test modules
│   ├── utils/           # Test utilities and helpers
│   ├── run_tests_simple.py    # Simple test runner
│   └── run_all_tests.py       # Comprehensive test runner
├── scripts/             # Test execution scripts
│   ├── run-tests.sh            # Main test runner
│   ├── run-python-tests.sh    # Python test runner
│   ├── run-rust-tests.sh      # Rust test runner
│   └── ...                     # Various test scripts
├── tools/               # Testing tools and utilities
│   ├── diagnostics/     # Diagnostic and debugging tools
│   ├── monitoring/      # Monitoring utilities
│   └── test_coverage_analysis.py  # Coverage analyzer
├── host-tests/          # Host-based Rust tests
└── phase2/              # Phase 2 test suite (legacy)
```

## Running Tests

### Quick Start

```bash
# Run all tests
./tests/scripts/run-tests.sh --all

# Run only Python tests
./tests/scripts/run-python-tests.sh

# Run only Rust tests
./tests/scripts/run-rust-tests.sh

# Run minimal smoke tests
./tests/scripts/run-minimal-tests.sh
```

### Python Tests

Python tests are organized by functionality:

- **Integration Tests**: Require a live ESP32 device
  - `integration/ota/` - OTA update testing
  - `integration/stability/` - Stress and freeze tests
  - `integration/web/` - Web server and metrics tests

- **Test Runners**:
  - `python/run_tests_simple.py` - Basic smoke tests
  - `python/run_all_tests.py` - Comprehensive test suite

### Rust Tests

- **Host Tests**: In `host-tests/` directory, run on development machine
- **Unit Tests**: Embedded in source files with `#[cfg(test)]`

Run with:
```bash
./tests/scripts/run-rust-tests.sh
```

### Test Tools

#### Coverage Analysis
```bash
python tests/tools/test_coverage_analysis.py
```

#### Monitoring Tools
- `tools/monitoring/telnet_monitor.py` - Monitor device via telnet
- `tools/monitoring/monitor_device_health.py` - Health monitoring

#### Diagnostic Tools
- `tools/diagnostics/debug_freeze.py` - Debug freeze issues
- `tools/diagnostics/analyze_freeze.py` - Analyze freeze patterns
- `tools/diagnostics/diagnose_boot_issues.py` - Boot diagnostics

## Device Configuration

Most integration tests require a live ESP32 device. Default IP: `10.27.27.201`

To use a different device:
```bash
./tests/scripts/run-python-tests.sh --device-ip 192.168.1.100
```

## Test Categories

### 1. Unit Tests
- Platform-independent logic
- No hardware dependencies
- Fast execution

### 2. Integration Tests
- Require ESP32 device
- Test full system behavior
- Network communication

### 3. Stress Tests
- Long-running stability tests
- Memory leak detection
- Performance regression

### 4. Smoke Tests
- Quick validation
- Basic functionality
- CI/CD friendly

## Writing New Tests

### Python Tests
Place new tests in appropriate category:
- `integration/` for device-dependent tests
- `python/tests/` for general test modules

### Rust Tests
- Add unit tests in source files
- Create host tests in `host-tests/` for platform-independent testing

## CI/CD Integration

The test suite is designed for CI/CD integration:

1. **Host Tests**: Run without hardware
2. **Mock Tests**: Use simulated device responses
3. **Integration Tests**: Run against test devices

## Dependencies

### Python
- pytest
- requests
- websocket-client
- asyncio

Install with:
```bash
cd tests/python
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows
pip install -r requirements.txt
```

### Rust
- Standard Rust toolchain
- ESP32 toolchain for device tests

## Troubleshooting

### Tests Can't Find Device
- Check device IP address
- Ensure device is on same network
- Verify WiFi credentials

### Python Import Errors
- Activate virtual environment
- Install requirements.txt

### Rust Build Errors
- Run `./scripts/fix-build.sh`
- Check toolchain with `./check-toolchain.sh`

## Test Results

Test results are typically output to console. For CI/CD:
- Python tests use pytest's XML output
- Rust tests use cargo's JSON output

## Contributing

When adding new tests:
1. Place in appropriate directory
2. Update this README if adding new categories
3. Ensure tests are runnable via scripts
4. Add to CI/CD pipeline if applicable