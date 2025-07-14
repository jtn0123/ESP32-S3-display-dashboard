# Rust FFI Display Driver Example

This example demonstrates how to use your existing C display driver from Rust using FFI (Foreign Function Interface).

## Quick Start

### 1. Copy Your Display Driver

Copy your working display code from `dashboard.ino` into `src/display_driver.c`, implementing the functions declared in `display_ffi.h`:

```c
// src/display_driver.c
#include "display_ffi.h"
#include "driver/gpio.h"

// Your existing pin definitions
#define LCD_D0 39
// ... etc

// Implement the FFI functions
void display_init(void) {
    // Your existing LCD initialization code
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << LCD_D0) | (1ULL << LCD_D1) /* ... */,
        .mode = GPIO_MODE_OUTPUT,
    };
    gpio_config(&io_conf);
    
    // Your ST7789 init sequence
    // ...
}

void display_draw_pixel(uint16_t x, uint16_t y, uint16_t color) {
    // Your existing pixel drawing code
}
```

### 2. Install Rust ESP32 Toolchain

```bash
# Install espup
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-installer.sh | sh

# Install ESP32 Rust toolchain
espup install

# Source the environment
source ~/export-esp.sh
```

### 3. Build and Flash

```bash
# Build the project
cargo build --release

# Flash to your ESP32-S3
espflash flash target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard-rust

# Monitor output
espflash monitor
```

## Project Structure

```
rust_ffi_display/
├── Cargo.toml          # Rust dependencies
├── build.rs            # Compile C code
├── display_ffi.h       # C header for FFI
├── src/
│   ├── main.rs         # Rust main application
│   ├── display.rs      # Safe Rust display wrapper
│   └── display_driver.c # Your C display code
```

## How It Works

1. **C Code**: Your existing display driver remains in C
2. **FFI Header**: Defines the C functions Rust will call
3. **Build Script**: Compiles C code during `cargo build`
4. **Rust Wrapper**: Safe Rust API around unsafe FFI calls
5. **Application**: Use display like any Rust struct

## Benefits

- **Immediate Results**: Your display works on day 1
- **Gradual Migration**: Port C functions to Rust over time
- **Safe Interface**: Rust wrapper prevents unsafe usage
- **Performance**: No overhead vs pure C

## Next Steps

Once this works, you can:

1. **Add OTA Support**
   ```rust
   mod ota;
   use ota::OtaUpdater;
   ```

2. **Port Functions to Rust**
   ```rust
   // Replace C function with Rust
   pub fn draw_pixel(&mut self, x: u16, y: u16, color: Color) {
       // Pure Rust implementation
       self.set_addr_window(x, y, x, y);
       self.write_data(&color.to_bytes());
   }
   ```

3. **Add DMA Support**
   ```rust
   use esp_idf_hal::dma::DmaChannel;
   // Hardware accelerated drawing
   ```

## Troubleshooting

### "IDF_PATH not set"
```bash
export IDF_PATH=~/.espressif/frameworks/esp-idf
```

### Linking errors
Make sure your C file implements all functions in `display_ffi.h`

### Display not working
- Check pin definitions match your hardware
- Verify initialization sequence
- Try adding delays between commands

## Migration Checklist

- [ ] C display driver compiles
- [ ] "Hello Rust" displays on screen
- [ ] Buttons work
- [ ] All screens render correctly
- [ ] Performance acceptable
- [ ] Add WiFi support
- [ ] Implement OTA
- [ ] Full feature parity

## Performance Tips

1. **Batch Operations**: Group multiple draws
2. **Dirty Rectangles**: Only update changed areas
3. **DMA Transfers**: Use hardware acceleration
4. **Const Generics**: Compile-time optimizations

```rust
// Example: Compile-time optimized colors
impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        // BGR565 conversion at compile time
        Color((((b as u16) & 0xF8) << 8) |
              (((g as u16) & 0xFC) << 3) |
              (((r as u16) & 0xF8) >> 3))
    }
}
```

## Ready to Start?

This FFI approach gets you running Rust quickly while keeping your working display code. It's the safest path to evaluate if Rust is right for your project!