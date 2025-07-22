/// Display Error Diagnostics Module
/// Provides comprehensive error logging and diagnostics for display issues

use esp_idf_sys::*;
use log::{error, warn, info};
use std::sync::Mutex;
use std::collections::VecDeque;
use std::time::{Instant, Duration};

// Global error history for pattern detection
static mut ERROR_HISTORY: Option<Mutex<ErrorHistory>> = None;
static ERROR_HISTORY_INIT: std::sync::Once = std::sync::Once::new();

fn get_error_history() -> &'static Mutex<ErrorHistory> {
    unsafe {
        ERROR_HISTORY_INIT.call_once(|| {
            ERROR_HISTORY = Some(Mutex::new(ErrorHistory::new()));
        });
        ERROR_HISTORY.as_ref().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct DisplayError {
    pub timestamp: Instant,
    pub error_code: esp_err_t,
    pub context: String,
    pub operation: String,
}

pub struct ErrorHistory {
    errors: VecDeque<DisplayError>,
    max_errors: usize,
}

impl ErrorHistory {
    fn new() -> Self {
        Self {
            errors: VecDeque::with_capacity(100),
            max_errors: 100,
        }
    }
    
    fn add_error(&mut self, error: DisplayError) {
        if self.errors.len() >= self.max_errors {
            self.errors.pop_front();
        }
        self.errors.push_back(error);
    }
    
    fn get_recent_errors(&self, duration: Duration) -> Vec<&DisplayError> {
        let cutoff = Instant::now() - duration;
        self.errors.iter()
            .filter(|e| e.timestamp > cutoff)
            .collect()
    }
    
    fn detect_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        let recent = self.get_recent_errors(Duration::from_secs(10));
        
        // Check for repeated errors
        let mut error_counts = std::collections::HashMap::new();
        for err in &recent {
            *error_counts.entry(err.error_code).or_insert(0) += 1;
        }
        
        for (code, count) in error_counts {
            if count >= 3 {
                patterns.push(format!("Repeated error 0x{:X} ({} times in 10s)", code, count));
            }
        }
        
        // Check for error bursts
        if recent.len() >= 10 {
            patterns.push(format!("Error burst detected: {} errors in 10s", recent.len()));
        }
        
        patterns
    }
}

/// Translate ESP-IDF error codes to human-readable descriptions
pub fn translate_esp_error(err: esp_err_t) -> &'static str {
    match err {
        ESP_OK => "Success",
        ESP_FAIL => "Generic failure",
        ESP_ERR_NO_MEM => "Out of memory",
        ESP_ERR_INVALID_ARG => "Invalid argument",
        ESP_ERR_INVALID_STATE => "Invalid state",
        ESP_ERR_INVALID_SIZE => "Invalid size",
        ESP_ERR_NOT_FOUND => "Requested resource not found",
        ESP_ERR_NOT_SUPPORTED => "Operation not supported",
        ESP_ERR_TIMEOUT => "Operation timed out",
        ESP_ERR_INVALID_RESPONSE => "Invalid response received",
        ESP_ERR_INVALID_CRC => "CRC or checksum invalid",
        ESP_ERR_INVALID_VERSION => "Version invalid",
        ESP_ERR_INVALID_MAC => "MAC address invalid",
        ESP_ERR_NOT_FINISHED => "Operation not finished",
        _ => "Unknown error",
    }
}

