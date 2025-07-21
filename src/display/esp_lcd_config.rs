/// Configuration constants for ESP LCD optimization
use esp_idf_sys::*;

/// LCD clock speeds for performance testing
pub enum LcdClockSpeed {
    Mhz17,  // Conservative - reference speed
    Mhz24,  // Moderate increase
    Mhz30,  // Good balance
    Mhz40,  // High performance
    Mhz48,  // Maximum reliable
}

impl LcdClockSpeed {
    pub fn as_hz(&self) -> u32 {
        match self {
            LcdClockSpeed::Mhz17 => 17_000_000,
            LcdClockSpeed::Mhz24 => 24_000_000,
            LcdClockSpeed::Mhz30 => 30_000_000,
            LcdClockSpeed::Mhz40 => 40_000_000,
            LcdClockSpeed::Mhz48 => 48_000_000,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            LcdClockSpeed::Mhz17 => "17 MHz",
            LcdClockSpeed::Mhz24 => "24 MHz",
            LcdClockSpeed::Mhz30 => "30 MHz",
            LcdClockSpeed::Mhz40 => "40 MHz",
            LcdClockSpeed::Mhz48 => "48 MHz",
        }
    }
}

/// Transfer size configurations
pub enum TransferSize {
    Lines50,   // 16KB - Lower latency
    Lines100,  // 32KB - Balanced (default)
    Lines150,  // 48KB - Higher throughput
    Lines200,  // 64KB - Maximum
}

impl TransferSize {
    pub fn lines(&self) -> usize {
        match self {
            TransferSize::Lines50 => 50,
            TransferSize::Lines100 => 100,
            TransferSize::Lines150 => 150,
            TransferSize::Lines200 => 200,
        }
    }
    
    pub fn bytes(&self, width: usize) -> usize {
        self.lines() * width * 2 // 2 bytes per pixel (RGB565)
    }
}

/// Double buffer configuration
pub struct DoubleBufferConfig {
    pub enabled: bool,
    pub buffer_size: usize,
    pub use_psram: bool,
}

impl Default for DoubleBufferConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_size: 320 * 100 * 2, // 100 lines
            use_psram: false, // Use IRAM for best performance
        }
    }
}

/// Optimized LCD configuration
pub struct OptimizedLcdConfig {
    pub clock_speed: LcdClockSpeed,
    pub transfer_size: TransferSize,
    pub queue_depth: usize,
    pub double_buffer: DoubleBufferConfig,
}

impl Default for OptimizedLcdConfig {
    fn default() -> Self {
        Self {
            clock_speed: LcdClockSpeed::Mhz17,
            transfer_size: TransferSize::Lines100,
            queue_depth: 10,
            double_buffer: DoubleBufferConfig::default(),
        }
    }
}

impl OptimizedLcdConfig {
    /// Get configuration optimized for performance
    pub fn performance() -> Self {
        Self {
            clock_speed: LcdClockSpeed::Mhz40,
            transfer_size: TransferSize::Lines150,
            queue_depth: 12,
            double_buffer: DoubleBufferConfig {
                enabled: true,
                buffer_size: 320 * 150 * 2,
                use_psram: false,
            },
        }
    }
    
    /// Get configuration for maximum FPS
    pub fn max_fps() -> Self {
        Self {
            clock_speed: LcdClockSpeed::Mhz48,
            transfer_size: TransferSize::Lines100, // Smaller for lower latency
            queue_depth: 15,
            double_buffer: DoubleBufferConfig {
                enabled: true,
                buffer_size: 320 * 100 * 2,
                use_psram: false,
            },
        }
    }
    
    /// Get conservative configuration for stability
    pub fn conservative() -> Self {
        Self {
            clock_speed: LcdClockSpeed::Mhz24,
            transfer_size: TransferSize::Lines50,
            queue_depth: 8,
            double_buffer: DoubleBufferConfig::default(),
        }
    }
}