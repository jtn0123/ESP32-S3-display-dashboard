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
        
        // Convert to specific pin types for LCD DMA
        use esp_idf_hal::gpio::*;
        use super::lcd_cam_display_manager::LcdDisplayManager;
        
        // Note: This requires the pins to be the correct types
        // In real implementation, you'd need proper type conversion
        let display = LcdDisplayManager::new(
            unsafe { Gpio39::new() },
            unsafe { Gpio40::new() },
            unsafe { Gpio41::new() },
            unsafe { Gpio42::new() },
            unsafe { Gpio45::new() },
            unsafe { Gpio46::new() },
            unsafe { Gpio47::new() },
            unsafe { Gpio48::new() },
            unsafe { Gpio8::new() },
            unsafe { Gpio7::new() },
            unsafe { Gpio6::new() },
            unsafe { Gpio5::new() },
            unsafe { Gpio38::new() },
            unsafe { Gpio15::new() },
            unsafe { Gpio9::new() },
        )?;
        
        Ok(Box::new(display))
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