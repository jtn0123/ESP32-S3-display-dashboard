#!/bin/bash

echo "=== ESP LCD DMA Runtime Verification ==="
echo
echo "1. Checking build configuration..."

# Check default feature
echo -n "   Default feature in Cargo.toml: "
grep -A1 "^\[features\]" Cargo.toml | grep "default" | grep -o '"[^"]*"' | tr -d '"'

# Check if lcd-dma is in the default features
if grep -A1 "^\[features\]" Cargo.toml | grep "default.*lcd-dma" > /dev/null; then
    echo "   ✓ lcd-dma is the default feature"
else
    echo "   ✗ lcd-dma is NOT the default feature"
fi

echo
echo "2. Checking binary for ESP LCD symbols..."

# Check if the compiled binary contains ESP LCD symbols
if [ -f "target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard" ]; then
    # Check for ESP LCD specific symbols
    if strings target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard | grep -q "esp_lcd_panel_io_i80"; then
        echo "   ✓ Binary contains ESP LCD I80 symbols"
    else
        echo "   ✗ Binary does NOT contain ESP LCD I80 symbols"
    fi
    
    if strings target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard | grep -q "ESP LCD DMA"; then
        echo "   ✓ Binary contains 'ESP LCD DMA' backend name"
    else
        echo "   ✗ Binary does NOT contain 'ESP LCD DMA' backend name"
    fi
    
    if strings target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard | grep -q "LcdDisplayManager"; then
        echo "   ✓ Binary contains LcdDisplayManager symbols"
    else
        echo "   ✗ Binary does NOT contain LcdDisplayManager symbols"
    fi
else
    echo "   ✗ Binary not found - run ./compile.sh first"
fi

echo
echo "3. Checking source code integration..."

# Check if RUN_ESP_LCD_TEST is false
if grep -q "RUN_ESP_LCD_TEST: bool = false" src/main.rs; then
    echo "   ✓ ESP LCD test mode is DISABLED (running in production)"
else
    echo "   ✗ ESP LCD test mode might be enabled"
fi

# Check DisplayImpl usage
if grep -q "DisplayImpl::new" src/main.rs; then
    echo "   ✓ Main.rs uses DisplayImpl (type alias)"
else
    echo "   ✗ Main.rs does NOT use DisplayImpl"
fi

echo
echo "4. Runtime verification methods:"
echo "   a) Check serial output for:"
echo "      - 'Initializing ESP LCD DMA display...'"
echo "      - 'ESP LCD I80 bus created successfully'"
echo "      - 'Applied 6-block pattern fix'"
echo "      - 'ESP LCD display initialized successfully!'"
echo
echo "   b) Web API check (when device is running):"
echo "      curl http://<device-ip>/api/debug/display/state"
echo "      Should show: \"driver\": \"ESP_LCD_I80\""
echo
echo "   c) Telnet monitoring (when device is running):"
echo "      telnet <device-ip> 23"
echo "      Look for ESP LCD related messages"
echo
echo "5. Version check..."
grep "DISPLAY_VERSION" src/version.rs | grep -o '"[^"]*"' | tr -d '"'

echo
echo "=== DEFINITIVE TEST ==="
echo "The MOST reliable way to verify:"
echo "1. Flash the device: ./scripts/flash.sh"
echo "2. Monitor serial: espflash monitor"
echo "3. Look for these EXACT messages in order:"
echo "   - 'I (xxx) lcd_panel.io.i80: i80 bus created'"
echo "   - 'Detected 6-block pattern issue - applying targeted fix...'"
echo "   - 'ESP LCD display initialized successfully!'"
echo "4. The display should show v5.41-dma on screen"