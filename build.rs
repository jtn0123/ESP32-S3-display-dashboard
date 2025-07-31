use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Necessary for ESP-IDF
    embuild::espidf::sysenv::output();
    
    // Add crash log helper for better panic diagnostics
    println!("cargo:rustc-link-arg=-Wl,--undefined=esp_backtrace_print_app_description");
    
    // Read WiFi configuration if it exists
    let wifi_config_path = "wifi_config.h";
    if Path::new(wifi_config_path).exists() {
        let contents = fs::read_to_string(wifi_config_path)?;
        println!("cargo:warning=Found wifi_config.h with {} lines", contents.lines().count());
        
        // Parse SSID
        if let Some(ssid_line) = contents.lines().find(|l| l.contains("#define WIFI_SSID")) {
            if let Some(ssid) = ssid_line.split('"').nth(1) {
                println!("cargo:rustc-env=WIFI_SSID={ssid}");
                println!("cargo:warning=Setting WIFI_SSID={ssid}");
            } else {
                println!("cargo:warning=Failed to parse WIFI_SSID from line: {ssid_line}");
            }
        } else {
            println!("cargo:warning=WIFI_SSID not found in wifi_config.h");
        }
        
        // Parse Password  
        if let Some(pass_line) = contents.lines().find(|l| l.contains("#define WIFI_PASSWORD")) {
            if let Some(pass) = pass_line.split('"').nth(1) {
                println!("cargo:rustc-env=WIFI_PASSWORD={pass}");
                println!("cargo:warning=Setting WIFI_PASSWORD=<hidden>");
            } else {
                println!("cargo:warning=Failed to parse WIFI_PASSWORD from line: {pass_line}");
            }
        } else {
            println!("cargo:warning=WIFI_PASSWORD not found in wifi_config.h");
        }
    } else {
        // Use empty defaults if no config file
        println!("cargo:rustc-env=WIFI_SSID=");
        println!("cargo:rustc-env=WIFI_PASSWORD=");
        println!("cargo:warning=wifi_config.h not found! Copy wifi_config.h.example to wifi_config.h and add your credentials.");
    }
    
    Ok(())
}