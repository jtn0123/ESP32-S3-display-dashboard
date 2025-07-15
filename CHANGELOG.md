# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CHANGELOG.md for tracking project changes
- Comprehensive CI/CD pipeline with cargo audit security checks
- Binary size tracking and PR comments
- ESP32-S3 T-Display support with clean display driver abstraction
- Dependabot configuration for automated dependency updates
- GitHub Actions workflow for building and deploying API documentation
- `.cargo/config.toml` for improved developer experience with cargo aliases

### Changed
- Migrated from Arduino to Rust/ESP-IDF framework
- Separated unsafe LCD driver code into isolated modules (`src/display/lcd_bus.rs`)
- Pinned all dependencies to exact versions for reproducibility
- Moved legacy Makefile to `legacy/arduino-scripts/` directory
- Updated project name to `esp32-s3-dashboard` in Cargo.toml

### Security
- Implemented cargo audit in CI pipeline
- All dependencies version-pinned to prevent supply chain attacks
- Automated security updates via Dependabot

## [0.1.0] - 2025-01-15

### Added
- Initial Rust implementation of ESP32-S3 display dashboard
- Embassy async runtime integration
- Safe display driver abstraction over ESP-IDF LCD_CAM
- Network stack with TLS support
- Comprehensive GitHub Actions CI/CD
- Arduino legacy code preserved in `arduino/` directory

[Unreleased]: https://github.com/jtn0123/ESP32-S3-Display-Dashboard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jtn0123/ESP32-S3-Display-Dashboard/releases/tag/v0.1.0