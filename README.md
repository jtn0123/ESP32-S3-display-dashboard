# ESP32-S3 Display Dashboard

A comprehensive dashboard solution for ESP32-S3 development boards with ST7789V displays, specifically tested on the LilyGo T-Display-S3.

## 🎯 Project Overview

This project solves the persistent display issues with the T-Display-S3 and provides a foundation for building interactive dashboards.

## 🚨 The Display Problem (SOLVED)

ESP32-S3 boards with ST7789V displays (like the LilyGo T-Display-S3) have a **critical display memory initialization issue**:
- Colors only partially fill the screen initially
- Persistent blue/pink border areas 
- Display works properly only after multiple refresh cycles
- Inconsistent behavior between power cycles

## ✅ The Solution

**Root Cause**: The ST7789V display controller requires **comprehensive memory initialization** before normal operation.

**Fix**: Initialize ALL display memory regions (480×320 pixels) with known data before drawing graphics.

```cpp
// CRITICAL: Initialize entire display memory once during startup
setDisplayArea(0, 0, 479, 319);  // Maximum area
writeCommand(0x2C);
for (int i = 0; i < 480 * 320; i++) {
    writeData(0x00);  // Initialize with black
    writeData(0x00);
}
```

## 🔧 Hardware Specifications

- **Display**: 1.9" ST7789V TFT LCD
- **Resolution**: 320×240 pixels (effective)
- **Interface**: 8-bit parallel (NOT SPI)
- **Memory Map**: 480×320 initialization required
- **Orientation**: 0x60 memory access control value

### Display Area & Color Mapping (VERIFIED)

**Color Channel Mapping**: RGB→BRG channel rotation
```cpp
// CORRECT COLOR MAPPING for T-Display-S3
#define RED        0x07FF  // Send YELLOW to get RED
#define GREEN      0xF81F  // Send CYAN to get GREEN  
#define BLUE       0xF8E0  // Send MAGENTA to get BLUE
#define YELLOW     0x001F  // Send GREEN to get YELLOW
#define CYAN       0xF800  // Send BLUE to get CYAN
#define MAGENTA    0x07E0  // Send RED to get MAGENTA
#define WHITE      0x0000  // Confirmed working
#define BLACK      0xFFFF  // Confirmed working
```

## 📱 Programming & Upload

### Quick Upload Script (Recommended) 🚀

The easiest way to upload is using the provided upload script:

```bash
# From project root directory
./upload.sh
```

This script automatically:
- ✅ Detects connected ESP32 board
- ✅ Handles correct file paths
- ✅ Compiles and uploads in one step
- ✅ Provides clear success/error feedback

### Alternative: Make Commands

```bash
make upload    # Compile and upload
make clean     # Clear build cache
make monitor   # Open serial monitor
make all       # Clean, compile, and upload
```

### Manual Arduino CLI Method

If you prefer manual control:

```bash
# 1. Check connected devices
arduino-cli board list

# 2. Navigate to dashboard directory
cd dashboard

# 3. Compile and upload (use . for current directory)
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 . && \
arduino-cli upload -p /dev/cu.usbmodem101 --fqbn esp32:esp32:lilygo_t_display_s3 .
```

**Key Benefits:**
- ✅ **No manual boot mode** - esptool handles reset sequence automatically
- ✅ **Native USB support** - ESP32-S3 auto-enters download mode
- ✅ **Reliable uploads** - Consistent programming without button combinations
- ✅ **Port detection** - Use `arduino-cli board list` to find correct port

**Upload Verification Protocol:**
- ⚠️ **CRITICAL**: Always verify uploads worked by making a visible change first
- 📋 **Best Practice**: When uploading color/logic changes, also make obvious visual changes (like adding "-TEST" to screen names)
- ✅ **Verification**: Upload → Check visible change appears → Confirms invisible changes also worked
- 🎯 **Why**: Upload success doesn't guarantee functional changes are active

### Traditional Arduino IDE Method

If using Arduino IDE:
1. **Board**: ESP32S3 Dev Module (or LilyGo T-Display-S3 if available)
2. **Port**: Select the USB Serial port (e.g., COM3 or /dev/cu.usbmodem101)
3. **Upload Speed**: 921600 or 460800 for faster programming
4. **Simply click Upload** - no manual boot mode needed

**Maximum Usable Display Area**: 300×168 pixels
```cpp
#define DISPLAY_X_START 10   // Left boundary
#define DISPLAY_Y_START 36   // Top boundary
#define DISPLAY_WIDTH   300  // Maximum width (83% expansion)
#define DISPLAY_HEIGHT  168  // Height with T/B expansion
// Coordinates: X=10-309, Y=36-203
```

