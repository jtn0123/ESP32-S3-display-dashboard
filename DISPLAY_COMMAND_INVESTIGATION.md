# ST7789 Display Command Investigation Report

## Problem Summary
The Rust implementation calls `ensure_display_on()` which sends SLPOUT (0x11) and DISPON (0x29) commands repeatedly, potentially causing display malfunction. The Arduino implementation never sends these commands after initial display setup.

## Key Findings

### 1. Command Locations in Rust Code

#### Main Loop Call (src/main.rs:149)
```rust
// Force ensure display and backlight are on
info!("Ensuring display stays on...");
display_manager.ensure_display_on()?;
```

#### Implementation (src/display/mod.rs:372-378)
```rust
pub fn ensure_display_on(&mut self) -> Result<()> {
    // Send SLPOUT and DISPON to ensure display doesn't sleep
    self.lcd_bus.write_command(CMD_SLPOUT)?;  // 0x11
    FreeRtos::delay_ms(5);
    self.lcd_bus.write_command(CMD_DISPON)?;   // 0x29
    Ok(())
}
```

#### During Initialization (src/display/mod.rs)
- Line 130: `self.lcd_bus.write_command(CMD_SLPOUT)?;` with 120ms delay
- Line 181: `self.lcd_bus.write_command(CMD_DISPON)?;` with 100ms delay
- Line 357: Another `self.lcd_bus.write_command(CMD_DISPON)?;` in toggle_display

#### Note in flush() method (src/display/mod.rs:367)
```rust
// Removed ensure_display_on() - was causing display issues
// Display should stay on from initialization
```

### 2. Arduino Implementation Comparison

The Arduino code (dashboard.ino) only sends these commands ONCE during initialization:

```c
void initDisplay() {
    // ... hardware setup ...
    writeCommand(0x01);  // Software reset
    delay(120);
    writeCommand(0x11);  // Sleep out (SLPOUT) - ONLY CALLED ONCE
    delay(120);
    // ... other init commands ...
    writeCommand(0x29);  // Display on (DISPON) - ONLY CALLED ONCE
    delay(100);
    // ... rest of init ...
}
```

**Critical difference**: Arduino NEVER calls these commands again after initialization.

### 3. ST7789 Command Behavior Analysis

#### SLPOUT (0x11) - Sleep Out Command
- Wakes the display from sleep mode
- Requires 120ms delay after execution (per most implementations)
- Enables normal display operation and memory access

#### DISPON (0x29) - Display On Command
- Turns on the display output
- Should be called after all configuration is complete
- Typically requires 100ms delay

### 4. Potential Issues with Repeated Commands

1. **State Machine Disruption**: The ST7789 has an internal state machine. Sending SLPOUT when already out of sleep might:
   - Reset internal timings
   - Cause undefined behavior
   - Disrupt ongoing display operations

2. **Memory Access Interruption**: During pixel write operations, sending SLPOUT could:
   - Reset the memory pointer
   - Clear internal buffers
   - Interrupt DMA transfers

3. **Power Sequencing**: Repeated power state commands might:
   - Cause voltage fluctuations
   - Reset display parameters
   - Create visual artifacts

4. **Timing Violations**: The 5ms delay in Rust vs 120ms standard might be insufficient

### 5. Evidence from Code Comments

The Rust code already has a comment acknowledging the issue:
- "Removed ensure_display_on() - was causing display issues"
- "Display should stay on from initialization"

This suggests the problem was previously identified but not fully resolved.

### 6. Recommended Solution

1. **Remove the ensure_display_on() call from main.rs:149**
   - The display should remain on after initialization
   - ST7789 doesn't automatically sleep without explicit SLPIN command

2. **Remove or disable the ensure_display_on() method entirely**
   - Prevents accidental future use
   - Matches Arduino behavior

3. **If display sleep is a concern**, implement proper sleep/wake cycle:
   ```rust
   pub fn sleep_display(&mut self) -> Result<()> {
       self.lcd_bus.write_command(CMD_SLPIN)?;  // 0x10
       FreeRtos::delay_ms(120);
       Ok(())
   }
   
   pub fn wake_display(&mut self) -> Result<()> {
       self.lcd_bus.write_command(CMD_SLPOUT)?;  // 0x11
       FreeRtos::delay_ms(120);
       self.lcd_bus.write_command(CMD_DISPON)?;  // 0x29
       FreeRtos::delay_ms(100);
       Ok(())
   }
   ```

4. **Only call wake_display() if sleep_display() was explicitly called**

## Conclusion

The repeated SLPOUT/DISPON commands in the Rust implementation are likely causing display corruption by disrupting the ST7789's internal state machine during normal operation. The Arduino implementation's approach of only sending these commands once during initialization is the correct pattern to follow.

The issue appears to stem from a misunderstanding that the display might spontaneously enter sleep mode, when in fact it will only sleep if explicitly commanded to do so with SLPIN (0x10).