/// Log and analyze display errors with context
pub fn log_display_error(operation: &str, context: &str, error_code: esp_err_t) {
    let error_desc = translate_esp_error(error_code);
    
    // Log the error with full context
    error!("Display Error in {}: {} (0x{:X}) - Context: {}", 
        operation, error_desc, error_code, context);
    
    // Add to error history
    let display_error = DisplayError {
        timestamp: Instant::now(),
        error_code,
        context: context.to_string(),
        operation: operation.to_string(),
    };
    
    if let Ok(mut history) = get_error_history().lock() {
        history.add_error(display_error);
        
        // Check for patterns
        let patterns = history.detect_patterns();
        if !patterns.is_empty() {
            error!("Error patterns detected:");
            for pattern in patterns {
                error!("  - {}", pattern);
            }
        }
    }
    
    // Provide specific guidance based on error
    match error_code {
        ESP_ERR_NO_MEM => {
            error!("Memory allocation failed - possible causes:");
            error!("  1. DMA descriptors exhausted");
            error!("  2. Heap fragmentation");
            error!("  3. Frame buffer too large");
            log_memory_diagnostics();
        },
        ESP_ERR_INVALID_ARG => {
            error!("Invalid argument - check:");
            error!("  1. Coordinate bounds (x,y within display size)");
            error!("  2. Color format (RGB565 expected)");
            error!("  3. Null pointers in data");
        },
        ESP_ERR_TIMEOUT => {
            error!("Timeout - possible causes:");
            error!("  1. Display not responding");
            error!("  2. I80 bus congestion");
            error!("  3. Interrupt handling issues");
        },
        ESP_ERR_INVALID_STATE => {
            error!("Invalid state - check:");
            error!("  1. Display not initialized");
            error!("  2. Bus already in use");
            error!("  3. Panel in sleep mode");
        },
        _ => {}
    }
}

/// Log memory diagnostics
pub fn log_memory_diagnostics() {
    unsafe {
        let free_heap = esp_get_free_heap_size();
        let min_free_heap = esp_get_minimum_free_heap_size();
        
        info!("Memory Diagnostics:");
        info!("  Free heap: {} bytes", free_heap);
        info!("  Min free heap: {} bytes", min_free_heap);
        
        // Check DMA capability
        let dma_caps = MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL;
        let free_dma = heap_caps_get_free_size(dma_caps);
        let largest_dma = heap_caps_get_largest_free_block(dma_caps);
        
        info!("  Free DMA memory: {} bytes", free_dma);
        info!("  Largest DMA block: {} bytes", largest_dma);
        
        if free_dma < 65536 {
            warn!("Low DMA memory! May cause allocation failures");
        }
        
        if largest_dma < 32768 {
            warn!("DMA memory fragmented! Largest block only {} bytes", largest_dma);
        }
    }
}

/// Validate display parameters
pub fn validate_display_params(width: u16, height: u16, x: u16, y: u16, w: u16, h: u16) -> Result<(), String> {
    if x >= width {
        return Err(format!("X coordinate {} exceeds display width {}", x, width));
    }
    if y >= height {
        return Err(format!("Y coordinate {} exceeds display height {}", y, height));
    }
    if x + w > width {
        return Err(format!("Region extends past display width: {} + {} > {}", x, w, width));
    }
    if y + h > height {
        return Err(format!("Region extends past display height: {} + {} > {}", y, h, height));
    }
    if w == 0 || h == 0 {
        return Err(format!("Invalid dimensions: {}x{}", w, h));
    }
    Ok(())
}

/// Check display health and connectivity
pub fn check_display_health(panel_handle: esp_lcd_panel_handle_t) -> Result<(), String> {
    unsafe {
        info!("Performing display health check...");
        
        // Check if panel handle is valid
        if panel_handle.is_null() {
            return Err("Panel handle is NULL".to_string());
        }
        
        // Try a small draw operation to test connectivity
        let test_color: [u8; 2] = [0xFF, 0xFF]; // White pixel
        let ret = esp_lcd_panel_draw_bitmap(
            panel_handle,
            0, 0, 1, 1,
            test_color.as_ptr() as *const _
        );
        
        if ret != ESP_OK {
            return Err(format!("Display communication test failed: 0x{:X}", ret));
        }
        
        info!("Display health check passed");
        Ok(())
    }
}

