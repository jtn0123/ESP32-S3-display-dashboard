# ESP32-S3 Dashboard Architecture

## Hybrid std/no_std Approach

This project uses a **hybrid approach** combining ESP-IDF (std) for system features with selective no_std modules for testing and modularity.

### Why This Architecture?

1. **ESP-IDF (std) for main application**:
   - Provides WiFi, OTA, NVS storage out of the box
   - Mature, battle-tested networking stack
   - Easy HTTP server for web configuration
   - Standard Rust error handling with Result/anyhow

2. **no_std for library modules**:
   - Allows testing on host without ESP toolchain
   - Forces clean abstractions
   - Enables future embedded use cases
   - Smaller code size for critical paths

### Code Organization

```
src/
├── main.rs          # std - Entry point, uses ESP-IDF
├── lib.rs           # no_std - Library modules for testing
├── config.rs        # std - Uses serde_json, NVS
├── sensors.rs       # std - Uses ESP-IDF ADC HAL
├── animation/       # no_std - Pure algorithms, testable
│   └── mod.rs      
├── display/         # std - Hardware driver
│   ├── mod.rs      # Uses ESP-IDF GPIO HAL
│   ├── colors.rs   # no_std compatible
│   └── font5x7.rs  # no_std compatible
├── network/         # std - All networking code
│   ├── wifi.rs     # ESP-IDF WiFi
│   ├── ota.rs      # ESP-IDF OTA
│   └── web_server.rs # ESP-IDF HTTP
├── system/          # std - System features
│   ├── button.rs   # GPIO with ESP-IDF
│   ├── power.rs    # Power management
│   └── storage.rs  # NVS persistence
└── ui/             # std - UI rendering
    └── mod.rs      
```

### Dependencies Explained

```toml
# Core ESP-IDF support (provides std)
esp-idf-sys = { version = "0.35", features = ["binstart"] }
esp-idf-svc = "0.49"  # Service wrappers (WiFi, HTTP, etc.)
esp-idf-hal = "0.44"  # Hardware abstraction (GPIO, SPI, etc.)

# Async runtime (currently unused but ready)
embassy-executor = "0.6"  # For future async tasks
embassy-time = "0.3"      # Time utilities
embassy-sync = "0.6"      # Sync primitives

# Still useful in std environment
heapless = "0.8"  # Fixed-size collections
anyhow = "1.0"    # Error handling
serde_json = "1.0" # JSON config parsing
```

### Build Configuration

The project builds for the **xtensa-esp32s3-espidf** target, which includes:
- Full std library support
- Heap allocation (malloc/free)
- Threading (pthreads)
- Network stack
- File system

### Memory Layout

```
Flash: 4MB
├── Bootloader     (0x1000)
├── Partition Table (0x8000)
├── NVS            (0x9000)
├── PHY Init       (0xF000)
├── Application    (0x10000) <- Our code here
└── OTA Partition  (0x210000)

RAM: 512KB
├── ESP-IDF heap   (~200KB)
├── Stack          (~32KB)
└── Static data    (~50KB)
```

### Future Considerations

1. **Migration to pure no_std** (if needed):
   - Replace ESP-IDF WiFi with esp-wifi crate
   - Use Embassy for all async operations
   - Implement own HTTP server
   - Trade maturity for smaller size

2. **Keep hybrid approach** (recommended):
   - Best of both worlds
   - Faster time to market
   - Proven reliability
   - Easy to maintain

### Testing Strategy

- **Unit tests**: Run on host with `cargo test --lib`
- **Integration tests**: Require hardware or QEMU
- **no_std modules**: Test algorithms without hardware
- **std modules**: Test with ESP-IDF mocks where possible

### Performance Characteristics

- **Boot time**: ~1.5s (ESP-IDF initialization)
- **Binary size**: ~500KB (includes WiFi stack)
- **Free heap**: ~150KB after init
- **Display refresh**: 30 FPS (limited by SPI, not CPU)

This architecture provides a pragmatic balance between embedded constraints and development velocity.