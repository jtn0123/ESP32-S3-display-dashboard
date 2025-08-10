/// Deep reset functionality using RTC control registers
/// This provides a deeper reset than esp_restart() and clears HTTP server state

use log::{info, warn};

/// Performs a deep system reset using RTC control registers
/// This is more thorough than esp_restart() and clears more system state
pub fn perform_deep_reset() {
    info!("Performing deep RTC reset...");
    
    unsafe {
        // RTC_CNTL_OPTIONS0_REG address for ESP32-S3
        const RTC_CNTL_OPTIONS0_REG: *mut u32 = 0x60008000 as *mut u32;
        
        // RTC_CNTL_SW_SYS_RST bit (bit 31)
        const RTC_CNTL_SW_SYS_RST: u32 = 1 << 31;
        
        // Log the reset reason for next boot
        info!("Deep reset triggered - system will restart NOW");
        
        // Ensure log is flushed
        log::logger().flush();
        
        // Small delay to ensure UART output completes
        esp_idf_hal::delay::Ets::delay_us(1000);
        
        // Trigger the reset by writing to RTC control register
        // This causes an immediate system reset
        core::ptr::write_volatile(RTC_CNTL_OPTIONS0_REG, RTC_CNTL_SW_SYS_RST);
        
        // This line should never execute
        warn!("Reset failed - this should not happen!");
    }
}

/// Get the last reset reason as a string
pub fn get_reset_reason() -> &'static str {
    let reason = unsafe { esp_idf_sys::esp_reset_reason() };
    
    match reason {
        esp_idf_sys::esp_reset_reason_t_ESP_RST_UNKNOWN => "Unknown",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_POWERON => "Power-on",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_EXT => "External pin",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_SW => "Software reset",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_PANIC => "Panic",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_INT_WDT => "Interrupt watchdog",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_TASK_WDT => "Task watchdog",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_WDT => "Other watchdog",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_DEEPSLEEP => "Deep sleep",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_BROWNOUT => "Brownout",
        esp_idf_sys::esp_reset_reason_t_ESP_RST_SDIO => "SDIO",
        _ => "Unknown reason code",
    }
}

/// Check if the last reset was an RTC reset (indicating our deep reset worked)
pub fn was_rtc_reset() -> bool {
    // After RTC reset, the reason typically shows as power-on
    let reason = unsafe { esp_idf_sys::esp_reset_reason() };
    
    // RTC reset often appears as POWERON
    matches!(reason, 
        esp_idf_sys::esp_reset_reason_t_ESP_RST_POWERON |
        esp_idf_sys::esp_reset_reason_t_ESP_RST_WDT
    )
}