# Versioning Guide

This project uses multiple version numbers for different purposes:

## Version Types

### 1. Display Version (`v4.33-rust`)
- **Location**: `src/version.rs` - `DISPLAY_VERSION`
- **Purpose**: User-facing version shown on device screens
- **Format**: `vX.YY-rust` where X is major, YY is minor
- **When to update**: For significant user-visible changes

### 2. Cargo Version (`0.1.2`)
- **Location**: `Cargo.toml` - `version`
- **Purpose**: Technical version for OTA updates and API
- **Format**: Semantic versioning `X.Y.Z`
- **When to update**: For any release, even minor fixes

## How Versions Are Used

- **Boot Screen**: Shows display version (e.g., "v4.33-rust")
- **Settings Screen**: Shows display version
- **API `/api/system`**: Returns Cargo version (e.g., "0.1.2")
- **OTA Updates**: Uses Cargo version to determine if update is needed

## Important Notes

1. **OTA Requirement**: The Cargo version MUST be incremented for OTA to work. The system prevents installing the same version.

2. **Version Synchronization**: When making a release:
   - Update `DISPLAY_VERSION` in `src/version.rs` for major changes
   - Always update `version` in `Cargo.toml` for any release

3. **Version History**:
   - v4.0-v4.30: Arduino implementation
   - v4.31+: Rust implementation
   - Cargo versions track technical releases

## Example Update Process

For a minor fix:
```toml
# Cargo.toml
version = "0.1.3"  # Increment from 0.1.2
```

For a major feature:
```rust
// src/version.rs
pub const DISPLAY_VERSION: &str = "v4.34-rust";  // Update from v4.33-rust
```
```toml
# Cargo.toml
version = "0.2.0"  # Major version bump
```

## Checking Current Version

- On device: Settings screen shows display version
- Via API: `curl http://device-ip/api/system` shows Cargo version
- OTA script: Shows both versions during update process