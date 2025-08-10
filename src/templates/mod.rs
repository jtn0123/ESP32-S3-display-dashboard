// HTML templates for web server
// Separated from web_server.rs for better maintainability

/// Home page template with theme support
pub const HOME_PAGE_WITH_THEME: &str = include_str!("home_with_theme.html");

/// Sensor graphs page template
pub const GRAPHS_PAGE: &str = include_str!("graphs.html");

/// OTA update page template
pub const OTA_PAGE: &str = include_str!("ota.html");

/// OTA unavailable page template
pub const OTA_UNAVAILABLE_PAGE: &str = include_str!("ota_unavailable.html");

/// Generate home page with dynamic content
pub fn render_home_page(version: &str, ssid: &str, free_heap: u32, uptime_ms: u64) -> String {
    // Use the theme-enabled version
    render_home_page_with_theme(version, ssid, free_heap, uptime_ms)
}

/// Generate home page with theme support and dynamic content
pub fn render_home_page_with_theme(version: &str, ssid: &str, free_heap: u32, uptime_ms: u64) -> String {
    // Get current WiFi status
    let wifi_sta = unsafe { 
        let key = b"WIFI_STA_DEF\0";
        esp_idf_sys::esp_netif_get_handle_from_ifkey(key.as_ptr() as *const ::core::ffi::c_char) 
    };
    let is_connected = if wifi_sta.is_null() {
        false
    } else {
        unsafe {
            let mut ip_info = esp_idf_sys::esp_netif_ip_info_t::default();
            esp_idf_sys::esp_netif_get_ip_info(wifi_sta, &mut ip_info) == esp_idf_sys::ESP_OK
                && ip_info.ip.addr != 0
        }
    };
    
    // Get IP address if connected
    let ip_address = if is_connected {
        unsafe {
            let mut ip_info = esp_idf_sys::esp_netif_ip_info_t::default();
            if esp_idf_sys::esp_netif_get_ip_info(wifi_sta, &mut ip_info) == esp_idf_sys::ESP_OK {
                format!("{}.{}.{}.{}", 
                    ip_info.ip.addr & 0xff,
                    (ip_info.ip.addr >> 8) & 0xff,
                    (ip_info.ip.addr >> 16) & 0xff,
                    (ip_info.ip.addr >> 24) & 0xff)
            } else {
                "Unknown".to_string()
            }
        }
    } else {
        "Not connected".to_string()
    };
    
    // Get WiFi signal strength
    let signal_strength = if is_connected {
        unsafe {
            let mut ap_info = esp_idf_sys::wifi_ap_record_t::default();
            if esp_idf_sys::esp_wifi_sta_get_ap_info(&mut ap_info) == esp_idf_sys::ESP_OK {
                ap_info.rssi.to_string()
            } else {
                "Unknown".to_string()
            }
        }
    } else {
        "N/A".to_string()
    };
    
    // Get default config values
    let brightness = "80";
    let auto_dim = true;
    let dim_timeout = "60";
    let ota_url = "";
    let update_interval = "60";
    
    HOME_PAGE_WITH_THEME
        .replace("{{VERSION}}", version)
        .replace("{{ssid}}", ssid)
        .replace("{{is_connected}}", if is_connected { "true" } else { "" })
        .replace("{{ip_address}}", &ip_address)
        .replace("{{signal_strength}}", &signal_strength)
        .replace("{{free_heap_kb}}", &(free_heap / 1024).to_string())
        .replace("{{uptime}}", &format_uptime(uptime_ms))
        .replace("{{brightness}}", brightness)
        .replace("{{auto_dim}}", if auto_dim { "checked" } else { "" })
        .replace("{{dim_timeout}}", dim_timeout)
        .replace("{{ota_url}}", ota_url)
        .replace("{{update_interval}}", update_interval)
        .replace("{{password}}", "")
}

/// Format uptime in human-readable format
fn format_uptime(uptime_ms: u64) -> String {
    let seconds = uptime_ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours % 24, minutes % 60)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes % 60, seconds % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(1000), "1s");
        assert_eq!(format_uptime(61000), "1m 1s");
        assert_eq!(format_uptime(3661000), "1h 1m 1s");
        assert_eq!(format_uptime(90061000), "1d 1h 1m");
    }
}