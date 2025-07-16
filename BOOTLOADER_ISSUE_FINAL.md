# Bootloader 16MB Issue - Final Analysis

## Key Findings

1. **Correct config symbol**: `CONFIG_BOOTLOADER_SPI_FLASH_SIZE_16MB=y` (not `FLASH_SIZE`)
2. **Native build IS working**: Bootloader is being rebuilt (verified by timestamps)
3. **Root issue**: esp-idf-sys v0.36.1 is using ESP-IDF v5.1-beta1, not v5.3

## What Happened

1. Added correct bootloader config:
   ```
   CONFIG_BOOTLOADER_SPI_FLASH_SIZE_16MB=y
   CONFIG_BOOTLOADER_SPI_FLASH_SIZE_AUTO=n
   ```

2. Cleaned all caches and rebuilt - bootloader WAS recompiled

3. BUT: The bootloader still shows v5.1-beta1 because that's what esp-idf-sys v0.36.1 uses

## Current Status

- Bootloader: Compiled fresh but from ESP-IDF v5.1-beta1 
- Still reports 4MB due to v5.1 autodetect behavior
- The v5.1 bootloader may not respect the SPI_FLASH_SIZE config properly

## Solutions

### Option 1: Manual Bootloader (Recommended)
Build bootloader once with ESP-IDF v5.3+ and use it:
```bash
# In a temp directory
idf.py set-target esp32s3
idf.py menuconfig  # Set flash to 16MB
idf.py bootloader
# Copy build/bootloader/bootloader.bin to project
```

### Option 2: Update esp-idf-sys
Wait for newer esp-idf-sys that uses ESP-IDF v5.3+

### Option 3: Live with it
The 4MB limit only affects partition table boundaries. If your app partition is <4MB, it works fine.

## Impact
- Boot log shows 4MB (cosmetic)
- Partition table limited to 4MB boundary
- OTA updates may fail if trying to use >4MB partitions