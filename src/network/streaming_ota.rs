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
    let template = if has_ota_manager {
        crate::templates::OTA_PAGE
    } else {
        crate::templates::OTA_UNAVAILABLE_PAGE
    };
    
    // Prepend a lightweight global navbar to ensure consistency
    let navbar = r#"
    <nav style=\"background:#1a1a1a; border-bottom:1px solid #374151; padding:.75rem 1rem; display:flex; gap:.5rem; justify-content:center\">
        <a href=\"/\" style=\"color:#9ca3af; text-decoration:none; padding:.25rem .5rem\">Home</a>
        <a href=\"/dashboard\" style=\"color:#9ca3af; text-decoration:none; padding:.25rem .5rem\">Dashboard</a>
        <a href=\"/logs\" style=\"color:#9ca3af; text-decoration:none; padding:.25rem .5rem\">Logs</a>
        <a href=\"/files\" style=\"color:#9ca3af; text-decoration:none; padding:.25rem .5rem\">Files</a>
        <a href=\"/ota\" style=\"color:#60a5fa; text-decoration:none; padding:.25rem .5rem; background:#2a2a2a; border-radius:6px\">Update</a>
        <a href=\"/control\" style=\"color:#9ca3af; text-decoration:none; padding:.25rem .5rem\">Control</a>
        <a href=\"/dev\" style=\"color:#9ca3af; text-decoration:none; padding:.25rem .5rem\">Dev Tools</a>
    </nav>
    "#;
    
    // Merge navbar + template for streaming
    let merged = [navbar.as_bytes(), template.as_bytes()].concat();
    
    // Stream in 1KB chunks to avoid large allocations
    const CHUNK_SIZE: usize = 1024;
    let bytes = merged.as_slice();
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