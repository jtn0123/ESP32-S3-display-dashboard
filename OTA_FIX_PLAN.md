# ESP32-S3 OTA Fix Plan

## Current Issues and Root Causes

### Issue 1: App Size Too Large for Default OTA Partitions
**Problem**: Your app is 1.06MB but default ESP-IDF OTA partitions are only 1MB each (98.89% full)
**Root Cause**: Binary size inflation from Rust + esp-idf-sys dependencies
**Solution**: Use custom partition table with 1.5MB or 2MB partitions

### Issue 2: Custom Partition Tables Cause Boot Failure
**Problem**: Device shows "invalid magic byte" errors when using custom partition tables
**Root Cause**: When using custom partition tables, the app isn't being flashed to all partition offsets
- Factory partition at 0x10000 ✓ (flashed)
- OTA_0 partition at 0x190000 ✗ (not flashed)
- OTA_1 partition at 0x310000 ✗ (not flashed)
**Solution**: Flash app to factory partition only, or use esptool to flash all partitions

### Issue 3: OTA Endpoints Return 503/500 Errors
**Problem**: OTA upload fails with HTTP 503 (no partition) or 500 (write failed)
**Root Cause**: 
- 503: OTA manager can't find update partition when running from factory
- 500: App doesn't fit in partition during write
**Solution**: Already fixed by moving OTA to port 80 and proper error handling

### Issue 4: Monitor/Flash Script Issues
**Problem**: "Broken pipe" and "Failed to initialize input reader" errors
**Root Cause**: Terminal compatibility issues with espflash monitor
**Solution**: Added delay before monitor start and alternative monitoring methods

## Comprehensive Fix Plan

### Step 1: Create Proper Partition Table (1.5MB)
```csv
# partitions_ota_safe.csv - Safe OTA partition table for 1.06MB app
# Name,   Type, SubType, Offset,   Size,    Flags
nvs,      data, nvs,      0x9000,   0x5000,
otadata,  data, ota,      0xe000,   0x2000,
app0,     app,  ota_0,    0x10000,  0x180000,  # 1.5MB (no factory)
app1,     app,  ota_1,    0x190000, 0x180000,  # 1.5MB
spiffs,   data, spiffs,   0x310000, 0xCF0000,  # Remaining space
```

### Step 2: Fix Flash Process
The key issue is that espflash/esptool needs to flash the app to the correct partition offset. Options:

**Option A: Use Two OTA Partitions (No Factory)**
- Removes factory partition entirely
- First flash goes to ota_0
- Subsequent OTA alternates between ota_0 and ota_1

**Option B: Keep Factory + OTA**
- Flash factory partition via USB
- First OTA moves to ota_0
- Requires manual partition management

### Step 3: Implement Proper Flashing Script
```bash
#!/bin/bash
# Fixed flash script that handles OTA partitions correctly

# For initial flash with OTA support:
# 1. Flash bootloader at 0x0 (or 0x1000 for some chips)
# 2. Flash partition table at 0x8000
# 3. Flash app at first app partition offset (0x10000)
# 4. Let OTA handle subsequent updates

esptool.py --chip esp32s3 --port $PORT write_flash \
    --flash_mode dio \
    --flash_size 16MB \
    0x0 bootloader.bin \
    0x8000 partition-table.bin \
    0x10000 app.bin \
    0xe000 ota_data_initial.bin  # Important: Initialize OTA data
```

### Step 4: Binary Size Optimization
To fit in 1MB partitions (alternative approach):
1. Change Cargo.toml optimization:
   ```toml
   [profile.release]
   opt-level = "z"
   lto = "fat"
   codegen-units = 1  # Single codegen unit for better optimization
   strip = true
   ```

2. Remove unused features from dependencies
3. Use `cargo bloat` to identify large symbols
4. Consider splitting functionality into separate binaries

### Step 5: Test Procedure
1. Completely erase flash: `esptool.py erase_flash`
2. Flash with new partition table
3. Verify boot and partition detection
4. Test OTA update
5. Verify alternation between ota_0 and ota_1

## Recommended Solution Path

### Option 1: Quick Fix (Use Standard Partitions)
1. Optimize binary size to fit under 1MB
2. Use default ESP-IDF two_ota partition table
3. No custom partition table needed

### Option 2: Proper Fix (Custom Partitions)
1. Use the partitions_ota_safe.csv above
2. Modify flash.sh to properly initialize OTA data
3. Test complete OTA cycle

### Option 3: Alternative Approach
1. Use external OTA server with compression
2. Download compressed binary
3. Decompress and flash in chunks
4. Allows larger apps with smaller transfer size

## Key Learnings
1. **Always initialize OTA data partition** when using custom tables
2. **Partition alignment matters** - app partitions must be 64KB aligned
3. **Factory partition is optional** for OTA systems
4. **Test with erase_flash** to ensure clean state
5. **Monitor actual partition usage** not just file size

## Next Steps
1. Choose approach (Quick Fix vs Proper Fix)
2. Implement chosen solution
3. Test full OTA cycle
4. Document working configuration
5. Add size monitoring to build process