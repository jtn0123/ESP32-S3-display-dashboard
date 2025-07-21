/// GPIO DisplayManager - Wrapper for the default GPIO-based implementation
/// This module provides compatibility when ESP_LCD feature is enabled but we need to fall back to GPIO

use super::*;

// Re-export the default GPIO-based DisplayManager as GpioDisplayManager
pub type GpioDisplayManager = super::DisplayManager;