### Pin Configuration
```cpp
#define LCD_POWER_ON 15  // Must be HIGH
#define LCD_BL       38  // Backlight
#define LCD_RES      5   // Reset
#define LCD_CS       6   // Chip Select  
#define LCD_DC       7   // Data/Command
#define LCD_WR       8   // Write
#define LCD_RD       9   // Read

// Data pins D0-D7
#define LCD_D0       39
#define LCD_D1       40
#define LCD_D2       41
#define LCD_D3       42
#define LCD_D4       45
#define LCD_D5       46
#define LCD_D6       47
#define LCD_D7       48
```

## 📁 Project Structure

```
├── src/
│   └── dashboard.ino           # Working dashboard implementation
├── enhanced_dashboard/
│   ├── enhanced_dashboard.ino  # Phase 1: Better Graphics demo
│   ├── graphics.h             # Enhanced drawing functions
│   ├── icons.h                # Icon library
│   └── themes.h               # Color schemes
├── color_verify/
│   └── color_verify.ino       # CRITICAL: Color mapping & screen area test
├── examples/
│   └── memory-initialization-test.ino  # Demonstrates the fix
├── docs/
│   ├── FINDINGS.md            # Technical research findings
│   └── ROADMAP.md             # Development roadmap
└── README.md                  # This file
```

## 🚀 Quick Start

### Fastest Method - Upload Script
1. **Clone this repository**
2. **Connect your T-Display-S3 via USB**
3. **Run the upload script**:
   ```bash
   cd ESP32-S3-Display-Dashboard
   ./upload.sh
   ```
4. **Watch the dashboard start with 5 screens** (System, Power, WiFi, Hardware, Settings)

### Alternative Methods

#### Using Make
```bash
cd ESP32-S3-Display-Dashboard
make upload      # Compile and upload in one step
make monitor     # View serial output
```

#### Arduino IDE Method
1. **Open `dashboard/dashboard.ino` in Arduino IDE**
2. **Select Board**: LilyGo T-Display-S3
3. **Select Port**: Your USB port
4. **Click Upload** - no boot mode needed!

#### Manual Arduino CLI
```bash
cd ESP32-S3-Display-Dashboard/dashboard
arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 .
arduino-cli upload -p /dev/cu.usbmodem101 --fqbn esp32:esp32:lilygo_t_display_s3 .
```

### Boot Mode Instructions
- **Enter Boot Mode**: Hold BOOT button, press and release RESET, then release BOOT
- **Exit Boot Mode**: Press RESET button once upload is complete
- **Auto-Reset**: The ESP32-S3 will automatically reset after upload completes

## 🎨 Enhancement Roadmap

### Phase 1: Better Graphics ⏳
- Rounded corners and gradients
- Icon and symbol library
- Improved color schemes
- Shadow effects

### Phase 2: Text Rendering 📋
- Bitmap font integration
- Multi-size text
- Text alignment and wrapping
- Custom font tools

### Phase 3: Interactive Features 📋
- Touch screen integration
- WiFi connection and status
- Real-time data display
- Settings persistence

### Phase 4: Dashboard Screens 📋
- Weather display with icons
- System monitoring graphs
- Network configuration
- Data logging

## 🔍 Key Findings

1. **Memory Persistence**: T-Display-S3 has non-volatile memory that retains content
2. **Initialization Critical**: Comprehensive memory initialization required before normal operation
3. **8-bit Parallel Interface**: Uses parallel communication, not SPI
4. **Area Mapping**: 480×320 memory space maps to 320×240 visible area
5. **Orientation Dependency**: Specific memory access control (0x60) required
6. **Color Channel Rotation**: RGB channels are rotated to BRG (RGB→BRG mapping required)
7. **Maximum Screen Area**: Usable area is 300×168 pixels centered at X=10-309, Y=36-203
8. **Display Boundaries**: L/R borders can be expanded 83% from center, T/B borders expandable by 4 pixels

## 📖 Usage Examples

### Basic Display Test
```cpp
#include "src/dashboard.ino"
// Upload and watch 4 rotating demo screens
```

### Memory Initialization Verification
```cpp
#include "examples/memory-initialization-test.ino"
// Demonstrates the one-time memory init fix
```

### Color Mapping & Screen Area Test
```cpp
#include "color_verify/color_verify.ino"
// CRITICAL: Tests correct color mapping and maximum usable screen area
// Verifies RGB→BRG channel rotation and 300×168 pixel boundaries
```

## 🤝 Contributing

This project provides a solid foundation for T-Display-S3 development. Contributions welcome for:
- Enhanced graphics and UI elements
- Touch screen integration
- Additional dashboard widgets
- Performance optimizations

## 📄 License

MIT License - see LICENSE file for details

## 🏆 Success Criteria

After implementing this solution:
- ✅ Colors fill entire screen immediately
- ✅ No persistent blue/pink areas  
- ✅ Consistent behavior across power cycles
- ✅ Fast color transitions
- ✅ Reliable display operation

**ESP32-S3 displays with ST7789V controllers now work perfectly and are ready for advanced dashboard development!**