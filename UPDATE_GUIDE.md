# ESP32-S3 Dashboard Update Guide

## Build & Update Workflow

### First-Time Setup (After Cache Loss)
⚠️ The first build after losing the `.embuild` cache will take 2-3 hours. This is normal and only happens once.

### Normal Build Process
```bash
# Standard build (takes ~2 minutes after cache is built)
./compile.sh

# Clean build (only if needed)
./compile.sh --clean
```

### USB Flash (Device Connected via USB)
```bash
# Full flash with erase (recommended for issues)
./scripts/flash.sh

# Quick flash without erase
./scripts/flash.sh --no-erase

# Flash and monitor serial output
./scripts/flash.sh --monitor
```

### OTA Update (Over WiFi)
```bash
# Find devices on network
./scripts/ota.sh find

# Update specific device
./scripts/ota.sh <IP_ADDRESS>

# Update by hostname
./scripts/ota.sh esp32-dashboard.local
```

## Important Notes

1. **Build Cache**: The `.embuild` directory contains critical build cache. Never delete it unless absolutely necessary.

2. **Binary Location**: The compiled binary is at:
   - `target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard`

3. **OTA Requirements**:
   - Device must be on same network
   - Binary must be built first
   - Device must be running OTA-capable firmware

4. **Version Tracking**: Always update version in `src/version.rs` before building to verify updates worked.

## Preventing Issues

- ✅ DO: Use `cargo clean` if you need to clean build artifacts
- ❌ DON'T: Delete `.embuild` directory (contains ESP-IDF cache)
- ❌ DON'T: Delete `~/.cargo/registry` (contains Rust crate cache)

## Build Speed Reference

| Build Type | Cache Status | Time |
|------------|--------------|------|
| Normal build | Cached | ~2 minutes |
| Clean build | Cached | ~5 minutes |
| First build | No cache | ~2-3 hours |
| Incremental | Cached | ~30 seconds |

## Troubleshooting

If builds are slow:
1. Check `.embuild` directory exists
2. Ensure ESP-IDF path is set correctly
3. Don't interrupt first-time cache builds