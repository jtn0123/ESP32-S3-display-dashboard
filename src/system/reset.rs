use log::info;

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
