use std::fs;
use std::path::Path;

/// Credential values that were once hardcoded in this public repository and
/// must therefore never be provisioned again. They are permanently disclosed:
/// anyone who has read the repo, or a clone or mirror of it, knows them.
const RETIRED_CREDENTIALS: &[&str] = &["esp32-restart", "esp32"];

/// Extract the value of `#define <name> "<value>"` from a C header.
///
/// Deliberately stricter than a substring search. `contains("#define NAME")`
/// also matches `// #define NAME "old"` and `#define NAME_LEGACY "..."`, and
/// since the caller takes the *first* matching line, a commented-out or
/// similarly-named directive above the real one would silently win -- which for
/// a credential means building with the wrong secret.
fn define_from_header(contents: &str, name: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        // A commented-out directive is not a directive.
        if line.starts_with("//") || line.starts_with('*') || line.starts_with("/*") {
            continue;
        }
        let Some(rest) = line.strip_prefix("#define") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix(name) else {
            continue;
        };
        // Require a separator after the macro name so RESTART_TOKEN does not
        // match RESTART_TOKEN_LEGACY.
        if !rest.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        // Exactly one double-quoted value, with nothing but whitespace before it.
        let mut parts = rest.splitn(3, '"');
        let before = parts.next()?;
        if !before.trim().is_empty() {
            continue;
        }
        let value = parts.next()?;
        return Some(value.to_string());
    }
    None
}

/// Resolve a build-time credential: process environment first (so CI can inject
/// one without writing a file), then `wifi_config.h`. Empty and retired values
/// are treated as absent, which disables the endpoint rather than shipping a
/// known-public secret.
fn resolve_credential(name: &str, header: Option<&str>) -> Option<String> {
    let value = std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| header.and_then(|c| define_from_header(c, name)))
        .filter(|v| !v.is_empty())?;

    if RETIRED_CREDENTIALS.iter().any(|retired| *retired == value) {
        println!(
            "cargo:warning={name} is set to a value that was previously hardcoded in this \
             public repository and is permanently disclosed. It has been REJECTED and the \
             corresponding endpoint is DISABLED. Generate a new value: openssl rand -hex 24"
        );
        return None;
    }
    Some(value)
}

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
    let restart_token = resolve_credential("RESTART_TOKEN", wifi_config_contents.as_deref());
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
    let ota_password = resolve_credential("OTA_PASSWORD", wifi_config_contents.as_deref());
    match ota_password {
        Some(password) => println!("cargo:rustc-env=OTA_PASSWORD={password}"),
        None => {
            println!(
                "cargo:warning=OTA_PASSWORD not set (no env var, no #define in wifi_config.h). \
                 The /ota/update endpoint will be DISABLED in this build."
            );
        }
    }

    // WiFi credentials use the same strict parser as the endpoint credentials
    // above: they had the identical substring-matching defect, where a
    // commented-out directive or a longer macro name could be selected ahead of
    // the real one.
    //
    // Unlike RESTART_TOKEN/OTA_PASSWORD these are always emitted, empty if
    // absent, because config.rs reads them with env!() rather than option_env!()
    // -- a missing variable is a compile error rather than a disabled feature.
    if wifi_config_contents.is_none() {
        println!("cargo:warning=wifi_config.h not found! Copy wifi_config.h.example to wifi_config.h and add your credentials.");
    }
    for name in ["WIFI_SSID", "WIFI_PASSWORD"] {
        match wifi_config_contents.as_deref().and_then(|c| define_from_header(c, name)) {
            Some(value) => println!("cargo:rustc-env={name}={value}"),
            None => {
                println!("cargo:rustc-env={name}=");
                if wifi_config_contents.is_some() {
                    // Deliberately does not echo the offending line: for
                    // WIFI_PASSWORD that would print the password into the
                    // build log, which CI retains and people paste into issues.
                    println!(
                        "cargo:warning={name} not found in wifi_config.h, or its #define is \
                         malformed (expected: #define {name} \"value\"). Building with an empty value."
                    );
                }
            }
        }
    }
    
    Ok(())
}