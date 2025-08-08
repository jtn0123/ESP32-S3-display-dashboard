// WiFi status screen

use crate::display::{Display, Color};
use crate::ui::theme::Theme;
use super::Screen;

pub struct WiFiScreen {
    connected: bool,
    ssid: String,
    ip_address: String,
    gateway: String,
    mac: String,
    rssi: i8,
}

impl WiFiScreen {
    pub fn new() -> Self {
        Self {
            connected: false,
            ssid: "Not Connected".to_string(),
            ip_address: "0.0.0.0".to_string(),
            gateway: "0.0.0.0".to_string(),
            mac: "--:--:--:--:--:--".to_string(),
            rssi: -100,
        }
    }
}

impl Screen for WiFiScreen {
    fn title(&self) -> &str {
        "WiFi"
    }
    
    fn draw(&self, display: &mut Display, theme: &Theme) {
        // Connection status
        let status_color = if self.connected {
            theme.colors.success
        } else {
            theme.colors.error
        };
        
        display.draw_card(40, 25, 240, 45, "CONNECTION", status_color);
        display.draw_text(45, 40, if self.connected { "Connected" } else { "Disconnected" }, status_color);
        display.draw_text(45, 52, &self.ssid, theme.colors.text_secondary);
        
        // Network info
        if self.connected {
            display.draw_card(40, 75, 240, 45, "NETWORK", theme.colors.info);
            display.draw_text(45, 90, "IP:", theme.colors.text_secondary);
            display.draw_text(65, 90, &self.ip_address, theme.colors.info);
            display.draw_text(145, 90, "GW:", theme.colors.text_secondary);
            display.draw_text(170, 90, &self.gateway, theme.colors.info);
            
            // Signal strength
            display.draw_text(45, 105, "Signal:", theme.colors.text_secondary);
            display.draw_number(90, 105, self.rssi.abs() as u32, status_color);
            display.draw_text(115, 105, "dBm", theme.colors.text_secondary);
            
            // Signal bars
            let bars = match self.rssi {
                -50..=0 => 4,
                -60..=-51 => 3,
                -70..=-61 => 2,
                _ => 1,
            };
            
            for i in 0..4 {
                let height = 3 + (i * 2);
                let color = if i < bars { status_color } else { theme.colors.border };
                display.fill_rect(200 + (i * 8), 105 - height, 6, height, color);
            }
        }
        
        // Device info
        display.draw_card(40, 125, 240, 35, "OTA", theme.colors.secondary);
        display.draw_text(45, 140, "MAC:", theme.colors.text_secondary);
        display.draw_text(75, 140, &self.mac, theme.colors.text_secondary);
        display.draw_text(175, 140, if self.connected { "Ready" } else { "Offline" }, theme.colors.text_secondary);
    }
    
    fn update(&mut self) {
        // Pull latest from NetworkManager via UI glue
        if let Some(nm) = crate::network::NetworkManager::global() {
            self.connected = nm.is_connected();
            self.ssid = nm.get_ssid().to_string();
            self.ip_address = nm.get_ip().unwrap_or_default();
            self.gateway = nm.get_gateway().unwrap_or_default();
            self.mac = nm.get_mac().unwrap_or_else(|| "--:--:--:--:--:--".to_string());
            self.rssi = nm.get_signal_strength();
        }
    }
}