// Centralized version information

// Display version - this is what users see on screen
// Update this when making significant changes
pub const DISPLAY_VERSION: &str = "v5.91";

// Cargo package version from Cargo.toml
pub const CARGO_VERSION: &str = env!("CARGO_PKG_VERSION");

// Full version string including Cargo version
pub fn full_version() -> String {
    format!("{} ({})", DISPLAY_VERSION, CARGO_VERSION)
}

