# ESP32-S3 Display Fix Analysis Report

## Problem Timeline

### Initial Issue
- **Symptom**: Static display for 0.5 seconds, then black screen
- **Device**: ESP32-S3 T-Display (LilyGO)
- **Display**: ST7789 controller, 170×320 pixels (physical), 240×320 RAM

### Progressive Fixes and Results

#### 1. Static Display Fix
**Problem**: Random static/noise on display at boot
**Root Cause**: Uninitialized data pins sending garbage data
**Fix Applied**:
```rust
// In lcd_bus.rs - Clear all data pins before initialization
for pin in &mut bus.data_pins {
    pin.set_low()?;
}
// Set CS low permanently (matching Arduino)
bus.cs.set_low()?;
```
**Result**: ✅ Static eliminated

#### 2. Partial Display Issue
**Problem**: "Progress bar then nothing" → "White screen on half the screen"
**Root Cause**: Incorrect display boundaries
**Fix Applied**:
```rust
// Changed from incorrect 170×320 to verified Arduino boundaries
const DISPLAY_X_START: u16 = 10;   // Left boundary 
const DISPLAY_Y_START: u16 = 36;   // Top boundary
const DISPLAY_WIDTH: u16 = 300;    // Maximum visible width
const DISPLAY_HEIGHT: u16 = 168;   // Maximum visible height
```
**Result**: ✅ Full screen coverage achieved

#### 3. Display Initialization
**Problem**: Display not initializing properly
**Root Cause**: Missing ST7789 initialization commands
**Fix Applied**:
- Added comprehensive initialization sequence from LilyGO reference
- Added 480×320 controller memory clear (to remove factory patterns)
- Added proper timing delays between commands
- Added test pattern with white fill to verify display works

**Result**: ✅ Display shows white test screen

#### 4. Backlight Turning Off
**Problem**: "No backlight after white screen"
**Root Cause**: Rust dropping the backlight GPIO pin after initialization
**Fix Applied**:
```rust
// Added to DisplayManager struct
backlight_pin: Option<PinDriver<'static, AnyIOPin, Output>>,

// Store pin to keep it alive
display.backlight_pin = Some(backlight_pin);
```
**Result**: ✅ Backlight should persist (v4.2)

## Current Status

### What Works
1. Display initializes without static
2. Full 300×168 visible area is accessible
3. White test pattern displays correctly
4. Backlight turns on initially

### Remaining Issues

#### 1. Bootloader Flash Size Detection
**Problem**: Bootloader reports "SPI Flash Size : 4MB" instead of 16MB
**Impact**: 
- May cause issues with OTA updates
- Could affect partition table usage
- Limits available flash storage

**Attempted Solutions**:
1. Set `CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y` in sdkconfig.defaults ✅
2. Clean rebuild multiple times ✅
3. Direct esptool.py flash with `--flash_size 16MB` ✅
4. Set ESP_IDF_SDKCONFIG_DEFAULTS environment variable ✅

**Result**: Bootloader still compiled with 4MB setting

**Root Cause Analysis**:
- The bootloader binary appears to be pre-compiled with ESP-IDF v5.1
- The sdkconfig settings aren't being applied to bootloader compilation
- Need to force bootloader recompilation with correct settings

#### 2. Post-White-Screen Behavior
**Unknown**: What happens after the white test screen
- Does the main UI render?
- Does the backlight actually stay on with the fix?
- Are there any crashes or panics?

## Technical Details

### Display Configuration
- **Controller**: ST7789 (240×320 RAM)
- **Physical Display**: 170×320 pixels
- **Visible Area**: 300×168 pixels (with offsets)
- **Interface**: 8-bit parallel I8080
- **Power**: GPIO15 (LCD power), GPIO38 (backlight)

### Key Differences from Arduino Implementation
1. **Memory Management**: Rust drops GPIO pins when they go out of scope
2. **Timing**: GPIO operations in Rust are fast enough without explicit delays
3. **Initialization**: Required more comprehensive command sequence

### Files Modified
1. `/src/display/mod.rs` - Main display management
2. `/src/display/lcd_bus.rs` - Low-level parallel bus
3. `/src/main.rs` - Version update and power sequence
4. `/sdkconfig.defaults` - Flash size configuration

## Next Steps

### For Bootloader Issue
1. Force bootloader recompilation:
   - Delete `.embuild` directory
   - Set `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=n`
   - Try `CONFIG_ESPTOOLPY_FLASHSIZE_DETECT=n`

2. Alternative: Use custom partition table with explicit 16MB layout

3. Check if there's a way to specify bootloader configuration separately

### For Display Verification
1. Add debug output after white screen test
2. Implement actual UI rendering
3. Add heartbeat indicator to verify system is running

## Version History
- v4.0: Initial Rust implementation
- v4.1: Display boundaries and initialization fixes
- v4.2: Backlight persistence fix