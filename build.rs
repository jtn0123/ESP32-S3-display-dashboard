use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Necessary for ESP-IDF
    embuild::espidf::sysenv::output();
    println!("cargo:rerun-if-changed=wifi_config.h");
    // The restart token may come from the environment instead of wifi_config.h
    // (CI and test runs set it that way), so rebuild when it changes.
    println!("cargo:rerun-if-env-changed=RESTART_TOKEN");

    // Add crash log helper for better panic diagnostics
    println!("cargo:rustc-link-arg=-Wl,--undefined=esp_backtrace_print_app_description");
    
    // Read WiFi configuration if it exists
    let wifi_config_path = "wifi_config.h";
    let wifi_config_contents = if Path::new(wifi_config_path).exists() {
        Some(fs::read_to_string(wifi_config_path)?)
    } else {
        None
    };

    // Restart-endpoint token. Provisioned per build, never defaulted: an
    // absent token disables /restart and /api/restart in the firmware rather
    // than falling back to a shared value baked into a public repository.
    // The process environment wins over wifi_config.h so CI can inject one
    // without writing the file.
    let restart_token = std::env::var("RESTART_TOKEN").ok().filter(|t| !t.is_empty()).or_else(|| {
        wifi_config_contents
            .as_deref()
            .and_then(|c| c.lines().find(|l| l.contains("#define RESTART_TOKEN")))
            .and_then(|line| line.split('"').nth(1))
            .map(str::to_owned)
            .filter(|t| !t.is_empty())
    });
    match restart_token {
        Some(token) => println!("cargo:rustc-env=RESTART_TOKEN={token}"),
        None => {
            // Emit nothing, so option_env!("RESTART_TOKEN") resolves to None.
            println!(
                "cargo:warning=RESTART_TOKEN not set (no env var, no #define in wifi_config.h). \
                 The /restart and /api/restart endpoints will be DISABLED in this build."
            );
        }
    }

    // OTA upload password, same contract as the restart token above. This one
    // guards firmware upload, so an unprovisioned build closing the endpoint
    // matters more, not less.
    println!("cargo:rerun-if-env-changed=OTA_PASSWORD");
    let ota_password = std::env::var("OTA_PASSWORD").ok().filter(|p| !p.is_empty()).or_else(|| {
        wifi_config_contents
            .as_deref()
            .and_then(|c| c.lines().find(|l| l.contains("#define OTA_PASSWORD")))
            .and_then(|line| line.split('"').nth(1))
            .map(str::to_owned)
            .filter(|p| !p.is_empty())
    });
    match ota_password {
        Some(password) => println!("cargo:rustc-env=OTA_PASSWORD={password}"),
        None => {
            println!(
                "cargo:warning=OTA_PASSWORD not set (no env var, no #define in wifi_config.h). \
                 The /ota/update endpoint will be DISABLED in this build."
            );
        }
    }

    if let Some(contents) = wifi_config_contents.as_deref() {
        // Set SSID/PASSWORD env vars without emitting cargo warnings on success
        // Parse SSID
        if let Some(ssid_line) = contents.lines().find(|l| l.contains("#define WIFI_SSID")) {
            if let Some(ssid) = ssid_line.split('"').nth(1) {
                println!("cargo:rustc-env=WIFI_SSID={ssid}");
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