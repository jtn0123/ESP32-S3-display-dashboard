/// Error handling wrapper for HTTP handlers
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Wrap a handler with comprehensive error handling
pub fn wrap_handler<F, R>(
    handler_name: &'static str,
    handler: F,
) -> impl Fn(Request<&mut EspHttpConnection>) -> Result<(), anyhow::Error>
where
    F: Fn(Request<&mut EspHttpConnection>) -> Result<R, Box<dyn std::error::Error>> + Send + 'static,
    R: Send + 'static,
{
    move |req| {
        // Log handler entry
        log::debug!("{} handler called", handler_name);
        
        // Wrap in panic handler
        match catch_unwind(AssertUnwindSafe(|| handler(req))) {
            Ok(result) => {
                match result {
                    Ok(_) => {
                        log::debug!("{} handler completed successfully", handler_name);
                        Ok(())
                    }
                    Err(e) => {
                        log::error!("{} handler error: {}", handler_name, e);
                        Err(anyhow::anyhow!("{} failed: {}", handler_name, e))
                    }
                }
            }
            Err(panic_info) => {
                // Log panic details
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                
                log::error!("PANIC in {} handler: {}", handler_name, panic_msg);
                crate::memory_diagnostics::log_memory_state(&format!("PANIC-{}", handler_name));
                
                Err(anyhow::anyhow!("Internal server error in {}", handler_name))
            }
        }
    }
}

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

/// Log and create error response
pub fn handle_error(
    req: Request<&mut EspHttpConnection>,
    error: Box<dyn std::error::Error>,
    handler_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    log::error!("{} error: {}", handler_name, error);
    error_response(req, 500, &format!("Internal server error in {}", handler_name))
}