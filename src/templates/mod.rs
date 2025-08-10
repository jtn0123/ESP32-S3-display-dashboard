// HTML templates for web server
// Separated from web_server.rs for better maintainability

/// Sensor graphs page template
pub const GRAPHS_PAGE: &str = include_str!("graphs.html");

/// OTA update page template
pub const OTA_PAGE: &str = include_str!("ota.html");

/// OTA unavailable page template
pub const OTA_UNAVAILABLE_PAGE: &str = include_str!("ota_unavailable.html");

// Removed: home page rendering helpers (unused)

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