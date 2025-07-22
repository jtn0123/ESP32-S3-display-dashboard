# GDB initialization for ESP32-S3 Display Dashboard

# Connect to OpenOCD
target extended-remote :3333

# Helper functions
define reload
    mon reset halt
    load
    mon reset halt
end

define restart
    mon reset halt
    continue
end

# Display debugging helpers
define display_pins
    echo "=== Display Pin States ===\n"
    # GPIO15 - LCD_PWR
    x/1wx 0x60004044
    # GPIO38 - BL_EN (Backlight)
    x/1wx 0x600040CC
end

define display_regs
    echo "=== LCD_CAM Registers ===\n"
    # LCD_CAM_LCD_CLOCK_REG
    x/1wx 0x60041000
    # LCD_CAM_LCD_USER_REG
    x/1wx 0x60041004
    # LCD_CAM_LCD_OUT_REG
    x/1wx 0x60041008
    # LCD_CAM_LCD_CTRL_REG
    x/1wx 0x6004100C
end

define st7789_trace
    echo "=== ST7789 Command Trace ===\n"
    # Set breakpoint on command write function
    break esp_lcd_panel_io_tx_param
    commands
        printf "ST7789 CMD: 0x%02X\n", $a2
        continue
    end
end

# Useful memory regions
define memmap
    echo "=== ESP32-S3 Memory Map ===\n"
    echo "IRAM:     0x40370000 - 0x403E0000 (448KB)\n"
    echo "DRAM:     0x3FC88000 - 0x3FD00000 (480KB)\n"
    echo "PSRAM:    0x3C000000 - 0x3C800000 (8MB)\n"
    echo "Flash:    0x3C000000 - 0x3D000000 (16MB mapped)\n"
    echo "GPIO:     0x60004000 - 0x60004FFF\n"
    echo "LCD_CAM:  0x60041000 - 0x60041FFF\n"
end

# Stack trace with symbols
define bt_all
    thread apply all bt
end

# FreeRTOS task list
define tasks
    mon esp32 appimage_offset 0x10000
    mon esp32 sysview start file:///tmp/sysview.svdat
    info threads
end

# Initialize with helpful message
echo "ESP32-S3 Display Dashboard GDB initialized\n"
echo "Useful commands:\n"
echo "  reload       - Reset, load, and halt\n"
echo "  restart      - Reset and continue\n"
echo "  display_pins - Show display GPIO states\n"
echo "  display_regs - Show LCD_CAM registers\n"
echo "  st7789_trace - Trace ST7789 commands\n"
echo "  memmap       - Show memory regions\n"
echo "  tasks        - List FreeRTOS tasks\n"

# Load symbols
symbol-file target/xtensa-esp32s3-espidf/debug/esp32-s3-dashboard

# Set default breakpoints
break panic_abort
break __ubsan_handle_builtin_unreachable

# Ready
echo "\nReady for debugging!\n"