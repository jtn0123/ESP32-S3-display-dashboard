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

pub fn validate_brightness(_brightness: u8) -> Result<()> {
    // Brightness is u8, so it's always 0-255
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
    fn test_validate_filename() {
        assert!(validate_filename("config.json").is_ok());
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("file/name").is_err());
        assert!(validate_filename("").is_err());
    }
}