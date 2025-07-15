# ESP32-S3 Rust Partition Table Troubleshooting Guide

## Overview
This guide addresses the common "partitions.csv missing" error when building ESP-IDF Rust projects for ESP32-S3.

## Current Project Configuration

This project already has the necessary partition table configuration in place:

1. **partitions.csv** - Located in the project root directory
2. **sdkconfig.defaults** - Contains partition table configuration:
   ```
   CONFIG_PARTITION_TABLE_CUSTOM=y
   CONFIG_PARTITION_TABLE_FILENAME="partitions.csv"
   ```

## Common Solutions for "partitions.csv missing" Error

### 1. Verify File Location
The `partitions.csv` file MUST be in the project root directory, not in a subdirectory.

Current partition layout in this project:
```csv
# Name,   Type, SubType, Offset,  Size, Flags
nvs,      data, nvs,     0x9000,  0x6000,
phy_init, data, phy,     0xf000,  0x1000,
factory,  app,  factory, 0x10000, 0x3F0000,
```

### 2. Use cargo-espflash Instead of espflash
For ESP-IDF based Rust projects, use `cargo espflash` which automatically handles partition tables:

```bash
# Recommended method
cargo espflash flash --monitor

# Instead of
espflash flash target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

### 3. Manual Partition Table Specification
If you must use `espflash` directly, specify the partition table:

```bash
espflash flash \
  --bootloader target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-*/out/build/bootloader/bootloader.bin \
  --partition-table target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-*/out/build/partition_table/partition-table.bin \
  target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

### 4. Clean Build
Sometimes a clean build resolves partition table issues:

```bash
cargo clean
rm -rf target
cargo build --release
```

### 5. Check ESP-IDF Environment
Ensure ESP-IDF environment is properly set up:

```bash
# Source the ESP-IDF export script
source ~/export-esp.sh

# Verify environment variables
echo $IDF_PATH
echo $IDF_TOOLS_PATH
```

### 6. Verify build.rs Configuration
The `build.rs` file should include:

```rust
fn main() -> anyhow::Result<()> {
    embuild::espidf::sysenv::output();
    Ok(())
}
```

### 7. Check .cargo/config.toml
Ensure proper target configuration:

```toml
[build]
target = "xtensa-esp32s3-espidf"

[target.xtensa-esp32s3-espidf]
runner = "espflash flash --monitor"
```

## Partition Table Details

### Current Partition Layout
- **NVS (Non-Volatile Storage)**: 0x9000 - 24KB
- **PHY Init Data**: 0xf000 - 4KB  
- **Factory App**: 0x10000 - 4032KB (3.9MB)

### Important Notes
1. App partitions must be aligned to 0x10000 (64KB) boundaries
2. The bootloader expects the partition table at offset 0x8000
3. Total flash size is 8MB for this ESP32-S3 board

## Troubleshooting Steps

1. **Verify partition table exists**:
   ```bash
   ls -la partitions.csv
   ```

2. **Check partition table validity**:
   ```bash
   python $IDF_PATH/components/partition_table/gen_esp32part.py partitions.csv
   ```

3. **Build with verbose output**:
   ```bash
   cargo build --release -vv 2>&1 | grep -i partition
   ```

4. **Locate generated partition table**:
   ```bash
   find target -name "partition-table.bin" -o -name "partitions.bin"
   ```

## Alternative Flash Methods

### Using the provided flash.sh script:
```bash
./flash.sh
```

### Direct esptool.py method:
```bash
esptool.py --chip esp32s3 --port /dev/cu.usbmodem* \
  --baud 460800 write_flash --flash_mode dio --flash_size 8MB \
  0x0 target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-*/out/build/bootloader/bootloader.bin \
  0x8000 target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-*/out/build/partition_table/partition-table.bin \
  0x10000 target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard
```

## References
- [ESP-IDF Partition Tables Documentation](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/partition-tables.html)
- [The Rust on ESP Book - Troubleshooting](https://docs.esp-rs.org/book/troubleshooting/std.html)
- [esp-idf-sys Documentation](https://docs.esp-rs.org/esp-idf-sys/)