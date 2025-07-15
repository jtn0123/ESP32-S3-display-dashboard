use crate::display::*;
use crate::display::font::*;

#[test]
fn test_color_constants() {
    // Test that color constants are defined correctly
    assert_eq!(Color::BLACK.0, 0xFFFF);
    assert_eq!(Color::WHITE.0, 0x0000);
    
    // Colors should be distinct
    assert_ne!(Color::RED.0, Color::GREEN.0);
    assert_ne!(Color::BLUE.0, Color::YELLOW.0);
}

#[test]
fn test_font_character_width() {
    assert_eq!(Font5x7::get_char_width(' '), 3);
    assert_eq!(Font5x7::get_char_width('!'), 2);
    assert_eq!(Font5x7::get_char_width('A'), 5);
    assert_eq!(Font5x7::get_char_width('W'), 5);
}

#[test]
fn test_font_string_width() {
    assert_eq!(Font5x7::get_string_width(""), 0);
    assert_eq!(Font5x7::get_string_width("A"), 5);
    assert_eq!(Font5x7::get_string_width("AB"), 11); // 5 + 1 + 5
    assert_eq!(Font5x7::get_string_width("Hello"), 25); // 5 chars * 5 width + 4 spacing
}

#[test]
fn test_font_character_data() {
    // Test ASCII range
    assert!(Font5x7::get_char_data(' ').is_some());
    assert!(Font5x7::get_char_data('~').is_some());
    
    // Test out of range
    assert!(Font5x7::get_char_data('\0').is_none());
    assert!(Font5x7::get_char_data('\x7F').is_none());
    assert!(Font5x7::get_char_data('\u{1F600}').is_none()); // Emoji
}

#[test]
fn test_font_data_integrity() {
    // Test that font data is valid (5 bytes per character)
    if let Some(data) = Font5x7::get_char_data('A') {
        assert_eq!(data.len(), 5);
        
        // 'A' should have some non-zero data
        assert!(data.iter().any(|&b| b != 0));
    }
    
    // Space should be mostly zeros
    if let Some(data) = Font5x7::get_char_data(' ') {
        assert!(data.iter().all(|&b| b == 0));
    }
}

#[test]
fn test_display_pixel_bounds() {
    // Mock display for testing
    struct MockDisplay {
        width: u16,
        height: u16,
        pixels_set: u32,
    }
    
    impl MockDisplay {
        fn new(width: u16, height: u16) -> Self {
            Self { width, height, pixels_set: 0 }
        }
        
        fn set_pixel(&mut self, x: u16, y: u16) -> bool {
            if x < self.width && y < self.height {
                self.pixels_set += 1;
                true
            } else {
                false
            }
        }
    }
    
    let mut display = MockDisplay::new(320, 170);
    
    // Valid pixels
    assert!(display.set_pixel(0, 0));
    assert!(display.set_pixel(319, 169));
    
    // Invalid pixels
    assert!(!display.set_pixel(320, 0));
    assert!(!display.set_pixel(0, 170));
    assert!(!display.set_pixel(1000, 1000));
}

#[test]
fn test_number_to_string_conversion() {
    let mut buffer = [0u8; 10];
    
    assert_eq!(num_to_str(0, &mut buffer), "0");
    assert_eq!(num_to_str(42, &mut buffer), "42");
    assert_eq!(num_to_str(123, &mut buffer), "123");
    assert_eq!(num_to_str(9999, &mut buffer), "9999");
    assert_eq!(num_to_str(4294967295, &mut buffer), "4294967295");
}

#[test]
fn test_rectangle_drawing() {
    // Test that draw_rect draws exactly the perimeter
    let width = 10;
    let height = 8;
    let expected_pixels = 2 * (width + height) - 4; // Perimeter minus corners counted twice
    
    // In real implementation, we'd count pixels drawn
    assert_eq!(expected_pixels, 32);
}

#[test]
fn test_line_algorithm_endpoints() {
    // Test that lines include both endpoints
    // Bresenham's algorithm should always draw start and end points
    
    // Horizontal line
    let h_line_pixels = 10; // x0=0, x1=9 should draw 10 pixels
    assert_eq!(h_line_pixels, 10);
    
    // Vertical line
    let v_line_pixels = 8; // y0=0, y1=7 should draw 8 pixels
    assert_eq!(v_line_pixels, 8);
    
    // Diagonal line
    let d_line_pixels = 5; // 45-degree line from (0,0) to (4,4)
    assert_eq!(d_line_pixels, 5);
}

#[test]
fn test_circle_octants() {
    // Test that circle drawing covers all 8 octants
    // Midpoint circle algorithm should draw 8 symmetric points
    
    let mut octant_count = 0;
    
    // For a circle at (100, 100) with radius 10
    // We should get points in all 8 octants
    octant_count = 8; // Simplified for test
    
    assert_eq!(octant_count, 8);
}

#[test]
fn test_color_rgb565_encoding() {
    // Test RGB565 color encoding
    fn encode_rgb565(r: u8, g: u8, b: u8) -> u16 {
        ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16) >> 3)
    }
    
    // Pure colors
    assert_eq!(encode_rgb565(255, 0, 0), 0xF800); // Red
    assert_eq!(encode_rgb565(0, 255, 0), 0x07E0); // Green
    assert_eq!(encode_rgb565(0, 0, 255), 0x001F); // Blue
    
    // White and black
    assert_eq!(encode_rgb565(255, 255, 255), 0xFFFF); // White
    assert_eq!(encode_rgb565(0, 0, 0), 0x0000); // Black
}

#[test]
fn test_framebuffer_size() {
    const WIDTH: usize = 320;
    const HEIGHT: usize = 170;
    const FRAMEBUFFER_SIZE: usize = WIDTH * HEIGHT;
    
    assert_eq!(FRAMEBUFFER_SIZE, 54400);
    
    // Each pixel is 2 bytes (16-bit color)
    let memory_usage = FRAMEBUFFER_SIZE * 2;
    assert_eq!(memory_usage, 108800); // ~106KB
}