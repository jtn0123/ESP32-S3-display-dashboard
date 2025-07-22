use anyhow::Result;
use esp_idf_sys::*;
use log::info;
use esp_idf_hal::delay::Ets;
use core::ptr;

/// Debug blocky display with intermittent readable sections
pub unsafe fn debug_block_issue(
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== ESP LCD BLOCK DEBUG ===");
    info!("Issue: Display is blocky with intermittent readable sections");
    info!("This suggests DMA alignment or transfer size issues");
    
    // Test 1: Single pixel writes to verify basic communication
    info!("\nTest 1: Single pixel writes");
    for i in 0..10 {
        let x = i * 20;
        let y = 50;
        
        // Set window to single pixel
        set_window(io_handle, x, y, x, y)?;
        esp_lcd_panel_io_tx_param(io_handle, 0x2C, ptr::null(), 0);
        
        // Write single red pixel
        let pixel_data: [u8; 2] = [0xF8, 0x00]; // Red in RGB565
        esp_lcd_panel_io_tx_color(
            io_handle,
            -1,
            pixel_data.as_ptr() as *const _,
            2,
        );
        
        info!("  Drew pixel at ({}, {})", x, y);
    }
    Ets::delay_ms(1000);
    
    // Test 2: Small block writes with different sizes
    info!("\nTest 2: Testing different block sizes");
    let block_sizes = [1, 2, 4, 8, 16, 32, 64];
    
    for (i, &size) in block_sizes.iter().enumerate() {
        let y_offset = i as u16 * 20;
        
        // Clear area first
        draw_block(panel_handle, 0, y_offset, 320, 20, 0x0000)?; // Black
        
        // Draw test blocks
        for x in (0..320).step_by(size * 2) {
            draw_block(panel_handle, x as u16, y_offset, size as u16, 10, 0xF800)?; // Red
        }
        
        info!("  Drew blocks of size {} at y={}", size, y_offset);
        Ets::delay_ms(500);
    }
    
    // Test 3: Verify transfer alignment
    info!("\nTest 3: Testing transfer alignment");
    test_alignment_patterns(panel_handle)?;
    
    // Test 4: Test different transfer sizes
    info!("\nTest 4: Testing transfer sizes");
    test_transfer_sizes(io_handle)?;
    
    // Test 5: Memory barrier test
    info!("\nTest 5: Memory barrier test");
    test_with_memory_barriers(panel_handle)?;
    
    info!("\n=== BLOCK DEBUG ANALYSIS ===");
    info!("If you see:");
    info!("- Regular pattern of blocks: Transfer size issue");
    info!("- Random corruption: DMA timing issue");
    info!("- Consistent 6-block pattern: 6-byte alignment issue");
    info!("- Improvement with barriers: Cache coherency issue");
    
    Ok(())
}

/// Test alignment patterns
unsafe fn test_alignment_patterns(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    info!("Drawing alignment test patterns...");
    
    // Pattern 1: Single byte columns
    for x in 0..320 {
        let color = if x % 2 == 0 { 0xF800 } else { 0x07E0 }; // Red/Green
        let buffer = vec![color; 170];
        
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x as i32, 0,
            (x + 1) as i32, 170,
            buffer.as_ptr() as *const _,
        );
    }
    
    info!("  Pattern 1: Alternating red/green columns");
    Ets::delay_ms(1000);
    
    // Pattern 2: Aligned blocks
    let mut buffer = vec![0u16; 320 * 170];
    for y in 0..170 {
        for x in 0..320 {
            let idx = y * 320 + x;
            // Create 8-pixel aligned pattern
            buffer[idx] = if (x / 8) % 2 == 0 { 0xFFFF } else { 0x0000 };
        }
    }
    
    esp_lcd_panel_draw_bitmap(
        panel_handle,
        0, 0,
        320, 170,
        buffer.as_ptr() as *const _,
    );
    
    info!("  Pattern 2: 8-pixel aligned blocks");
    Ets::delay_ms(1000);
    
    Ok(())
}

/// Test different transfer sizes
unsafe fn test_transfer_sizes(io_handle: *mut esp_lcd_panel_io_t) -> Result<()> {
    // Test various transfer sizes to find optimal
    let transfer_sizes = [2, 4, 8, 16, 32, 64, 128, 256, 512, 1024];
    
    for &size in &transfer_sizes {
        info!("Testing transfer size: {} bytes", size);
        
        // Set small window
        set_window(io_handle, 0, 0, 63, 0)?; // 64 pixels = 128 bytes
        esp_lcd_panel_io_tx_param(io_handle, 0x2C, ptr::null(), 0);
        
        // Create pattern
        let mut data = Vec::new();
        for i in 0..size/2 {
            if i % 2 == 0 {
                data.push(0xFF); // White high byte
                data.push(0xFF); // White low byte
            } else {
                data.push(0x00); // Black high byte
                data.push(0x00); // Black low byte
            }
        }
        
        // Send in chunks
        let chunks = 128 / size;
        for _ in 0..chunks {
            esp_lcd_panel_io_tx_color(
                io_handle,
                -1,
                data.as_ptr() as *const _,
                data.len(),
            );
        }
        
        Ets::delay_ms(100);
    }
    
    Ok(())
}

