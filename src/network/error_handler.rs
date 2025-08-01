use anyhow::Result;
use esp_idf_svc::http::server::Request as EspHttpRequest;
use esp_idf_svc::io::Write;
use serde::{Serialize, Deserialize};
use std::fmt;
use std::error::Error as StdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ValidationFailed,
    NotFound,
    InternalError,
    BadRequest,
    Unauthorized,
    ServiceUnavailable,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ValidationFailed => "VALIDATION_FAILED",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            ErrorCode::ValidationFailed => 400,
            ErrorCode::BadRequest => 400,
            ErrorCode::Unauthorized => 401,
            ErrorCode::NotFound => 404,
            ErrorCode::InternalError => 500,
            ErrorCode::ServiceUnavailable => 503,
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
                .unwrap()
                .as_secs(),
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
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

    pub fn validation_failed(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ApiError::new(ErrorCode::ValidationFailed, message)
                .with_field(field),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ServiceUnavailable, message)
    }

    pub fn send<T>(self, req: EspHttpRequest<T>) -> Result<(), Box<dyn std::error::Error>> 
    where 
        T: esp_idf_svc::http::server::Connection,
        <T as esp_idf_svc::io::ErrorType>::Error: StdError + 'static
    {
        let json = serde_json::to_string(&self)?;
        let status_code = match self.error.code.as_str() {
            "VALIDATION_FAILED" | "BAD_REQUEST" => 400,
            "UNAUTHORIZED" => 401,
            "NOT_FOUND" => 404,
            "INTERNAL_ERROR" => 500,
            "SERVICE_UNAVAILABLE" => 503,
            _ => 500,
        };

        let mut response = req.into_status_response(status_code)?;
        response.write_all(json.as_bytes())?;
        Ok(())
    }
}

fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
    
    format!("req_{:08x}{:04x}", timestamp, count & 0xFFFF)
}

