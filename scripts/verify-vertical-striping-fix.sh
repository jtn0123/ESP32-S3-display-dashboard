#!/bin/bash
# Verify vertical striping fix implementation

echo "=== Verifying Vertical Striping Fix ==="
echo

# Check if gpio39_diagnostic module exists
echo "1. Checking GPIO39 diagnostic module..."
if [ -f "src/display/gpio39_diagnostic.rs" ]; then
    echo "✓ GPIO39 diagnostic module exists"
    grep -n "test_gpio39_stuck_high" src/display/gpio39_diagnostic.rs | head -5
else
    echo "✗ GPIO39 diagnostic module missing!"
fi
echo

# Check if D0 test pattern module exists
echo "2. Checking D0 test pattern module..."
if [ -f "src/display/d0_test_pattern.rs" ]; then
    echo "✓ D0 test pattern module exists"
    grep -n "draw_d0_test_pattern" src/display/d0_test_pattern.rs | head -5
else
    echo "✗ D0 test pattern module missing!"
fi
echo

# Check RASET fix
echo "3. Checking RASET calculation fix..."
grep -n "35 to 204" src/display/lcd_cam_esp_hal.rs
if [ $? -eq 0 ]; then
    echo "✓ RASET calculation fixed (35 + 169 = 204)"
else
    echo "✗ RASET calculation not fixed!"
fi
echo

# Check watchdog reset in render loop
echo "4. Checking watchdog reset in render loop..."
grep -A2 -B2 "render_time > Duration::from_millis" src/main.rs
if [ $? -eq 0 ]; then
    echo "✓ Watchdog reset added after heavy rendering"
else
    echo "✗ Watchdog reset not added!"
fi
echo

# Check module registration
echo "5. Checking module registration in mod.rs..."
grep "gpio39_diagnostic" src/display/mod.rs
grep "d0_test_pattern" src/display/mod.rs
echo

echo "=== Verification Complete ==="
echo
echo "To test the fixes:"
echo "1. Build with display tests: cargo build --release --features lcd-dma,display-tests"
echo "2. Flash and monitor: ./scripts/flash.sh"
echo "3. Look for GPIO39 diagnostic output in the logs"
echo "4. Watch for D0 test patterns on the display"