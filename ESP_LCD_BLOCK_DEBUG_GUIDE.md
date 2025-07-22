# ESP LCD Block Debug Guide

## Issue: Blocky Display with 6 Readable Blocks

The display shows blocky output with intermittent readable sections, specifically "6 blocks that are readable".

## Applied Debug Tests

### 1. 6-Block Pattern Debug (`esp_lcd_block_debug.rs`)
Tests specifically for the 6-block pattern:
- 6-pixel wide columns with different colors
- 6-byte transfer tests  
- DMA descriptor boundary tests
- Pattern identification with colored blocks

### 2. Timing Debug (`esp_lcd_timing_debug.rs`)
Tests various timing configurations:
- Inter-operation delays (0-500us)
- Different transfer speeds
- Memory barriers and synchronization
- 6-block timing patterns

### 3. Comprehensive Block Debug
Tests multiple hypotheses:
- Single pixel writes
- Different block sizes (1, 2, 4, 8, 16, 32, 64 pixels)
- Transfer alignment patterns
- Transfer size optimization
- Memory barrier effects

## What to Look For

### During 6-Block Pattern Test:
1. **6-pixel columns** - If clean columns appear, it's a 6-pixel alignment issue
2. **Every 6th pixel white** - Confirms 6-pixel periodicity
3. **Colored blocks (R,G,B,W,Y,M)** - Note which colors are readable

### During Timing Test:
1. **Improvement with delays** - Timing is too fast
2. **Pattern changes with delays** - Setup/hold time issues
3. **Consistent blocks** - Not a timing issue

### During Block Debug:
1. **Regular pattern of blocks** - Transfer size issue
2. **Random corruption** - DMA timing issue
3. **Consistent 6-block pattern** - 6-byte alignment issue
4. **Improvement with barriers** - Cache coherency issue

## Possible Root Causes

1. **DMA Alignment** - Transfers splitting at 6-pixel boundaries
2. **I80 Bus Timing** - Every 6th transfer succeeds due to timing
3. **Buffer Alignment** - 6-byte boundary issues
4. **Clock Divider** - Creating 1-in-6 timing windows

## Version

Updated to `v5.39-blockdbg` to verify the debug code is running.

## Next Steps

Based on test results:
- If 6-pixel columns are clean → Fix pixel alignment
- If timing delays help → Adjust clock speed or add delays
- If specific transfer sizes work → Configure optimal DMA size
- If memory barriers help → Add proper synchronization