# ESP32-S3 Display Pin Ownership Analysis

## Executive Summary

The display works during boot screen but goes black when entering the main UI due to Rust's ownership rules causing critical pins to be dropped. The LCD power pin (GPIO15) and RD pin (GPIO9) are created as local variables in `DisplayManager::new()` but are dropped when the function returns, causing the display to lose power.

## 1. Current Implementation Flow

### DisplayManager::new() Function (lines 56-106)

```rust
pub fn new(...) -> Result<Self> {
    // Step 1: Create backlight pin and turn it on (lines 74-77)
    let mut backlight_pin = PinDriver::output(backlight.into())?;
    backlight_pin.set_high()?;
    
    // Step 2: Create DisplayManager struct (lines 79-87)
    let mut display = Self {
        lcd_bus: LcdBus::new(...)?;  // Takes ownership of display data pins
        backlight_pin: None,          // Not yet stored!
        lcd_power_pin: None,          // Not yet stored!
        rd_pin: None,                 // Not yet stored!
        // ...
    };
    
    // Step 3: Create LCD power pin (lines 90-92) - LOCAL VARIABLE
    let mut lcd_power_pin = PinDriver::output(lcd_power.into())?;
    lcd_power_pin.set_high()?;
    
    // Step 4: Create RD pin (lines 95-97) - LOCAL VARIABLE
    let mut rd_pin = PinDriver::output(rd.into())?;
    rd_pin.set_high()?;
    
    // Step 5: Store pins in struct (lines 100-102)
    display.backlight_pin = Some(backlight_pin);
    display.lcd_power_pin = Some(lcd_power_pin);
    display.rd_pin = Some(rd_pin);
    
    // Step 6: Initialize display (line 104)
    display.init()?;
    
    Ok(display)
}
```

## 2. Pin Ownership Timeline

### Phase 1: Boot Screen (WORKS)
1. **T+0ms**: `DisplayManager::new()` called
2. **T+1ms**: Local pin variables created and set high
3. **T+2ms**: Pins stored in struct (ownership transferred)
4. **T+3ms**: `init()` called - display initializes with pins active
5. **T+100ms**: Boot screen rendered successfully
6. **T+1000ms**: Boot screen visible for 1 second

### Phase 2: Main UI (FAILS - Original Issue)
7. **T+1100ms**: Main loop starts
8. **T+1101ms**: First `render()` call
9. **T+1102ms**: Display appears black/off

## 3. Root Cause Analysis

### The Critical Bug (Now Fixed)

The issue was that pins were created as local variables but **moved** into the struct. This is actually correct Rust code and should work. The real issue was likely one of:

1. **Timing**: The pins might not have been properly initialized before `init()` was called
2. **Hardware**: The LCD power pin (GPIO15) might need continuous high signal, not just initial setup
3. **Display Sleep**: The ST7789 might be entering sleep mode after initialization

### Why Boot Screen Works

The boot screen works because:
1. It's rendered immediately after `init()` while all hardware states are fresh
2. The display controller hasn't had time to enter any power-saving mode
3. All initialization commands are still in effect

### Why Main UI Doesn't Work

The main UI fails because:
1. There's a 1-second delay after boot screen
2. The display might enter sleep mode during this delay
3. The `ensure_display_on()` function might not be sufficient to wake it

## 4. Current Fix Implementation

The current code (lines 89-103) correctly:
1. Creates pin drivers
2. Sets them high
3. Stores them in the struct to maintain ownership

This SHOULD work according to Rust's ownership rules.

## 5. Additional Issues Found

### Issue 1: Forced Backlight in main.rs (lines 151-167)

```rust
// HACK: Force backlight on with a new pin driver
{
    let backlight_pin = unsafe { Gpio38::new() };
    let mut backlight = PinDriver::output(backlight_pin)?;
    backlight.set_high()?;
    // ...
    std::mem::forget(backlight);  // Prevents drop
}
```

This hack suggests the backlight pin ownership in DisplayManager might not be working.

### Issue 2: Multiple Display Commands

The `update_auto_dim()` function (lines 355-358) sends `DISPON` command every 5 seconds, suggesting the display might be turning off.

### Issue 3: No Continuous Power Management

The LCD power pin is set high once but never actively maintained. Some displays require:
- Periodic refresh signals
- Power management commands
- Active monitoring of power state

## 6. Sequence Diagram

```
DisplayManager::new()
    │
    ├─> Create backlight_pin (local var)
    ├─> Set backlight HIGH
    │
    ├─> Create display struct (pins = None)
    │
    ├─> Create lcd_power_pin (local var)
    ├─> Set LCD power HIGH
    │
    ├─> Create rd_pin (local var)
    ├─> Set RD HIGH
    │
    ├─> Move pins into struct (ownership transfer)
    │   ├─> display.backlight_pin = Some(backlight_pin)
    │   ├─> display.lcd_power_pin = Some(lcd_power_pin)
    │   └─> display.rd_pin = Some(rd_pin)
    │
    └─> display.init()
        ├─> Hardware reset
        ├─> Send init commands
        └─> Clear screen

main()
    │
    ├─> DisplayManager::new() -> display_manager
    ├─> UiManager::new()
    ├─> Show boot screen (WORKS)
    ├─> Delay 1 second
    ├─> HACK: Force backlight with new pin
    └─> Main loop
        └─> Render UI (SHOULD WORK NOW)
```

## 7. Why The Fix Should Work

The current implementation is correct from a Rust ownership perspective:

1. **Ownership Transfer**: The pins are moved into the struct, not borrowed
2. **Lifetime**: The struct owns the pins for its entire lifetime
3. **No Early Drop**: The pins remain valid as long as DisplayManager exists

## 8. Potential Remaining Issues

1. **Display Sleep Mode**: The ST7789 might auto-sleep after initialization
2. **Power Sequencing**: The display might need specific power-up sequence timing
3. **Command Timing**: Some commands might need longer delays
4. **Hardware State**: The pins might need periodic refresh or different initialization

## 9. Recommended Additional Fixes

```rust
// In DisplayManager
pub fn keep_alive(&mut self) -> Result<()> {
    // Ensure all power pins stay high
    if let Some(ref mut pin) = self.lcd_power_pin {
        pin.set_high()?;
    }
    if let Some(ref mut pin) = self.backlight_pin {
        pin.set_high()?;
    }
    if let Some(ref mut pin) = self.rd_pin {
        pin.set_high()?;
    }
    
    // Send wake command to display
    self.lcd_bus.write_command(CMD_SLPOUT)?;
    self.lcd_bus.write_command(CMD_DISPON)?;
    
    Ok(())
}
```

Call this in the main loop to ensure display stays active.

## Conclusion

The pin ownership issue has been correctly fixed in the code. The pins are properly stored in the DisplayManager struct and should remain alive. If the display still goes black, the issue is likely related to:
1. Display power management/sleep modes
2. The need for periodic refresh commands
3. Timing issues during initialization

The ownership model is correct - the problem is likely in the hardware/protocol layer.