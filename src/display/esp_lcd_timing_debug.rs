use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;

/// Maximum timing debug for blocky display
pub unsafe fn debug_timing_issues(
    bus_handle: esp_lcd_i80_bus_handle_t,
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== ESP LCD TIMING DEBUG ===");
    info!("Testing various timing configurations to fix blocky display");
    
    // Test 1: Add delays between all operations
    info!("\nTest 1: Adding inter-operation delays");
    
    // Clear screen with delays
    for y in 0..170 {
        let black_line = vec![0u16; 320];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, y,
            320, y + 1,
            black_line.as_ptr() as *const _,
        );
        
        // Delay between each line
        Ets::delay_us(10);
    }
    
    info!("  Cleared screen with 10us delays between lines");
    
    // Test 2: Slow pattern with delays
    info!("\nTest 2: Drawing pattern with various delays");
    
    let delay_values = [0, 1, 5, 10, 50, 100, 500];
    
    for (i, &delay_us) in delay_values.iter().enumerate() {
        let y = (i * 20) as i32;
        
        // Draw a test bar
        for x in 0..320 {
            let color = if x % 20 < 10 { 0xF800 } else { 0x07E0 };
            let pixel = [color];
            
            esp_lcd_panel_draw_bitmap(
                panel_handle,
                x, y,
                x + 1, y + 20,
                pixel.as_ptr() as *const _,
            );
            
            if delay_us > 0 {
                Ets::delay_us(delay_us);
            }
        }
        
        info!("  Row {}: {}us delay between pixels", i, delay_us);
    }
    
    Ets::delay_ms(2000);
    
    // Test 3: Flush and sync operations
    info!("\nTest 3: Testing flush and sync");
    
    // Draw test pattern
    let pattern = create_test_pattern();
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        pattern.as_ptr() as *const _,
    );
    
    // Try to flush/sync if available
    info!("  Pattern drawn, waiting for DMA completion");
    
    // Add memory barrier
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    
    // Wait for any pending DMA
    Ets::delay_ms(100);
    
    // Test 4: Clock speed ramping
    info!("\nTest 4: Testing timing at different speeds");
    
    // We can't change clock dynamically, but we can test with different delays
    // to simulate the effect
    
    for multiplier in 1..=5 {
        info!("  Testing with {}x timing delays", multiplier);
        
        // Draw checkerboard with timing delays
        for y in 0..10 {
            for x in 0..32 {
                let color = if (x + y) % 2 == 0 { 0xFFFF } else { 0x0000 };
                let block = vec![color; 100]; // 10x10 block
                
                esp_lcd_panel_draw_bitmap(
                    panel_handle,
                    (x * 10) as i32,
                    (y * 10) as i32,
                    ((x + 1) * 10) as i32,
                    ((y + 1) * 10) as i32,
                    block.as_ptr() as *const _,
                );
                
                // Delay proportional to multiplier
                Ets::delay_us(multiplier * 10);
            }
        }
        
        Ets::delay_ms(500);
    }
    
    // Test 5: Verify bus timing
    info!("\nTest 5: Bus timing verification");
    verify_bus_timing()?;
    
    info!("\n=== TIMING DEBUG COMPLETE ===");
    info!("Look for:");
    info!("- Improvement with delays: Timing too fast");
    info!("- Pattern changes with different delays: Setup/hold time issues");
    info!("- Consistent blocks regardless: Not a timing issue");
    
    Ok(())
}

/// Create test pattern for timing debug
fn create_test_pattern() -> Vec<u16> {
    let mut pattern = vec![0u16; 320 * 170];
    
    // Create pattern that will show timing issues
    for y in 0..170 {
        for x in 0..320 {
            let idx = y * 320 + x;
            
            // Alternating pattern that's sensitive to timing
            if (x + y) % 2 == 0 {
                pattern[idx] = 0xF800; // Red
            } else if x % 10 == 0 {
                pattern[idx] = 0xFFFF; // White markers every 10 pixels
            } else {
                pattern[idx] = 0x001F; // Blue
            }
        }
    }
    
    pattern
}

/// Verify I80 bus timing parameters
unsafe fn verify_bus_timing() -> Result<()> {
    info!("Checking I80 bus timing parameters:");
    
    // These would be the timing parameters if we could access them
    info!("  Setup time: Default (would need to check registers)");
    info!("  Hold time: Default (would need to check registers)");
    info!("  WR pulse width: Based on clock speed");
    
    // Test with manual timing control
    info!("  Testing with manual WR pulse control...");
    
    // Note: We can't directly control WR timing from here,
    // but we can add delays to help diagnose
    
    Ok(())
}

/// Test specific for the 6-block readable pattern
pub unsafe fn test_6_block_timing(
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== 6-BLOCK TIMING TEST ===");
    
    // Theory: Every 6th block is readable because of timing alignment
    // Test: Draw pattern that emphasizes 6-block boundaries
    
    // Pattern 1: 6-block width stripes
    info!("Pattern 1: 6-block width stripes");
    
    for block in 0..53 { // 320/6 ≈ 53
        let x = block * 6;
        let color = if block % 2 == 0 { 0xF800 } else { 0x07E0 };
        
        // Draw 6-pixel wide stripe
        let stripe = vec![color; 6 * 170];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x as i32, 0,
            (x + 6) as i32, 170,
            stripe.as_ptr() as *const _,
        );
        
        // Critical: Add small delay between 6-pixel blocks
        Ets::delay_us(1);
    }
    
    info!("  Drew 53 6-pixel stripes with 1us gaps");
    Ets::delay_ms(2000);
    
    // Pattern 2: Test if it's every 6th transfer
    info!("\nPattern 2: Every 6th pixel different");
    
    let mut pattern = vec![0u16; 320 * 170];
    for i in 0..pattern.len() {
        if i % 6 == 5 {
            pattern[i] = 0xFFFF; // White every 6th pixel
        } else {
            pattern[i] = 0x0000; // Black otherwise
        }
    }
    
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        pattern.as_ptr() as *const _,
    );
    
    info!("  If you see vertical white lines, it confirms 6-pixel period");
    Ets::delay_ms(2000);
    
    // Pattern 3: Numbers to identify which blocks are readable
    info!("\nPattern 3: Block identification");
    
    // Draw numbered blocks
    for i in 0..6 {
        let x = i * 53;
        let color = match i {
            0 => 0xF800, // Red
            1 => 0x07E0, // Green  
            2 => 0x001F, // Blue
            3 => 0xFFFF, // White
            4 => 0xFFF0, // Yellow
            5 => 0xF81F, // Magenta
            _ => 0x0000,
        };
        
        // Draw large colored block
        let block = vec![color; 53 * 170];
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x as i32, 0,
            (x + 53) as i32, 170,
            block.as_ptr() as *const _,
        );
    }
    
    info!("  Drew 6 colored blocks (R,G,B,W,Y,M)");
    info!("  Note which colors you can see clearly");
    
    Ok(())
}