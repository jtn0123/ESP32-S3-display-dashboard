# CI/CD Setup for ESP32-S3 Display Dashboard

This document describes the continuous integration and continuous deployment setup for both Arduino and Rust implementations.

## GitHub Actions Workflows

### 1. Rust CI (`.github/workflows/rust-ci.yml`)

Runs on every push and pull request affecting the Rust codebase.

#### Jobs:

- **Check**: Validates the code compiles for ESP32-S3 target
- **Test**: Runs all unit tests
- **Format**: Ensures code follows Rust formatting standards
- **Clippy**: Static analysis for common mistakes and improvements
- **Build**: Builds both debug and release versions
- **Security**: Audits dependencies for known vulnerabilities

#### Key Features:
- Caches cargo dependencies for faster builds
- Uses ESP32-specific toolchain (xtensa-esp32s3-none-elf)
- Checks binary size to prevent bloat
- Runs security audits on dependencies

### 2. Arduino CI (`.github/workflows/arduino-ci.yml`)

Runs on every push and pull request affecting the Arduino codebase.

#### Jobs:

- **Compile**: Validates the Arduino sketch compiles
- Installs all required libraries
- Reports sketch size and memory usage

## Local CI Testing

### Running Rust CI Locally

```bash
# Format check
cd rust-dashboard
cargo fmt --check

# Clippy
cargo clippy -- -D warnings

# Tests
cargo test

# Security audit
cargo install cargo-audit
cargo audit
```

### Running Arduino CI Locally

```bash
# Install Arduino CLI
curl -fsSL https://raw.githubusercontent.com/arduino/arduino-cli/master/install.sh | sh

# Install ESP32 board
arduino-cli core update-index
arduino-cli core install esp32:esp32

# Compile
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 minimal_dashboard
```

## Build Optimization

### Rust Release Builds

The CI automatically builds release versions with optimizations:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for better optimization
strip = true        # Strip symbols
```

### Size Monitoring

Both workflows report binary/sketch sizes to track bloat:

- Rust: Uses `size` command on the compiled binary
- Arduino: Parses compiler output for sketch size

## Branch Protection

Recommended branch protection rules for `main`:

1. Require pull request reviews
2. Require status checks to pass:
   - Rust CI / Check
   - Rust CI / Test
   - Rust CI / Format
   - Arduino CI / Compile
3. Require branches to be up to date
4. Include administrators

## Deployment

### OTA Update Workflow (Future)

```yaml
name: Deploy OTA Update

on:
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build firmware
        # Build steps...
        
      - name: Upload to OTA server
        # Upload binary to server
        
      - name: Notify devices
        # Trigger OTA update
```

## Performance Metrics

The CI tracks several metrics:

1. **Build Time**: How long compilation takes
2. **Binary Size**: Final firmware size
3. **Test Coverage**: Percentage of code covered by tests
4. **Memory Usage**: RAM and Flash usage

## Troubleshooting

### Common Issues

1. **ESP toolchain not found**
   - Ensure `espup` is properly installed
   - Source the export script: `source $HOME/export-esp.sh`

2. **Clippy warnings**
   - Fix with: `cargo clippy --fix`
   - Or suppress specific warnings in code

3. **Format failures**
   - Auto-fix with: `cargo fmt`

4. **Arduino library not found**
   - Add to workflow: `arduino-cli lib install "LibraryName"`

## Future Enhancements

1. **Code Coverage**: Add coverage reporting with `cargo-tarpaulin`
2. **Benchmarks**: Run performance benchmarks on CI
3. **Documentation**: Auto-generate and publish API docs
4. **Release Automation**: Auto-create releases with changelogs
5. **Hardware-in-Loop Testing**: Test on actual ESP32-S3 hardware