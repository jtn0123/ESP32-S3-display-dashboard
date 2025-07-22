# ESP LCD 6-Block Pattern Analysis

## Observed Issue
Display shows blocky output with "6 blocks that are readable" - this is a very specific pattern that indicates a systematic issue.

## Analysis

### 6-Pixel/6-Byte Pattern
The fact that every 6th block is readable suggests:
1. **DMA Transfer Alignment**: The I80 DMA is transferring data in chunks that don't align with the display's expectations
2. **6-Byte Boundary**: In RGB565 (2 bytes per pixel), 6 bytes = 3 pixels. This could be related to:
   - DMA descriptor alignment requirements
   - I80 bus timing creating a 1-in-6 success rate
   - Buffer alignment in memory

### Current Configuration
From the debug output:
- Display driver: ESP_LCD_I80
- Bus width: 8-bit
- Clock: 17 MHz (could be too fast)
- PSRAM align: 64 bytes
- SRAM align: 4 bytes
- Max transfer: Based on config

## Likely Root Causes

### 1. DMA Descriptor Alignment (Most Likely)
The ESP32-S3 DMA has specific alignment requirements:
- DMA descriptors must be aligned to 4-byte boundaries
- Buffer data should be aligned to cache line size (32 bytes for PSRAM)
- The 6-byte pattern suggests misalignment between buffer and DMA expectations

### 2. Clock Speed Too High
At 17 MHz with 8-bit bus:
- Each byte transfer takes ~59ns
- The display might not be keeping up
- Every 6th transfer succeeds when timing aligns

### 3. FIFO Underrun
The LCD_CAM peripheral has a FIFO that might be:
- Underrunning due to DMA not keeping up
- Creating periodic gaps in data

## Recommended Fixes

### Fix 1: Adjust DMA Alignment
```rust
// In esp_lcd_config.rs
buffer_size: match pixel_count {
    count if count < 64 => BufferSize::Pixels64,
    count if count < 128 => BufferSize::Pixels128,
    count if count < 256 => BufferSize::Pixels256,
    // Force larger buffers for better alignment
    _ => BufferSize::Pixels512,
}

// Ensure transfer size is multiple of 6 pixels (12 bytes)
max_transfer_bytes: ((width * 6 * 2 + 63) / 64) * 64, // Round up to 64-byte boundary
```

### Fix 2: Reduce Clock Speed
```rust
// Try much slower clock first
clock_speed: LcdClockSpeed::Mhz5, // or even Mhz2
```

### Fix 3: Add DMA Wait States
```rust
// In I80 config
pclk_hz: 5_000_000, // 5 MHz instead of 17 MHz
trans_queue_depth: 1, // Reduce queue depth to ensure completion
```

## Test Strategy

1. **Slow Clock Test**: Reduce to 5 MHz or 2 MHz
2. **Alignment Test**: Force all transfers to be multiples of 12 bytes (6 pixels)
3. **Single Transfer Test**: Use queue_depth=1 to ensure each transfer completes

## Next Steps

1. Create `esp_lcd_6block_fix.rs` with targeted fixes
2. Test each hypothesis systematically
3. The readable blocks pattern is diagnostic gold - it tells us exactly what's wrong