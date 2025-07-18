# OTA Setup Guide

## Current Status

Your app size is 1.06MB, which is too large for the default ESP-IDF OTA partitions (1MB each). This causes OTA updates to fail with "WriteFailed" errors.

## Solution Options

### Option 1: Reduce App Size (Recommended)
Reduce your app below 1MB by:
- Using `--release` builds (already doing this)
- Enabling size optimizations in Cargo.toml
- Removing unused dependencies
- Disabling unused features

### Option 2: Use Larger Partitions
The included `partitions_ota_1_5mb.csv` has 1.5MB partitions, but custom partition tables seem to cause boot issues on your device.

### Option 3: Use External OTA Server
Instead of embedding OTA in the device, use an external update mechanism.

## Current Workaround

For now, continue using USB flashing with:
```bash
./flash.sh
```

## Testing OTA

Once you get app size below 1MB:
```bash
# Flash with standard OTA partitions
espflash flash --flash-size 16mb --partition-table ~/.espressif/esp-idf/v5.3/components/partition_table/partitions_two_ota.csv target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard --port /dev/cu.usbmodem101

# Then test OTA
./ota.sh find
./ota.sh <device-ip>
```