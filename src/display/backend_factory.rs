/// Factory for creating display backends based on feature flags
use super::display_backend::DisplayBackend;
use anyhow::Result;
use esp_idf_hal::gpio::AnyIOPin;
use log::info;

/// Create the appropriate display backend based on compile-time features
pub fn create_display_backend(
    d0: impl Into<AnyIOPin> + 'static,
    d1: impl Into<AnyIOPin> + 'static,
    d2: impl Into<AnyIOPin> + 'static,
    d3: impl Into<AnyIOPin> + 'static,
    d4: impl Into<AnyIOPin> + 'static,
    d5: impl Into<AnyIOPin> + 'static,
    d6: impl Into<AnyIOPin> + 'static,
    d7: impl Into<AnyIOPin> + 'static,
    wr: impl Into<AnyIOPin> + 'static,
    dc: impl Into<AnyIOPin> + 'static,
    cs: impl Into<AnyIOPin> + 'static,
    rst: impl Into<AnyIOPin> + 'static,
    backlight: impl Into<AnyIOPin> + 'static,
    lcd_power: impl Into<AnyIOPin> + 'static,
    rd: impl Into<AnyIOPin> + 'static,
) -> Result<Box<dyn DisplayBackend>> {
    #[cfg(feature = "lcd-dma")]
    {
        info!("Creating ESP LCD DMA display backend");
        
        // For LCD DMA, we need specific pin types which are not available from AnyIOPin
        // This is a limitation of the current architecture
        // The LCD DMA backend should be created directly in main.rs, not through the factory
        
        // For now, return an error indicating this limitation
        anyhow::bail!("LCD DMA backend cannot be created through the factory due to pin type constraints. Create LcdDisplayManager directly in main.rs instead.");
    }
    
    #[cfg(not(feature = "lcd-dma"))]
    {
        info!("Creating GPIO bit-bang display backend");
        use super::DisplayManager;
        
        let display = DisplayManager::new(
            d0, d1, d2, d3, d4, d5, d6, d7,
            wr, dc, cs, rst, backlight, lcd_power, rd,
        )?;
        
        Ok(Box::new(display))
    }
}

/// Get the name of the active display backend
pub fn get_backend_name() -> &'static str {
    #[cfg(feature = "lcd-dma")]
    return "ESP LCD DMA";
    
    #[cfg(not(feature = "lcd-dma"))]
    return "GPIO Bit-bang";
}

/// Get expected FPS for the active backend
pub fn get_expected_fps() -> f32 {
    #[cfg(feature = "lcd-dma")]
    return 40.0;
    
    #[cfg(not(feature = "lcd-dma"))]
    return 10.0;
}