# ESP LCD Jumbled Display Fix Guide

## Status: Display Shows Output But Jumbled

This is great progress! The fact that you see something means:
- ✅ ESP LCD driver is working
- ✅ Display is receiving data
- ✅ Commands are being processed
- ❌ Data format/byte order is incorrect

## Applied Fix

Changed in `lcd_cam_esp_hal.rs`:
```rust
swap_color_bytes: 1, // Was 0 - now swapping bytes in RGB565
```

## If Still Jumbled, Try These:

### 1. Different MADCTL Value
The jumbled display might be due to wrong orientation. Try changing MADCTL in `lcd_cam_esp_hal.rs`:
```rust
// Current:
let madctl_data: [u8; 1] = [0x60];

// Try these alternatives:
let madctl_data: [u8; 1] = [0x00]; // Default portrait
let madctl_data: [u8; 1] = [0x70]; // Landscape + RGB  
let madctl_data: [u8; 1] = [0x68]; // Landscape + BGR
let madctl_data: [u8; 1] = [0xA0]; // Landscape rotated
```

### 2. Reverse Bit Order
If colors look inverted or strange, enable bit reversal:
```rust
reverse_color_bits: 1, // Try this with swap_color_bytes
```

### 3. Change Data Endianness
In the bus config structure:
```rust
data_endian: lcd_rgb_data_endian_t_LCD_RGB_DATA_ENDIAN_LITTLE, // Try LITTLE instead of BIG
```

### 4. Color Space (RGB vs BGR)
In panel config:
```rust
color_space: esp_lcd_rgb_panel_config_t__bindgen_ty_1_ESP_LCD_COLOR_SPACE_BGR, // Try BGR
```

## Quick Test Pattern

To verify the fix worked, the display should show:
1. During boot: Color rectangles (red, green, blue, white)
2. During tests: Various patterns and text
3. Version should show "v5.38-bytefix"

## Common Jumbled Patterns

- **Vertical lines/stripes**: Usually byte order issue (swap_color_bytes)
- **Wrong colors**: RGB/BGR issue or bit reversal needed
- **Mirrored/flipped**: MADCTL configuration
- **Offset/wrapped**: Window addressing or gap settings

## Next Debug Steps

1. Take a photo of the jumbled display - the pattern can help identify the issue
2. Note which test patterns are visible (colors, text, shapes)
3. Try the alternative configurations above one at a time

The fact that you're seeing output means we're very close to a working display!