# ESP-IDF Version Summary

## Current Situation

1. **esp-idf-sys v0.36.1 limitation**: This crate bundles its own ESP-IDF and doesn't properly use the ESP_IDF_VERSION environment variable for the bootloader build. It appears to use ESP-IDF v5.1-beta1 internally.

2. **Build vs Runtime**: 
   - The application builds with ESP-IDF v5.3 (we can see this in the build logs)
   - But the bootloader is still from ESP-IDF v5.1-beta1
   - This causes the 4MB flash detection issue

3. **Why setting ESP_IDF_VERSION didn't work**:
   - Setting ESP_IDF_VERSION="v5.3.3" successfully downloads ESP-IDF v5.3.3
   - The main application compiles against v5.3.3
   - BUT: esp-idf-sys doesn't rebuild the bootloader - it uses stale artifacts
   - The bootloader binary is cached and CMake thinks it's up-to-date

## Attempted Solutions

1. **Removed ESP-IDF cache**: `rm -rf ~/.cargo/esp-idf-sys/` - ESP-IDF v5.3.3 downloaded but bootloader still v5.1-beta1
2. **Cleaned all build artifacts**: `cargo clean` - Still using cached bootloader
3. **Removed bootloader build directory**: Still didn't force rebuild
4. **Added CONFIG_BOOTLOADER_REBUILD=y**: No effect on forcing rebuild
5. **Manually copied bootloader from build directory**: The build bootloader shows v5.3 but still flashes v5.1-beta1

## Root Cause

As identified by the user: esp-idf-sys generates bootloader once and CMake caches it. Subsequent builds skip bootloader rebuild even when ESP-IDF version changes.

## Solutions

### Option 1: Manual Bootloader (Recommended if 16MB is critical)
Build bootloader once with ESP-IDF v5.3+ and use it:
```bash
cd /tmp
git clone https://github.com/espressif/esp-idf.git
cd esp-idf
git checkout v5.3.3
./install.sh esp32s3
. ./export.sh
idf.py create-project bootloader-fix
cd bootloader-fix
idf.py set-target esp32s3
idf.py menuconfig  # Set Serial flasher config -> Flash size -> 16MB
idf.py bootloader
# Copy build/bootloader/bootloader.bin to project
```

### Option 2: Live with 4MB detection
Since your app is only 967KB, the 4MB limit doesn't affect functionality. The physical 16MB flash is still there, just the bootloader reports it incorrectly.

### Option 3: Wait for esp-idf-sys update
Future versions of esp-idf-sys may properly support ESP-IDF 5.3+ for bootloader builds.

## What Actually Happened

1. We successfully set ESP_IDF_VERSION="v5.3.3"
2. ESP-IDF v5.3.3 was downloaded to .embuild/espressif/esp-idf/v5.3.3
3. The main application compiled against ESP-IDF v5.3.3
4. But the bootloader binary is a stale artifact from the first build with v5.1-beta1
5. This is why the device still shows "ESP-IDF v5.1-beta1" and "SPI Flash Size : 4MB" at boot

## Recommendation

For now, continue with the current setup. The app works fine despite the bootloader showing 4MB. If you need the full 16MB for OTA or larger partitions later, build a custom bootloader with Option 1.