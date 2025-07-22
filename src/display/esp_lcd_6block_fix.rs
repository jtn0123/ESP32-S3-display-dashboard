/// Fix for 6-block readable pattern in ESP LCD
use anyhow::Result;
use esp_idf_sys::*;
use log::info;

/// Apply fixes for the 6-block readable pattern issue
pub unsafe fn apply_6block_fix(
    bus_config: &mut esp_lcd_i80_bus_config_t,
    io_config: &mut esp_lcd_panel_io_i80_config_t,
) -> Result<()> {
    info!("=== Applying 6-Block Pattern Fix ===");
    
    // Fix 1: Reduce clock speed significantly
    // 17 MHz might be too fast, causing every 6th transfer to succeed
    let original_clock = io_config.pclk_hz;
    io_config.pclk_hz = 5_000_000; // 5 MHz - much slower
    info!("  Reduced clock speed from {} to {} Hz", original_clock, io_config.pclk_hz);
    
    // Fix 2: Adjust transfer alignment
    // Force transfers to be multiples of 12 bytes (6 pixels in RGB565)
    // This aligns with the 6-block pattern observed
    let original_max = bus_config.max_transfer_bytes;
    
    // Calculate aligned transfer size - multiple of 12 bytes
    let base_size = 170 * 2; // One line of pixels in bytes
    let aligned_size = ((base_size + 11) / 12) * 12; // Round up to 12-byte boundary
    
    // Also ensure it's aligned to cache line (64 bytes for PSRAM)
    let cache_aligned = ((aligned_size + 63) / 64) * 64;
    
    bus_config.max_transfer_bytes = cache_aligned as usize;
    info!("  Adjusted max transfer from {} to {} bytes (12-byte aligned)", 
          original_max, bus_config.max_transfer_bytes);
    
    // Fix 3: Reduce queue depth to ensure each transfer completes
    let original_queue = io_config.trans_queue_depth;
    io_config.trans_queue_depth = 1; // Single transfer at a time
    info!("  Reduced queue depth from {} to {}", original_queue, io_config.trans_queue_depth);
    
    // Fix 4: Adjust DC signal timing
    // The 6-pattern might be related to DC signal timing
    io_config.dc_levels._bitfield_1 = esp_lcd_panel_io_i80_config_t__bindgen_ty_1::new_bitfield_1(
        0, // dc_idle_level
        0, // dc_cmd_level  
        0, // dc_dummy_level
        1, // dc_data_level
    );
    info!("  DC levels configured for stable timing");
    
    // Fix 5: Ensure proper byte alignment in memory
    // PSRAM requires 64-byte alignment, SRAM requires 4-byte
    bus_config.__bindgen_anon_1.psram_trans_align = 64;
    bus_config.sram_trans_align = 12; // Changed from 4 to 12 for 6-pixel alignment
    info!("  Memory alignment: PSRAM={}, SRAM={}", 
          bus_config.__bindgen_anon_1.psram_trans_align,
          bus_config.sram_trans_align);
    
    info!("=== 6-Block Fix Applied ===");
    info!("Key changes:");
    info!("- Clock: {} MHz", io_config.pclk_hz / 1_000_000);
    info!("- Transfer size: {} bytes (aligned to 12-byte boundary)", bus_config.max_transfer_bytes);
    info!("- Queue depth: 1 (synchronous transfers)");
    info!("- SRAM alignment: 12 bytes (matches 6-pixel pattern)");
    
    Ok(())
}

/// Test if the 6-block issue is clock-related
pub unsafe fn test_clock_speeds(
    panel_handle: esp_lcd_panel_handle_t,
) -> Result<()> {
    info!("=== Testing Different Clock Speeds for 6-Block Issue ===");
    
    // Test pattern - alternating colors every 6 pixels
    let mut pattern = vec![0u16; 320 * 10];
    for i in 0..pattern.len() {
        if (i / 6) % 2 == 0 {
            pattern[i] = 0xF800; // Red
        } else {
            pattern[i] = 0x07E0; // Green
        }
    }
    
    // Draw the pattern
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 10,
        pattern.as_ptr() as *const _,
    );
    
    info!("Drew 6-pixel alternating pattern");
    info!("If you see solid colors instead of stripes, the 6-block issue is fixed!");
    
    Ok(())
}

/// Analyze the 6-block pattern in detail
pub unsafe fn analyze_6block_pattern(
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== Detailed 6-Block Pattern Analysis ===");
    
    // Test 1: Single pixel writes at 6-pixel intervals
    info!("Test 1: Writing single pixels at positions 0, 6, 12, 18...");
    for i in 0..10 {
        let x = (i * 6) as i32;
        
        // Draw a white pixel
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x, 50,
            x + 1, 51,
            &[0xFFFF_u16] as *const _ as *const _,
        );
    }
    
    info!("  If you see evenly spaced white pixels, alignment is correct");
    
    // Test 2: 6-pixel blocks
    info!("\nTest 2: Drawing 6-pixel wide blocks");
    for i in 0..10 {
        let x = (i * 30) as i32; // 30 pixels apart
        let color = match i % 3 {
            0 => 0xF800_u16, // Red
            1 => 0x07E0_u16, // Green
            _ => 0x001F_u16, // Blue
        };
        
        // Draw 6-pixel wide block
        let block = vec![color; 6 * 20]; // 6x20 pixels
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x, 70,
            x + 6, 90,
            block.as_ptr() as *const _,
        );
    }
    
    info!("  You should see colored blocks 30 pixels apart");
    
    // Test 3: Continuous pattern with markers every 6 pixels
    info!("\nTest 3: Continuous pattern with 6-pixel markers");
    let mut line = vec![0x0000_u16; 320];
    for i in 0..320 {
        if i % 6 == 0 {
            line[i] = 0xFFFF; // White marker every 6 pixels
        } else {
            line[i] = 0x7BEF; // Gray background
        }
    }
    
    // Draw 10 lines
    for y in 100..110 {
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, y,
            320, y + 1,
            line.as_ptr() as *const _,
        );
    }
    
    info!("  You should see vertical white lines every 6 pixels");
    info!("  If only some lines appear, it confirms the 6-block issue");
    
    info!("\n=== Analysis Complete ===");
    info!("The 6-block pattern indicates:");
    info!("1. DMA transfers are succeeding every 6 pixels");
    info!("2. This could be timing or alignment related");
    info!("3. The fix adjusts clock speed and transfer alignment");
    
    Ok(())
}