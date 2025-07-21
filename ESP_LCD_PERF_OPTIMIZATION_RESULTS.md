# ESP_LCD PERF Optimization Results

## Key Discovery

Changing from SIZE to PERF optimization resolved the multiple DROM segments issue!

### What Changed

**Before (SIZE optimization):**
```
CONFIG_COMPILER_OPTIMIZATION_SIZE=y
Result: Multiple DROM segments in binary
```

**After (PERF optimization):**
```
CONFIG_COMPILER_OPTIMIZATION_PERF=y
Result: Single DROM segment!
```

### Binary Analysis

Using esptool.py image_info:
```
Segment 1: len 0x0b5c0 load 0x3c020020 file_offs 0x00000018 [DROM]
Segment 2: len 0x0335c load 0x3fc92a00 file_offs 0x0000b5e0 [BYTE_ACCESSIBLE,MEM_INTERNAL,DRAM]
Segment 3: len 0x016cc load 0x40374000 file_offs 0x0000e944 [MEM_INTERNAL,IRAM]
Segment 4: len 0x173c0 load 0x42000020 file_offs 0x00010018 [IROM]
Segment 5: len 0x0d2ec load 0x403756cc file_offs 0x000273e0 [MEM_INTERNAL,IRAM]
```

Only ONE DROM segment now - this should boot successfully!

## Why This Works

The SIZE optimization (-Os) appears to create code/data layouts that the linker splits into multiple DROM segments. The PERF optimization (-O2) generates a layout that fits in a single segment.

## Configuration Changes

In `sdkconfig.defaults`:
```diff
-# Optimize for size
-CONFIG_COMPILER_OPTIMIZATION_SIZE=y
-CONFIG_COMPILER_OPTIMIZATION_LEVEL_RELEASE=y
+# Optimize for performance (may help with single DROM segment)
+CONFIG_COMPILER_OPTIMIZATION_PERF=y
+# CONFIG_COMPILER_OPTIMIZATION_SIZE is not set
+# CONFIG_COMPILER_OPTIMIZATION_LEVEL_RELEASE is not set
```

## Status

- ✅ Build successful with ESP_LCD feature
- ✅ Single DROM segment achieved
- ✅ All four BREAK fixes implemented
- ⏳ Flashing to device...
- ⏳ Boot verification pending

## Version

v5.53-lcdPerf - Testing ESP_LCD with PERF optimization