/// Test with memory barriers
unsafe fn test_with_memory_barriers(panel_handle: esp_lcd_panel_handle_t) -> Result<()> {
    info!("Testing with memory barriers...");
    
    // Create test pattern
    let mut buffer = vec![0u16; 320 * 10];
    
    for i in 0..10 {
        // Fill with pattern
        for x in 0..320 {
            buffer[i * 320 + x] = if x % 10 < 5 { 0xF800 } else { 0x001F };
        }
        
        // Memory barrier before DMA
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        
        // Draw line
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, (i * 10) as i32,
            320, ((i + 1) * 10) as i32,
            buffer[i * 320..].as_ptr() as *const _,
        );
        
        // Memory barrier after DMA
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        
        // Small delay between transfers
        Ets::delay_us(100);
    }
    
    info!("  Drew 10 lines with memory barriers");
    
    Ok(())
}

/// Helper to set window
unsafe fn set_window(
    io: *mut esp_lcd_panel_io_t,
    x1: u16, y1: u16, x2: u16, y2: u16
) -> Result<()> {
    // CASET
    let caset: [u8; 4] = [
        (x1 >> 8) as u8, (x1 & 0xFF) as u8,
        (x2 >> 8) as u8, (x2 & 0xFF) as u8,
    ];
    esp_lcd_panel_io_tx_param(io, 0x2A, caset.as_ptr() as *const _, 4);
    
    // RASET with Y_GAP
    let y1_off = y1 + 35;
    let y2_off = y2 + 35;
    let raset: [u8; 4] = [
        (y1_off >> 8) as u8, (y1_off & 0xFF) as u8,
        (y2_off >> 8) as u8, (y2_off & 0xFF) as u8,
    ];
    esp_lcd_panel_io_tx_param(io, 0x2B, raset.as_ptr() as *const _, 4);
    
    Ok(())
}

/// Helper to draw a block
fn draw_block(
    panel_handle: esp_lcd_panel_handle_t,
    x: u16, y: u16, w: u16, h: u16, color: u16
) -> Result<()> {
    let buffer = vec![color; (w * h) as usize];
    unsafe {
        esp_lcd_panel_draw_bitmap(
            panel_handle,
            x as i32, y as i32,
            (x + w) as i32, (y + h) as i32,
            buffer.as_ptr() as *const _,
        );
    }
    Ok(())
}

/// Debug the 6-block pattern specifically
pub unsafe fn debug_6_block_pattern(
    panel_handle: esp_lcd_panel_handle_t,
    io_handle: *mut esp_lcd_panel_io_t,
) -> Result<()> {
    info!("=== 6-BLOCK PATTERN DEBUG ===");
    info!("You mentioned seeing 6 readable blocks intermittently");
    info!("This suggests a 6-byte or 6-word alignment issue");
    
    // Test 1: Draw 6-pixel wide columns
    info!("\nTest 1: 6-pixel columns");
    for i in 0..53 { // 320/6 ≈ 53
        let x = i * 6;
        let color = match i % 3 {
            0 => 0xF800, // Red
            1 => 0x07E0, // Green
            _ => 0x001F, // Blue
        };
        
        for y in 0..170 {
            draw_block(panel_handle, x, y, 6, 1, color)?;
        }
    }
    info!("  If this shows clean columns, it's a 6-pixel alignment issue");
    Ets::delay_ms(2000);
    
    // Test 2: Test with 6-byte transfers
    info!("\nTest 2: 6-byte transfer test");
    set_window(io_handle, 0, 0, 2, 0)?; // 3 pixels = 6 bytes
    esp_lcd_panel_io_tx_param(io_handle, 0x2C, ptr::null(), 0);
    
    // Send 6 bytes at a time
    let data: [u8; 6] = [0xF8, 0x00, 0x07, 0xE0, 0x00, 0x1F]; // R, G, B
    for _ in 0..100 {
        esp_lcd_panel_io_tx_color(
            io_handle,
            -1,
            data.as_ptr() as *const _,
            6,
        );
    }
    
    info!("  Sent 100 6-byte transfers");
    
    // Test 3: Check if it's related to DMA descriptor size
    info!("\nTest 3: DMA descriptor boundaries");
    
    // Common DMA descriptor sizes that might cause 6-block patterns
    let descriptor_sizes = [4092, 4096, 8184, 8192]; // Near 4K boundaries
    
    for &size in &descriptor_sizes {
        let pixels = size / 2; // 2 bytes per pixel
        if pixels <= 320 * 170 {
            let buffer = vec![0xFFFF_u16; pixels];
            info!("  Testing {} byte transfer ({} pixels)", size, pixels);
            
            esp_lcd_panel_draw_bitmap(
                panel_handle,
                0, 0,
                320.min(pixels as i32), (pixels / 320).min(170) as i32,
                buffer.as_ptr() as *const _,
            );
            
            Ets::delay_ms(500);
        }
    }
    
    info!("\n=== 6-BLOCK PATTERN ANALYSIS ===");
    info!("Possible causes:");
    info!("1. DMA transfers splitting at 6-pixel boundaries");
    info!("2. I80 bus timing causing every 6th transfer to work");
    info!("3. Buffer alignment issues with 6-byte boundaries");
    info!("4. Clock divider creating 1-in-6 timing windows");
    
    Ok(())
}