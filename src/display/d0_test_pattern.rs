/// D0 Test Pattern Module
/// Specific patterns to diagnose vertical striping issues
use anyhow::Result;
use esp_idf_sys::*;
use log::{info, warn};
use super::safe_draw;

/// Draw a pattern that specifically tests D0 (LSB) behavior
pub fn draw_d0_test_pattern(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    info!("=== D0 Test Pattern - Vertical Striping Diagnosis ===");
    
    // Pattern 1: Alternating columns with D0 set/clear
    info!("Pattern 1: Alternating D0 columns");
    draw_alternating_d0_columns(panel_handle)?;
    esp_idf_hal::delay::FreeRtos::delay_ms(2000);
    
    // Pattern 2: Horizontal lines with D0 variations
    info!("Pattern 2: Horizontal D0 test lines");
    draw_horizontal_d0_lines(panel_handle)?;
    esp_idf_hal::delay::FreeRtos::delay_ms(2000);
    
    // Pattern 3: Checkerboard with D0 focus
    info!("Pattern 3: D0 checkerboard");
    draw_d0_checkerboard(panel_handle)?;
    esp_idf_hal::delay::FreeRtos::delay_ms(2000);
    
    // Pattern 4: Solid colors with D0 set
    info!("Pattern 4: Solid colors with D0 variations");
    draw_d0_solid_colors(panel_handle)?;
    
    info!("=== D0 Test Pattern Complete ===");
    Ok(())
}

/// Draw alternating columns where D0 bit differs
fn draw_alternating_d0_columns(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    let width = 320;
    let height = 170;
    let mut buffer = vec![0u16; width * height];
    
    // Create pattern where even columns have D0=0, odd columns have D0=1
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if x % 2 == 0 {
                // Even column: color with D0=0 (e.g., 0xF800 = red)
                buffer[idx] = 0xF800; // Pure red, D0=0
            } else {
                // Odd column: color with D0=1 (e.g., 0xF801)
                buffer[idx] = 0xF801; // Red with D0=1
            }
        }
    }
    
    // Draw the pattern
    unsafe {
        safe_draw::safe_draw_bitmap(
            panel_handle,
            0, 0,
            width as i32, height as i32,
            buffer.as_ptr() as *const _
        )?;
    }
    
    info!("Alternating D0 columns drawn - look for missing columns");
    Ok(())
}

/// Draw horizontal lines with different D0 patterns
fn draw_horizontal_d0_lines(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    let width = 320;
    let height = 170;
    let mut buffer = vec![0u16; width * height];
    
    // Create horizontal bands with different D0 patterns
    let band_height = height / 5;
    
    for y in 0..height {
        let band = y / band_height;
        for x in 0..width {
            let idx = y * width + x;
            match band {
                0 => buffer[idx] = 0x0000, // Black (D0=0)
                1 => buffer[idx] = 0x0001, // Almost black (D0=1)
                2 => buffer[idx] = 0xFFFE, // Almost white (D0=0)
                3 => buffer[idx] = 0xFFFF, // White (D0=1)
                4 => {
                    // Alternating D0 within the line
                    if x % 2 == 0 {
                        buffer[idx] = 0x07E0; // Green (D0=0)
                    } else {
                        buffer[idx] = 0x07E1; // Green (D0=1)
                    }
                }
                _ => buffer[idx] = 0x001F, // Blue
            }
        }
    }
    
    unsafe {
        safe_draw::safe_draw_bitmap(
            panel_handle,
            0, 0,
            width as i32, height as i32,
            buffer.as_ptr() as *const _
        )?;
    }
    
    info!("Horizontal D0 test lines drawn - look for missing or altered bands");
    Ok(())
}

/// Draw a checkerboard pattern focusing on D0 bit
fn draw_d0_checkerboard(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    let width = 320;
    let height = 170;
    let mut buffer = vec![0u16; width * height];
    let checker_size = 10;
    
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let checker_x = x / checker_size;
            let checker_y = y / checker_size;
            
            if (checker_x + checker_y) % 2 == 0 {
                // Use color with D0=0
                buffer[idx] = 0x001E; // Blue with D0=0
            } else {
                // Use color with D0=1
                buffer[idx] = 0x001F; // Blue with D0=1
            }
        }
    }
    
    unsafe {
        safe_draw::safe_draw_bitmap(
            panel_handle,
            0, 0,
            width as i32, height as i32,
            buffer.as_ptr() as *const _
        )?;
    }
    
    info!("D0 checkerboard drawn - missing squares indicate D0 issues");
    Ok(())
}

/// Draw solid colors with D0 variations
fn draw_d0_solid_colors(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    let width = 320;
    let height = 170;
    
    // Test colors with D0 set and clear
    let test_colors = [
        (0xF800, "Red D0=0"),
        (0xF801, "Red D0=1"),
        (0x07E0, "Green D0=0"),
        (0x07E1, "Green D0=1"),
        (0x001E, "Blue D0=0"),
        (0x001F, "Blue D0=1"),
    ];
    
    for (color, name) in test_colors.iter() {
        info!("Drawing solid color: {} (0x{:04X})", name, color);
        
        let buffer = vec![*color; width * height];
        unsafe {
            safe_draw::safe_draw_bitmap(
                panel_handle,
                0, 0,
                width as i32, height as i32,
                buffer.as_ptr() as *const _
            )?;
        }
        
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    }
    
    Ok(())
}

/// Analyze D0 stuck HIGH symptoms
pub fn analyze_d0_symptoms(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    info!("=== Analyzing D0 Stuck HIGH Symptoms ===");
    
    // If D0 is stuck HIGH, we expect:
    // 1. All even colors (D0=0) will appear as odd colors (D0=1)
    // 2. Vertical stripes when alternating D0
    // 3. Color shifts in gradients
    
    // Test 1: Write 0x0000 (black) and see if it appears as 0x0001
    let black_buffer = vec![0x0000u16; 100];
    let almost_black_buffer = vec![0x0001u16; 100];
    
    info!("Test 1: Drawing pure black (0x0000)...");
    unsafe {
        safe_draw::safe_draw_bitmap(
            panel_handle,
            10, 10,
            20, 20,
            black_buffer.as_ptr() as *const _
        )?;
    }
    
    info!("Test 2: Drawing almost black (0x0001) for comparison...");
    unsafe {
        safe_draw::safe_draw_bitmap(
            panel_handle,
            30, 10,
            40, 20,
            almost_black_buffer.as_ptr() as *const _
        )?;
    }
    
    info!("If both squares look identical, D0 is likely stuck HIGH");
    
    // Test 2: Gradient test
    info!("Test 3: Drawing gradient to detect D0 issues...");
    let mut gradient_buffer = vec![0u16; 320 * 50];
    for x in 0..320 {
        let gray = (x * 31 / 320) as u16; // 5-bit gray
        let color = (gray << 11) | (gray << 6) | gray; // RGB gray
        for y in 0..50 {
            gradient_buffer[y * 320 + x] = color;
        }
    }
    
    unsafe {
        safe_draw::safe_draw_bitmap(
            panel_handle,
            0, 60,
            320, 110,
            gradient_buffer.as_ptr() as *const _
        )?;
    }
    
    info!("Look for banding or steps in the gradient - indicates D0 issues");
    
    info!("=== D0 Analysis Complete ===");
    warn!("If you see vertical stripes or color shifts, GPIO39 (D0) may be stuck HIGH");
    warn!("This is often a hardware issue requiring inspection of the PCB");
    
    Ok(())
}