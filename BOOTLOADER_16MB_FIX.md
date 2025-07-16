# Bootloader 16MB Flash Detection Issue

## Problem
ESP32-S3 bootloader reports 4MB instead of actual 16MB flash size, even after:
1. Setting `CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y` in sdkconfig.defaults
2. Adding `CONFIG_BOOTLOADER_FLASH_SIZE_16MB=y` 
3. Enabling `native` feature in esp-idf-sys
4. Clean rebuilds

## Root Cause
The bootloader is using a pre-compiled binary from ESP-IDF v5.1 that was built with 4MB configuration. The `native` feature IS compiling with CMake, but the bootloader configuration isn't inheriting the flash size setting.

## Current State
- Application sdkconfig: ✅ Shows 16MB correctly
- Bootloader binary: ❌ Still compiled for 4MB
- Flash tools: ✅ Use correct 16MB size
- Actual impact: Application limited to 4MB partition space

## What's Happening
1. esp-idf-sys with `native` feature DOES trigger CMake build
2. CMake files are generated in `target/*/build/esp-idf-sys-*/out/build/`
3. BUT the bootloader subdirectory has its own config that defaults to 4MB
4. The `CONFIG_BOOTLOADER_FLASH_SIZE_16MB` symbol may not exist in ESP-IDF v5.1

## Next Steps to Try
1. Check if ESP-IDF v5.1 uses different config symbols for bootloader flash size
2. Try Option B from the guide: Build bootloader manually with idf.py
3. Override the bootloader binary path in flash script
4. Check if there's a way to pass CMake variables directly to bootloader build