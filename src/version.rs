// Centralized version information

// Display version - this is what users see on screen
// Update this when making significant changes
pub const DISPLAY_VERSION: &str = "v4.56-rust";

// Cargo package version from Cargo.toml
pub const CARGO_VERSION: &str = env!("CARGO_PKG_VERSION");

// Full version string including Cargo version
pub fn full_version() -> String {
    format!("{} ({})", DISPLAY_VERSION, CARGO_VERSION)
}

// Version info string for logging
pub fn version_info() -> String {
    format!("Display: {}, Cargo: {}", DISPLAY_VERSION, CARGO_VERSION)
}

// Just the display version number (e.g., "4.33")
pub fn version_number() -> &'static str {
    if DISPLAY_VERSION.len() >= 5 {
        &DISPLAY_VERSION[1..5]  // Skip 'v' and '-rust'
    } else {
        "0.0"
    }
}