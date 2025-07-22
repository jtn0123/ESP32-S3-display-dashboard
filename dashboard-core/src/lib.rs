//! Dashboard Core - Hardware-independent logic for ESP32-S3 Display Dashboard
//! 
//! This crate contains business logic that can be tested on the host platform
//! without requiring ESP32 hardware.

pub mod config;
pub mod display_math;
pub mod color_utils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        assert_eq!(2 + 2, 4);
    }
}