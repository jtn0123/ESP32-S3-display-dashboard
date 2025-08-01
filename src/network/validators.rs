use anyhow::{anyhow, Result};

pub fn validate_ssid(ssid: &str) -> Result<()> {
    if ssid.is_empty() {
        return Err(anyhow!("WiFi SSID cannot be empty"));
    }
    if ssid.len() > 32 {
        return Err(anyhow!("WiFi SSID must be 32 characters or less"));
    }
    if ssid.chars().any(|c| c.is_control()) {
        return Err(anyhow!("WiFi SSID cannot contain control characters"));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<()> {
    if !password.is_empty() && password.len() < 8 {
        return Err(anyhow!("WiFi password must be at least 8 characters"));
    }
    if password.len() > 64 {
        return Err(anyhow!("WiFi password must be 64 characters or less"));
    }
    Ok(())
}

pub fn validate_brightness(_brightness: u8) -> Result<()> {
    // Brightness is u8, so it's always 0-255
    Ok(())
}

pub fn validate_url(url: &str) -> Result<()> {
    if url.is_empty() {
        return Err(anyhow!("URL cannot be empty"));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(anyhow!("URL must start with http:// or https://"));
    }
    if url.len() > 256 {
        return Err(anyhow!("URL must be 256 characters or less"));
    }
    Ok(())
}

pub fn validate_filename(filename: &str) -> Result<()> {
    if filename.is_empty() {
        return Err(anyhow!("Filename cannot be empty"));
    }
    if filename.contains("..") {
        return Err(anyhow!("Filename cannot contain '..'"));
    }
    if filename.chars().any(|c| matches!(c, '/' | '\\' | '\0')) {
        return Err(anyhow!("Filename contains invalid characters"));
    }
    if filename.len() > 128 {
        return Err(anyhow!("Filename must be 128 characters or less"));
    }
    Ok(())
}

pub fn validate_json(json_str: &str) -> Result<serde_json::Value> {
    serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Invalid JSON: {}", e))
}

pub fn sanitize_log_output(log: &str) -> String {
    // Remove any potential HTML/script injection
    log.replace('<', "&lt;")
       .replace('>', "&gt;")
       .replace('"', "&quot;")
       .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ssid() {
        assert!(validate_ssid("MyNetwork").is_ok());
        assert!(validate_ssid("").is_err());
        assert!(validate_ssid("a".repeat(33).as_str()).is_err());
        assert!(validate_ssid("Network\0").is_err());
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("").is_ok()); // Empty is allowed
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("short").is_err());
        assert!(validate_password("a".repeat(65).as_str()).is_err());
    }

    #[test]
    fn test_validate_filename() {
        assert!(validate_filename("config.json").is_ok());
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("file/name").is_err());
        assert!(validate_filename("").is_err());
    }
}