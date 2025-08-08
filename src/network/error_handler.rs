use anyhow::Result;
use esp_idf_svc::http::server::Request as EspHttpRequest;
use esp_idf_svc::io::Write;
use serde::{Serialize, Deserialize};
use std::fmt;
use std::error::Error as StdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    NotFound,
    BadRequest,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::BadRequest => "BAD_REQUEST",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub request_id: String,
    pub timestamp: u64,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
            field: None,
            request_id: generate_request_id(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ApiError,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: ApiError::new(code, message),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    pub fn send<T>(self, req: EspHttpRequest<T>) -> Result<(), Box<dyn std::error::Error>> 
    where 
        T: esp_idf_svc::http::server::Connection,
        <T as esp_idf_svc::io::ErrorType>::Error: StdError + 'static
    {
        let json = serde_json::to_string(&self)?;
        let status_code = match self.error.code.as_str() {
            "BAD_REQUEST" => 400,
            "NOT_FOUND" => 404,
            _ => 500,
        };
        // Guard against double send: try to write once; if it fails with already sent, just log
        match req.into_status_response(status_code) {
            Ok(mut response) => {
                let _ = response.write_all(json.as_bytes());
            }
            Err(e) => {
                log::warn!("ErrorResponse send skipped: response already committed? err={:?}", e);
            }
        }
        Ok(())
    }
}

fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32;
    
    format!("req_{:08x}{:04x}", timestamp, count & 0xFFFF)
}

