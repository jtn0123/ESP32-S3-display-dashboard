/// Error handling wrapper for HTTP handlers
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};

// Removed: wrapper not adopted broadly; we keep error_response only for lightweight usage

/// Create a simple error response
pub fn error_response(
    req: Request<&mut EspHttpConnection>,
    status: u16,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = req.into_status_response(status)?;
    response.write_all(message.as_bytes())?;
    Ok(())
}

// Removed: handle_error indirection; call error_response directly where needed