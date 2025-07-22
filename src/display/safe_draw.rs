/// Safe drawing wrapper that prevents DMA buffer overflows
/// This module provides a drop-in replacement for esp_lcd_panel_draw_bitmap
/// that automatically chunks large transfers to stay within DMA limits

use esp_idf_sys::*;

/// Re-export safe_draw_bitmap as draw_bitmap for easy replacement
pub use super::esp_lcd_chunk_wrapper::safe_draw_bitmap;
pub use safe_draw_bitmap as draw_bitmap;

/// Helper to clear the screen safely
pub unsafe fn clear_screen(panel: esp_lcd_panel_handle_t) -> anyhow::Result<()> {
    let black_buffer = vec![0u16; 320 * 170];
    draw_bitmap(
        panel,
        0, 0,
        320, 170,
        black_buffer.as_ptr() as *const _,
    )
}

/// Helper to fill screen with a color safely
pub unsafe fn fill_screen(panel: esp_lcd_panel_handle_t, color: u16) -> anyhow::Result<()> {
    let buffer = vec![color; 320 * 170];
    draw_bitmap(
        panel,
        0, 0,
        320, 170,
        buffer.as_ptr() as *const _,
    )
}

/// Helper for drawing with watchdog reset
pub unsafe fn draw_with_wdt_reset(
    panel: esp_lcd_panel_handle_t,
    x0: i32, y0: i32,
    x1: i32, y1: i32,
    data: *const core::ffi::c_void,
) -> anyhow::Result<()> {
    let result = draw_bitmap(panel, x0, y0, x1, y1, data);
    
    // Reset watchdog after draw operation
    esp_task_wdt_reset();
    
    result
}