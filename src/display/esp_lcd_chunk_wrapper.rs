/// Safe chunking wrapper for ESP LCD transfers that exceed DMA buffer limits
use anyhow::Result;
use esp_idf_sys::*;
use log::{debug, error, warn};
use core::cmp::min;
use core::ffi::c_void;
use super::error_diagnostics::{log_display_error, validate_display_params};

/// Maximum safe transfer size (64KB minus some overhead for safety)
/// The DMA buffer is 64KB but we leave some headroom
const MAX_CHUNK_SIZE: usize = 60 * 1024; // 60KB to be safe

/// Safe wrapper for esp_lcd_panel_draw_bitmap that chunks large transfers
/// 
/// This prevents DMA buffer overflow by splitting transfers larger than 64KB
/// into smaller chunks. Each chunk is sent sequentially.
pub unsafe fn safe_draw_bitmap(
    panel: esp_lcd_panel_handle_t,
    x_start: i32,
    y_start: i32,
    x_end: i32,
    y_end: i32,
    color_data: *const c_void,
) -> Result<()> {
    // Validate parameters first
    if panel.is_null() {
        error!("safe_draw_bitmap: panel handle is NULL!");
        return Err(anyhow::anyhow!("Panel handle is NULL"));
    }
    
    if color_data.is_null() {
        error!("safe_draw_bitmap: color_data is NULL!");
        return Err(anyhow::anyhow!("Color data is NULL"));
    }
    
    // Calculate total size
    let width = (x_end - x_start) as usize;
    let height = (y_end - y_start) as usize;
    let bytes_per_pixel = 2; // RGB565
    let total_bytes = width * height * bytes_per_pixel;
    
    debug!("safe_draw_bitmap: region ({},{}) to ({},{}) = {}x{} pixels, {} bytes",
          x_start, y_start, x_end, y_end, width, height, total_bytes);
    
    // If it fits in one transfer, just do it directly
    if total_bytes <= MAX_CHUNK_SIZE {
        let ret = esp_lcd_panel_draw_bitmap(panel, x_start, y_start, x_end, y_end, color_data);
        if ret != ESP_OK {
            log_display_error("esp_lcd_panel_draw_bitmap", 
                &format!("Direct transfer of {}x{} region", width, height), ret);
            return Err(anyhow::anyhow!("Failed to draw bitmap: error {}", ret));
        }
        return Ok(());
    }
    
    // Need to chunk - calculate rows per chunk
    let bytes_per_row = width * bytes_per_pixel;
    let rows_per_chunk = MAX_CHUNK_SIZE / bytes_per_row;
    
    if rows_per_chunk == 0 {
        // Single row is too big - need to chunk horizontally
        return Err(anyhow::anyhow!(
            "Row size {} bytes exceeds chunk size {} - horizontal chunking not implemented",
            bytes_per_row, MAX_CHUNK_SIZE
        ));
    }
    
    debug!("Chunking large transfer: {}x{} pixels ({} bytes) into {} row chunks",
          width, height, total_bytes, rows_per_chunk);
    
    // Process in chunks
    let data_ptr = color_data as *const u8;
    let mut current_y = y_start;
    let mut offset = 0;
    let mut chunk_count = 0;
    
    while current_y < y_end {
        let chunk_height = min(rows_per_chunk, (y_end - current_y) as usize);
        let chunk_end_y = current_y + chunk_height as i32;
        let chunk_bytes = chunk_height * bytes_per_row;
        
        // Draw this chunk
        let chunk_data = data_ptr.add(offset) as *const c_void;
        let ret = esp_lcd_panel_draw_bitmap(
            panel,
            x_start,
            current_y,
            x_end,
            chunk_end_y,
            chunk_data
        );
        
        if ret != ESP_OK {
            log_display_error("esp_lcd_panel_draw_bitmap", 
                &format!("Chunk {} at y={}, size={} bytes", chunk_count, current_y, chunk_bytes), 
                ret);
            error!("Chunk transfer failed after {} successful chunks", chunk_count);
            return Err(anyhow::anyhow!(
                "Failed to draw chunk at y={}: error {}",
                current_y, ret
            ));
        }
        
        // Reset watchdog every few chunks to prevent timeout
        chunk_count += 1;
        if chunk_count % 4 == 0 {
            esp_task_wdt_reset();
        }
        
        // Move to next chunk
        current_y = chunk_end_y;
        offset += chunk_bytes;
    }
    
    debug!("Successfully completed chunked transfer");
    Ok(())
}

/// Helper to calculate if a transfer needs chunking
pub fn needs_chunking(width: usize, height: usize) -> bool {
    let bytes_per_pixel = 2; // RGB565
    let total_bytes = width * height * bytes_per_pixel;
    total_bytes > MAX_CHUNK_SIZE
}

/// Get the maximum safe dimensions for a single transfer
pub fn max_safe_dimensions() -> (usize, usize) {
    // For 170 pixel width: max height = 60KB / (170 * 2) = 176 rows
    // For 320 pixel width: max height = 60KB / (320 * 2) = 93 rows
    (320, 93) // Conservative for landscape mode
}