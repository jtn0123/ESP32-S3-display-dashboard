/// Runtime check to verify which display driver is active
use log::info;

pub fn log_active_driver() {
    #[cfg(feature = "lcd-dma")]
    {
        info!("=== DISPLAY DRIVER: ESP LCD DMA (Hardware Accelerated) ===");
        info!("Using LcdDisplayManager with ESP-IDF I80 bus");
        info!("DMA transfers enabled for maximum performance");
    }
    
    #[cfg(not(feature = "lcd-dma"))]
    {
        info!("=== DISPLAY DRIVER: GPIO Bit-banging (Software) ===");
        info!("Using DisplayManager with manual GPIO control");
        info!("No DMA acceleration");
    }
}