# ESP LCD Viewport Fix - Summary

## The Problem
The display was blank because we were drawing to off-screen memory. The T-Display-S3's ST7789 controller has a 35-pixel row offset that wasn't being accounted for.

## The Solution
Used ESP-IDF's built-in panel configuration functions instead of custom initialization:

```rust
// Critical configuration after panel init
esp_lcd_panel_set_gap(panel, 0, 35);      // Map visible window (35 pixel Y offset)
esp_lcd_panel_swap_xy(panel, true);       // 170×320 → 320×170 landscape  
esp_lcd_panel_mirror(panel, true, false); // Make (0,0) top-left
esp_lcd_panel_invert_color(panel, true);  // Fix inverted colors
```

## Key Changes Made

1. **Correct dimensions**: 170×320 portrait → 320×170 landscape after swap
2. **Set gap**: Y offset of 35 pixels to map visible area
3. **No custom init needed**: ESP-IDF driver handles everything with proper config
4. **Simplified code**: Removed unnecessary custom initialization

## Expected Results
- Display should show test pattern (black → colors → rectangles → text)
- Performance should be ~30+ FPS with DMA
- No more blank screen!

## Testing
Run: `./run-esp-lcd-test.sh`

Watch for:
- "ESP LCD INITIALIZED" in logs
- Display turning black then showing colors
- FPS measurement >25