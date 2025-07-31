// HTML templates for web server
// Separated from web_server.rs for better maintainability

/// Home page template
pub const HOME_PAGE: &str = include_str!("home.html");

/// OTA update page template
pub const OTA_PAGE: &str = include_str!("ota.html");

/// OTA unavailable page template
pub const OTA_UNAVAILABLE_PAGE: &str = include_str!("ota_unavailable.html");

/// Generate home page with dynamic content
pub fn render_home_page(version: &str, ssid: &str, free_heap: u32, uptime_ms: u64) -> String {
    HOME_PAGE
        .replace("{{VERSION}}", version)
        .replace("{{SSID}}", ssid)
        .replace("{{FREE_HEAP}}", &(free_heap / 1024).to_string())  // Convert bytes to KB
        .replace("{{UPTIME}}", &format_uptime(uptime_ms))
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