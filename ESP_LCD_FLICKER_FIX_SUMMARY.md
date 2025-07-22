# ESP LCD Display Flicker Fix Summary

## Version: v5.42-smooth

### What Was Fixed

1. **Clock Speed Increased**: 5 MHz → 24 MHz
   - 5 MHz was causing visible flickering due to slow refresh
   - 24 MHz provides smoother updates without stability issues

2. **Frame Rate Limiting Enabled**: 60 FPS cap
   - Prevents the main loop from running uncapped at 19k FPS
   - Provides consistent frame timing

3. **Transfer Size Optimized**: 40 lines per transfer
   - Larger transfers reduce overhead
   - Aligned to 64-byte boundaries for DMA efficiency

4. **Synchronous Transfers**: Queue depth = 1
   - Prevents tearing between frames
   - Ensures each frame completes before the next starts

### Expected Improvements

- **Smoother Display Updates**: 4.8x faster pixel clock
- **No Tearing**: Synchronous frame updates
- **Consistent Timing**: 60 FPS frame cap
- **Better Performance**: ~40 FPS theoretical maximum

### If Still Flickering

1. **Power Supply**: Check if USB power is stable
2. **Try Different Clock Speeds**:
   - 20 MHz (more conservative)
   - 30 MHz (faster but may be less stable)
   - 40 MHz (maximum recommended)
3. **Cable Connection**: Ensure display ribbon cable is secure
4. **Temperature**: Monitor if device is overheating

### Diagnostic Messages

When running v5.42-smooth, you'll see:
```
=== Applying ESP LCD Anti-Flicker Configuration ===
Clock speed set to 24 MHz (was 5 MHz)
Transaction queue depth set to 1 (synchronous mode)
Transfer size set to 13696 bytes (40 lines per transfer)
```

The display should show "v5.42-smooth" on screen.