/// Color manipulation utilities
/// 
/// This module provides color conversion and manipulation functions
/// that can be tested without hardware dependencies.

/// Convert RGB888 to RGB565
pub fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16 & 0xF8) >> 3)
}

/// Convert RGB565 to RGB888
pub fn rgb565_to_rgb888(color: u16) -> (u8, u8, u8) {
    let r = ((color >> 11) & 0x1F) as u8;
    let g = ((color >> 5) & 0x3F) as u8;
    let b = (color & 0x1F) as u8;
    
    // Expand to 8-bit by replicating upper bits
    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 2) | (g >> 4);
    let b8 = (b << 3) | (b >> 2);
    
    (r8, g8, b8)
}

/// Blend two RGB565 colors with alpha (0-255)
pub fn blend_rgb565(fg: u16, bg: u16, alpha: u8) -> u16 {
    if alpha == 255 {
        return fg;
    }
    if alpha == 0 {
        return bg;
    }
    
    let (fr, fg, fb) = rgb565_to_rgb888(fg);
    let (br, bg, bb) = rgb565_to_rgb888(bg);
    
    let alpha = alpha as u16;
    let inv_alpha = 255 - alpha;
    
    let r = ((fr as u16 * alpha + br as u16 * inv_alpha) / 255) as u8;
    let g = ((fg as u16 * alpha + bg as u16 * inv_alpha) / 255) as u8;
    let b = ((fb as u16 * alpha + bb as u16 * inv_alpha) / 255) as u8;
    
    rgb888_to_rgb565(r, g, b)
}

/// Adjust brightness of RGB565 color (0-100%)
pub fn adjust_brightness(color: u16, brightness: u8) -> u16 {
    let brightness = brightness.min(100) as u16;
    let (r, g, b) = rgb565_to_rgb888(color);
    
    let r = ((r as u16 * brightness) / 100) as u8;
    let g = ((g as u16 * brightness) / 100) as u8;
    let b = ((b as u16 * brightness) / 100) as u8;
    
    rgb888_to_rgb565(r, g, b)
}

/// Common colors in RGB565 format
pub mod colors {
    pub const BLACK: u16 = 0x0000;
    pub const WHITE: u16 = 0xFFFF;
    pub const RED: u16 = 0xF800;
    pub const GREEN: u16 = 0x07E0;
    pub const BLUE: u16 = 0x001F;
    pub const YELLOW: u16 = 0xFFE0;
    pub const CYAN: u16 = 0x07FF;
    pub const MAGENTA: u16 = 0xF81F;
    pub const GRAY: u16 = 0x8410;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_conversion_roundtrip() {
        // Test that conversion is reasonably reversible
        let original = (128, 64, 192);
        let rgb565 = rgb888_to_rgb565(original.0, original.1, original.2);
        let (r, g, b) = rgb565_to_rgb888(rgb565);
        
        // Due to bit reduction, we can't expect exact match
        assert!((r as i16 - original.0 as i16).abs() <= 8);
        assert!((g as i16 - original.1 as i16).abs() <= 4);
        assert!((b as i16 - original.2 as i16).abs() <= 8);
    }
    
    #[test]
    fn test_known_colors() {
        assert_eq!(rgb888_to_rgb565(255, 0, 0), colors::RED);
        assert_eq!(rgb888_to_rgb565(0, 255, 0), colors::GREEN);
        assert_eq!(rgb888_to_rgb565(0, 0, 255), colors::BLUE);
        assert_eq!(rgb888_to_rgb565(0, 0, 0), colors::BLACK);
        assert_eq!(rgb888_to_rgb565(255, 255, 255), colors::WHITE);
    }
    
    #[test]
    fn test_blend_extremes() {
        // Full alpha should return foreground
        assert_eq!(blend_rgb565(colors::RED, colors::BLUE, 255), colors::RED);
        
        // Zero alpha should return background
        assert_eq!(blend_rgb565(colors::RED, colors::BLUE, 0), colors::BLUE);
        
        // 50% blend should be purple-ish
        let blended = blend_rgb565(colors::RED, colors::BLUE, 128);
        let (r, g, b) = rgb565_to_rgb888(blended);
        assert!(r > 100 && b > 100 && g < 50);
    }
    
    #[test]
    fn test_brightness_adjustment() {
        // 50% brightness of white should be gray
        let dimmed = adjust_brightness(colors::WHITE, 50);
        let (r, g, b) = rgb565_to_rgb888(dimmed);
        assert!(r > 100 && r < 150);
        assert!(g > 100 && g < 150);
        assert!(b > 100 && b < 150);
        
        // 0% brightness should be black
        assert_eq!(adjust_brightness(colors::RED, 0), colors::BLACK);
        
        // 100% brightness should be unchanged
        assert_eq!(adjust_brightness(colors::GREEN, 100), colors::GREEN);
    }
    
    // Ensure color packing is correct
    #[test]
    fn test_rgb565_bit_packing() {
        let color = rgb888_to_rgb565(0b11111000, 0b11111100, 0b11111000);
        
        // Check individual components
        assert_eq!((color >> 11) & 0x1F, 0b11111);  // Red: 5 bits
        assert_eq!((color >> 5) & 0x3F, 0b111111);  // Green: 6 bits
        assert_eq!(color & 0x1F, 0b11111);          // Blue: 5 bits
    }
}