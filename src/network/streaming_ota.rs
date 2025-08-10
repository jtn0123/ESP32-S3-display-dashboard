/// Streaming OTA page handler to prevent memory issues
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;

/// Stream the OTA page in chunks
pub fn handle_ota_streaming(req: Request<&mut EspHttpConnection>, has_ota_manager: bool) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Streaming OTA page - manager available: {}", has_ota_manager);
    crate::memory_diagnostics::log_memory_state("OTA streaming - start");
    
    // Create response
    let mut response = req.into_response(
        200,
        Some("OK"),
        &[
            ("Content-Type", "text/html; charset=utf-8"),
            ("Connection", "close"),
            // Don't use compression for OTA page
        ]
    )?;
    
    // Choose which template to stream
    let template_raw = if has_ota_manager {
        crate::templates::OTA_PAGE
    } else {
        crate::templates::OTA_UNAVAILABLE_PAGE
    };

    // Build shared navbar using partial with active state
    let mut navbar = include_str!("../templates/partials/navbar.html").to_string();
    navbar = navbar
        .replace("{{HOME_ACTIVE}}", "")
        .replace("{{DASH_ACTIVE}}", "")
        .replace("{{LOGS_ACTIVE}}", "")
        .replace("{{FILES_ACTIVE}}", "")
        .replace("{{OTA_ACTIVE}}", "class=\\\"active\\\"")
        .replace("{{DEV_ACTIVE}}", "");

    // Minimal navbar CSS to match global style (uses OTA page CSS variables)
    const NAV_CSS: &str = r#"
    <style>
      .navbar { background: var(--bg-card); border-bottom: 1px solid var(--border); padding: 1rem; display: flex; justify-content: center; }
      .nav-links { display: flex; gap: 1rem; }
      .nav-links a { color: var(--text-dim); text-decoration: none; padding: 0.5rem 0.75rem; border-radius: 6px; transition: background-color .2s, color .2s; }
      .nav-links a:hover { color: var(--text); background: var(--bg-hover); }
      .nav-links a.active { color: var(--accent); background: var(--bg-hover); }
    </style>
    "#;

    // Inject navbar and CSS into the template
    let mut html = template_raw.to_string();
    if !html.contains("<nav class=\"navbar\">") {
        html = html.replacen("<body>", &format!("<body>\n{}", navbar), 1);
    }
    if !html.contains(".navbar") {
        html = html.replacen("<head>", &format!("<head>\n{}", NAV_CSS), 1);
    }

    // Stream in 1KB chunks to avoid large allocations
    let bytes = html.as_bytes();
    
    // Stream in 1KB chunks to avoid large allocations
    const CHUNK_SIZE: usize = 1024;
    let mut offset = 0;
    
    while offset < bytes.len() {
        let end = (offset + CHUNK_SIZE).min(bytes.len());
        let chunk = &bytes[offset..end];
        
        match response.write_all(chunk) {
            Ok(_) => {
                offset = end;
                
                // Log progress for large pages
                if bytes.len() > 5000 && offset % 5000 == 0 {
                    log::debug!("OTA page progress: {}/{} bytes", offset, bytes.len());
                }
            }
            Err(e) => {
                log::error!("Failed to write OTA chunk at offset {}: {:?}", offset, e);
                return Err(Box::new(e));
            }
        }
        
        // Yield to other tasks periodically
        if offset % 4096 == 0 {
            unsafe {
                esp_idf_sys::vTaskDelay(1);
            }
        }
    }
    
    log::info!("OTA page streamed successfully ({} bytes)", bytes.len());
    crate::memory_diagnostics::log_memory_state("OTA streaming - complete");
    
    Ok(())
}