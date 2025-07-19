# LCD_CAM Implementation Debug Status

## Current State (v4.95-test-lcdcam)

### What We've Implemented:

1. **LCD_CAM Low-Level Bindings** (`lcd_cam_ll.rs`)
   - Basic LCD_CAM peripheral initialization
   - Clock configuration
   - GPIO matrix setup with correct signal indices
   - i8080 8-bit mode configuration

2. **DMA Support** (`lcd_cam_dma.rs`)
   - DMA descriptor management
   - GDMA channel configuration
   - Transfer state tracking

3. **Integrated Display Driver** (`lcd_cam_display.rs`)
   - ST7789 initialization sequence
   - Frame buffer in DRAM
   - Command/data transfer methods (incomplete)

4. **Test Programs**
   - `lcd_cam_test.rs`: Toggle color test using LCD_CAM
   - `simple_test.rs`: Basic GPIO toggle test for debugging

### Issues Found:

1. **Missing DC Pin Control**
   - The `write_command()` and `write_data()` methods don't actually control the DC pin
   - LCD_CAM needs to be configured to automatically control DC based on command/data mode

2. **Incomplete LCD_CAM Configuration**
   - Missing LCD_USER register configuration for command/data mode
   - Missing LCD_CMD_VAL register setup for command values
   - Missing proper DMA-to-LCD_CAM linkage

3. **Serial Port Communication Issues**
   - Unable to read serial output to verify test results
   - Device appears to be running but serial port is blocked

### Next Steps:

1. **Fix DC Pin Control**
   ```rust
   // In lcd_cam_ll.rs, add:
   const LCD_CMD_FLAG: u32 = 1 << 26;  // Command mode flag
   const LCD_DUMMY_CYCLELEN_SHIFT: u32 = 6;
   
   // Configure for automatic DC control
   pub unsafe fn set_command_mode(&mut self, is_command: bool) {
       let lcd_user_reg = LCD_CAM_LCD_USER_REG as *mut u32;
       let current = lcd_user_reg.read_volatile();
       if is_command {
           lcd_user_reg.write_volatile(current | LCD_CMD_FLAG);
       } else {
           lcd_user_reg.write_volatile(current & !LCD_CMD_FLAG);
       }
   }
   ```

2. **Complete LCD_CAM-DMA Integration**
   ```rust
   // Configure GDMA to connect to LCD_CAM peripheral
   const LCD_CAM_PERIPH_SEL: u32 = 5;  // LCD_CAM peripheral ID
   ```

3. **Add Proper Command/Data Sequencing**
   - Use LCD_CMD_VAL register for command bytes
   - Use DMA for data transfers
   - Ensure proper timing between command and data

4. **Debug Strategy**
   - First verify the simple GPIO test shows any display activity
   - Add visual feedback (like alternating backlight) to confirm code is running
   - Use logic analyzer on WR pin to verify timing

### Performance Expectations:

Based on our analysis:
- **Current GPIO bit-bang**: 10 FPS (1 MB/s)
- **Expected LCD_CAM**: 50-120 FPS (5-12 MB/s)
- **Theoretical max**: 240 FPS at 40MHz pixel clock

### How to Test:

1. **Physical Reset**: Unplug and replug the USB cable
2. **Flash the Device**: 
   ```bash
   ./scripts/flash.sh --release
   ```
3. **Monitor Output**:
   ```bash
   # Try these alternatives:
   screen /dev/cu.usbmodem101 115200
   # or
   minicom -D /dev/cu.usbmodem101 -b 115200
   # or
   cu -l /dev/cu.usbmodem101 -s 115200
   ```

4. **Expected Behavior**:
   - Simple test: Display backlight should turn on, pins should toggle
   - LCD_CAM test: Display should flash between red and cyan colors

### Files Modified:
- `/src/display/lcd_cam_ll.rs` - Added correct GPIO signal indices
- `/src/display/lcd_cam_display.rs` - Added SIG_GPIO_OUT_IDX
- `/src/display/lcd_cam_test.rs` - Added LCD power and backlight control
- `/src/display/simple_test.rs` - Created simple GPIO toggle test
- `/src/main.rs` - Set up to run simple test in debug mode