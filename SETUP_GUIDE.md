# ESP32-S3 Dashboard Setup Guide

This guide documents the complete setup process for the ESP32-S3 Dashboard project, including common issues and their solutions.

## Prerequisites

- macOS (Intel or Apple Silicon)
- Rust installed via rustup
- Git
- Python 3.8+
- At least 10GB free disk space

## Step-by-Step Setup

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/ESP32-S3-Display-Dashboard.git
cd ESP32-S3-Display-Dashboard
```

### 2. Install ESP Toolchain

#### Option A: Using espup (Recommended)

```bash
# Install espup
cargo install espup --version 0.13.0  # Note: v0.15.1 has dependency conflicts

# Install ESP toolchain for ESP32-S3
espup install --targets esp32s3 --std

# Source the environment (add to your shell profile)
source ~/export-esp.sh
```

#### Option B: Manual Installation

```bash
# Download the toolchain
curl -LO https://github.com/esp-rs/rust-build/releases/download/v1.87.0.0/rust-1.87.0.0-aarch64-apple-darwin.tar.xz
tar -xf rust-1.87.0.0-aarch64-apple-darwin.tar.xz
cd rust-nightly-aarch64-apple-darwin
./install.sh --default-host aarch64-apple-darwin --prefix ~/.rustup/toolchains/esp
```

### 3. Set Up Environment Variables

Add to your `~/.zshrc` or `~/.bashrc`:

```bash
# ESP-IDF environment
source ~/export-esp.sh
export ESP_IDF_VERSION="v5.3.3"
export IDF_PATH="$HOME/.espressif/esp-idf/v5.3"
export ESP_IDF_TOOLS_INSTALL_DIR="global"
```

### 4. Build the Project

```bash
# First time setup
./setup-toolchain.sh  # If not using espup

# Build
./compile.sh          # Release build (default)
./compile.sh --debug  # Debug build
./compile.sh --clean  # Clean build
```

## Common Issues and Solutions

### Issue 1: Build Hangs at ~22 Compilations

**Symptoms:**
- Build stops at "Compiling regex-automata v0.4.9"
- No error messages, just hangs indefinitely

**Causes:**
1. VS Code or another IDE running `cargo check` in background
2. Stale lock files from interrupted builds
3. Corrupted cargo registry cache

**Solutions:**
```bash
# 1. Close VS Code or disable rust-analyzer
# 2. Kill all cargo processes
pkill -9 cargo

# 3. Clean all lock files
find . -name ".cargo-lock" -delete
rm -rf ~/.cargo/.package-cache*

# 4. Clear cargo caches
rm -rf ~/.cargo/registry/cache
rm -rf ~/.cargo/registry/index

# 5. Try building again
./compile.sh
```

### Issue 2: Partition Table Not Found

**Symptoms:**
```
FileNotFoundError: [Errno 2] No such file or directory: '.../partitions_16mb_ota.csv'
```

**Solution:**
Either use the default partition table or ensure custom partition file exists:

```toml
# In sdkconfig.defaults, use default:
CONFIG_PARTITION_TABLE_CUSTOM=n
CONFIG_PARTITION_TABLE_TWO_OTA=y

# Or if using custom, ensure file exists:
# partitions/partitions_16mb_ota.csv
```

### Issue 3: Frame Pointer Warning

**Symptoms:**
```
warning: Inherited flag "-fno-omit-frame-pointer" is not supported by the currently used CC
```

**Solution:**
Remove frame pointer flags from `.cargo/config.toml`:

```toml
rustflags = [
    # Don't use: "-C", "force-frame-pointers=yes",
]
```

### Issue 4: ESP Toolchain Not Recognized

**Symptoms:**
```
error: override toolchain 'esp' is not installed
```

**Solutions:**
1. Ensure espup installation completed successfully
2. Source the environment: `source ~/export-esp.sh`
3. Check toolchain exists: `ls ~/.rustup/toolchains/esp`

### Issue 5: espup Installation Fails

**Symptoms:**
```
error[E0308]: mismatched types (indicatif versions)
```

**Solution:**
Use older stable version:
```bash
cargo install espup --version 0.13.0
```

## Verification Steps

After setup, verify everything works:

```bash
# 1. Check ESP toolchain
rustup show | grep esp

# 2. Check environment
echo $IDF_PATH
echo $LIBCLANG_PATH

# 3. Test build
./compile.sh --clean

# 4. Check binary
ls -la target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

## Project Structure Notes

- **Toolchain Config**: `rust-toolchain.toml` specifies ESP toolchain
- **Build Config**: `.cargo/config.toml` sets target and build options
- **ESP-IDF Config**: `sdkconfig.defaults` configures ESP-IDF settings
- **Build Script**: `compile.sh` wraps cargo with proper environment

## Tips for Smooth Development

1. **Always close VS Code** before building to avoid lock conflicts
2. **Use compile.sh** instead of cargo directly - it sets up the environment
3. **Monitor build output** - if it hangs for >2 minutes, something's wrong
4. **Clean builds** help when switching between debug/release
5. **Restart your Mac** if you encounter persistent toolchain issues

## Troubleshooting Checklist

If build fails:

- [ ] Close all IDEs (VS Code, etc.)
- [ ] Kill all cargo processes: `pkill -9 cargo`
- [ ] Source environment: `source ~/export-esp.sh`
- [ ] Clean build: `./compile.sh --clean`
- [ ] Check disk space: `df -h .`
- [ ] Verify toolchain: `rustup show`
- [ ] Clear caches if needed (see Issue 1 solutions)

## Required Toolchain Versions

- ESP Rust: 1.87.0 or later
- ESP-IDF: v5.3.3 LTS
- espflash: v3.3.0 (NOT v4.x)
- espup: v0.13.0 (NOT v0.15.x)

## Additional Resources

- [ESP-RS Book](https://esp-rs.github.io/book/)
- [ESP-IDF Programming Guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/)
- [Project README](README.md) for feature documentation