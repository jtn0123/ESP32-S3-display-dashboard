/// Configuration structures that can be tested independently
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayConfig {
    pub width: u16,
    pub height: u16,
    pub brightness: u8,
    pub auto_dim: bool,
    pub dim_timeout_secs: u32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: 320,
            height: 170,
            brightness: 100,
            auto_dim: true,
            dim_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkConfig {
    pub wifi_ssid: String,
    pub wifi_password: String,
    pub mdns_name: String,
    pub telnet_enabled: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            mdns_name: "esp32-display".to_string(),
            telnet_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_config_serialization() {
        let config = DisplayConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DisplayConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_network_config_defaults() {
        let config = NetworkConfig::default();
        assert_eq!(config.mdns_name, "esp32-display");
        assert!(config.telnet_enabled);
    }
    
    #[test]
    fn test_brightness_bounds() {
        let mut config = DisplayConfig::default();
        config.brightness = 255; // Should be clamped to 100
        assert!(config.brightness <= 100);
    }
}