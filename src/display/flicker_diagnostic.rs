/// Runtime diagnostic for display flicker issues
use log::{info, warn};

pub fn log_display_performance_metrics() {
    info!("=== Display Performance Diagnostic ===");
    
    // Check if we're using the optimized settings
    #[cfg(feature = "lcd-dma")]
    {
        info!("Display Driver: ESP LCD DMA (Hardware Accelerated)");
        info!("Expected Performance:");
        info!("  - Clock Speed: 24 MHz (was 5 MHz)");
        info!("  - Queue Depth: 1 (synchronous transfers)");
        info!("  - Transfer Size: ~6.8KB (20 lines per transfer)");
        info!("  - Frame Rate Cap: 60 FPS");
        info!("  - Theoretical Max FPS: ~40 FPS");
        
        warn!("If still flickering:");
        warn!("  1. Check power supply - unstable power causes flicker");
        warn!("  2. Try clock speeds: 20, 24, 30, or 40 MHz");
        warn!("  3. Ensure display cable is properly connected");
        warn!("  4. Monitor temperature - overheating can cause instability");
    }
    
    #[cfg(not(feature = "lcd-dma"))]
    {
        warn!("Display Driver: GPIO Bit-banging (Software)");
        warn!("This mode has inherent flickering due to slow GPIO updates");
        warn!("Maximum achievable: ~10 FPS");
    }
}