/// Log I80 bus configuration for debugging
pub fn log_i80_config(bus_config: &esp_lcd_i80_bus_config_t, io_config: &esp_lcd_panel_io_i80_config_t) {
    info!("I80 Bus Configuration:");
    info!("  Data pins: D0={}, D1={}, D2={}, D3={}", 
        bus_config.data_gpio_nums[0],
        bus_config.data_gpio_nums[1], 
        bus_config.data_gpio_nums[2],
        bus_config.data_gpio_nums[3]);
    info!("  Data pins: D4={}, D5={}, D6={}, D7={}", 
        bus_config.data_gpio_nums[4],
        bus_config.data_gpio_nums[5], 
        bus_config.data_gpio_nums[6],
        bus_config.data_gpio_nums[7]);
    info!("  Control pins: WR={}, DC={}", bus_config.wr_gpio_num, bus_config.dc_gpio_num);
    info!("  Bus width: {} bits", bus_config.bus_width);
    info!("  Max transfer: {} bytes", bus_config.max_transfer_bytes);
    info!("  SRAM trans align: {}", bus_config.sram_trans_align);
    
    info!("I80 IO Configuration:");
    info!("  CS pin: {}", io_config.cs_gpio_num);
    info!("  Pclk Hz: {}", io_config.pclk_hz);
    info!("  Trans queue depth: {}", io_config.trans_queue_depth);
    // Skip detailed flag decoding since structure varies by version
}

/// Analyze crash patterns
pub fn analyze_crash_pattern() {
    if let Ok(history) = get_error_history().lock() {
        let recent_errors = history.get_recent_errors(Duration::from_secs(30));
        
        info!("=== Display Error Analysis (last 30s) ===");
        info!("Total errors: {}", recent_errors.len());
        
        // Group by operation
        let mut op_counts = std::collections::HashMap::new();
        for err in &recent_errors {
            *op_counts.entry(err.operation.as_str()).or_insert(0) += 1;
        }
        
        info!("Errors by operation:");
        for (op, count) in op_counts {
            info!("  {}: {} errors", op, count);
        }
        
        // Most recent errors
        info!("Recent errors:");
        for (i, err) in recent_errors.iter().rev().take(5).enumerate() {
            info!("  {}: {} in {} - 0x{:X}", 
                i, err.operation, err.context, err.error_code);
        }
    }
}

/// Emergency display recovery attempt
pub fn attempt_display_recovery(panel_handle: esp_lcd_panel_handle_t) -> Result<(), String> {
    warn!("Attempting display recovery...");
    
    unsafe {
        // Try to reset the panel
        let ret = esp_lcd_panel_reset(panel_handle);
        if ret != ESP_OK {
            error!("Panel reset failed: 0x{:X}", ret);
            return Err(format!("Reset failed: 0x{:X}", ret));
        }
        
        // Wait for reset
        esp_idf_hal::delay::FreeRtos::delay_ms(100);
        
        // Re-initialize
        let ret = esp_lcd_panel_init(panel_handle);
        if ret != ESP_OK {
            error!("Panel re-init failed: 0x{:X}", ret);
            return Err(format!("Re-init failed: 0x{:X}", ret));
        }
        
        warn!("Display recovery completed");
        Ok(())
    }
}

/// Dump complete display diagnostics
pub fn dump_display_diagnostics() {
    info!("=== DISPLAY DIAGNOSTICS DUMP ===");
    
    // Memory state
    log_memory_diagnostics();
    
    // Error history
    analyze_crash_pattern();
    
    // GPIO states
    unsafe {
        info!("GPIO States:");
        let pins = [
            (39, "D0"), (40, "D1"), (41, "D2"), (42, "D3"),
            (45, "D4"), (46, "D5"), (47, "D6"), (48, "D7"),
            (8, "WR"), (7, "DC"), (6, "CS"), (5, "RST"),
            (38, "BL"), (15, "PWR"), (9, "RD")
        ];
        
        for (pin, name) in &pins {
            let level = esp_idf_sys::gpio_get_level(*pin);
            // We'll just mark all as output since we set them that way
            let dir = "OUT";
            info!("  GPIO{} ({}): {} level={}", pin, name, dir, level);
        }
    }
    
    // Task info
    unsafe {
        let handle = esp_idf_sys::xTaskGetCurrentTaskHandle();
        if !handle.is_null() {
            let name = esp_idf_sys::pcTaskGetName(handle);
            if !name.is_null() {
                let task_name = std::ffi::CStr::from_ptr(name).to_string_lossy();
                info!("Current task: {}", task_name);
            }
            
            let stack_hwm = esp_idf_sys::uxTaskGetStackHighWaterMark(handle);
            info!("Stack high water mark: {} bytes", stack_hwm);
        }
    }
    
    info!("=== END DIAGNOSTICS ===");
}