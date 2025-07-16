# ESP32-S3 Bootloader Flash Size Detection Investigation Report

## Executive Summary

After extensive research, the issue where ESP32-S3 T-Display with 16MB flash shows only 4MB is a well-documented problem in the ESP32 ecosystem. The root cause is a combination of bootloader caching in esp-idf-sys and the ESP-IDF bootloader's default behavior when flash detection fails.

## Key Findings

### 1. **Default 4MB Fallback Behavior**
- When ESP-IDF bootloader fails to detect flash size, it defaults to 4MB
- This is documented behavior in ESP-IDF: "If detection fails, a warning is printed and a default value of 4MB is used"
- This affects many ESP32-S3 boards with larger flash sizes

### 2. **esp-idf-sys Bootloader Caching Issue**
- **GitHub Issue #134**: esp-idf-sys doesn't rebuild when ESP_IDF_VERSION changes
- The bootloader is built once and cached, subsequent builds reuse the stale binary
- CMake considers the bootloader "up-to-date" and skips rebuilding
- Workaround: `cargo clean -p esp-idf-sys` forces complete rebuild

### 3. **Bootloader Location Complexity**
- **GitHub Issue #97**: Bootloader is stored in unpredictable paths with unique fingerprints
- Example: `target/xtensa-esp32s3-espidf/debug/build/esp-idf-sys-1d1212f75c9bfd7a/out/build/bootloader/bootloader.bin`
- This makes it difficult for tools to locate the correct bootloader

### 4. **Version Compatibility Issues**
- ESP-IDF v5.1 introduced breaking changes: "Bootloaders built from versions prior to V5.1 do not support CONFIG_ESP_SYSTEM_ESP32_SRAM1_REGION_AS_IRAM"
- Many users report bootloader compatibility issues when upgrading ESP-IDF versions
- The bootloader embeds CONFIG_ESPTOOLPY_FLASHSIZE in its header

### 5. **Tool-Specific Behaviors**
- `cargo-espflash` can detect esp-idf-sys builds and use the correct bootloader
- `espflash` lacks cargo integration and may use wrong bootloader
- Manual flash size specification often required: `--flash-size 16MB`

## Community-Reported Solutions

### 1. **Manual Flash Size Specification**
```bash
esptool.py write_flash --flash-size 16MB 0x0 bootloader.bin
espflash flash --flash-size 16mb [binary]
```

### 2. **Verify Flash Chip**
```bash
esptool.py --port /dev/ttyUSB0 flash_id
```

### 3. **Force Rebuild**
```bash
cargo clean -p esp-idf-sys
cargo clean  # Full clean sometimes required
rm -rf target/
```

### 4. **Platform Updates**
- PlatformIO users report success with espressif32 @ 6.0+ (fails with 5.4.0)
- Arduino-ESP32 users need latest bootloader via .factory.bin

### 5. **Custom Bootloader Build**
Many users resort to building bootloader separately with native ESP-IDF:
```bash
idf.py set-target esp32s3
idf.py menuconfig  # Set flash size to 16MB
idf.py bootloader
```

## Technical Details

### Flash Size Detection Process
1. ROM bootloader reads Second Stage Bootloader header
2. Second Stage Bootloader attempts SPI flash detection
3. On failure, defaults to 4MB (hardcoded in ESP-IDF)
4. Application uses bootloader's detected size

### Configuration Embedding
- Bootloader contains: CONFIG_ESPTOOLPY_FLASHMODE, CONFIG_ESPTOOLPY_FLASHFREQ, CONFIG_ESPTOOLPY_FLASHSIZE
- These are set at bootloader compile time, not runtime
- Changing sdkconfig.defaults doesn't affect existing bootloader binary

## Root Cause Analysis

1. **esp-idf-sys v0.36.1** builds bootloader with ESP-IDF v5.1-beta1
2. CMake caching prevents bootloader rebuild when configuration changes
3. ESP-IDF v5.1-beta1 bootloader may have flash detection issues
4. No mechanism to force bootloader rebuild in esp-idf-sys

## Recommendations

### Short-term Solutions
1. Use `cargo-espflash` instead of `espflash` for esp-idf-sys projects
2. Always specify `--flash-size 16mb` when flashing
3. Build custom bootloader with ESP-IDF v5.3+ if 16MB is critical

### Long-term Solutions
1. Wait for esp-idf-sys update that properly handles ESP_IDF_VERSION changes
2. Consider using native ESP-IDF for production deployments requiring specific flash sizes
3. Monitor esp-rs/esp-idf-sys repository for fixes to issues #97 and #134

## Impact Assessment

- **Current Impact**: Bootloader reports 4MB instead of 16MB
- **Functional Impact**: Minimal if app size < 4MB
- **Future Impact**: Will prevent OTA updates or larger partitions
- **Workaround Complexity**: Medium - requires manual bootloader management

## References

1. [esp-rs/esp-idf-sys Issue #134](https://github.com/esp-rs/esp-idf-sys/issues/134) - Not rebuilding when ESP_IDF_VERSION changes
2. [esp-rs/esp-idf-sys Issue #97](https://github.com/esp-rs/esp-idf-sys/issues/97) - Bootloader location issue
3. [ESP-IDF Documentation](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/bootloader.html) - Bootloader guide
4. [esptool Documentation](https://docs.espressif.com/projects/esptool/en/latest/esp32/esptool/flash-modes.html) - Flash detection behavior
5. [PlatformIO Issue #1232](https://github.com/platformio/platform-espressif32/issues/1232) - ESP32-S3 flash size issues

## Conclusion

This is a known limitation in the esp-idf-sys ecosystem affecting many users. The issue stems from multiple layers: ESP-IDF's default behavior, CMake caching, and esp-idf-sys's build process. While workarounds exist, a proper fix requires changes to esp-idf-sys's build system to properly track and rebuild the bootloader when configuration changes.