use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple rate limiter for ESP32
/// Tracks request counts per IP to prevent overloading
pub struct RateLimiter {
    requests: Mutex<HashMap<String, RequestInfo>>,
    max_requests: u32,
    window: Duration,
}

struct RequestInfo {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window: Duration::from_secs(window_seconds),
        }
    }
    
    /// Check if a request from the given IP should be allowed
    pub fn check_rate_limit(&self, client_ip: &str) -> bool {
        let mut requests = match self.requests.lock() {
            Ok(guard) => guard,
            Err(_) => return true, // Allow on lock failure
        };
        
        let now = Instant::now();
        
        // Clean up old entries
        requests.retain(|_, info| now.duration_since(info.window_start) < self.window);
        
        // Check or create entry for this IP
        let info = requests.entry(client_ip.to_string()).or_insert(RequestInfo {
            count: 0,
            window_start: now,
        });
        
        // Reset window if expired
        if now.duration_since(info.window_start) >= self.window {
            info.count = 0;
            info.window_start = now;
        }
        
        // Check rate limit
        if info.count >= self.max_requests {
            false
        } else {
            info.count += 1;
            true
        }
    }
    
    /// Get current request count for an IP
    pub fn get_request_count(&self, client_ip: &str) -> u32 {
        match self.requests.lock() {
            Ok(guard) => guard.get(client_ip).map(|info| info.count).unwrap_or(0),
            Err(_) => 0,
        }
    }
}

/// Create a rate limiter instance
/// Should be stored in your server context
pub fn create_rate_limiter() -> RateLimiter {
    RateLimiter::new(10, 60) // 10 requests per minute
}

/// Middleware function to check rate limits
pub fn check_rate_limit_with_limiter(
    limiter: &RateLimiter,
    req: &esp_idf_svc::http::server::Request<&mut esp_idf_svc::http::server::EspHttpConnection>
) -> Result<bool, Box<dyn std::error::Error>> {
    // Get client IP from request
    let client_ip = get_client_ip(req).unwrap_or_else(|| "unknown".to_string());
    
    // Check rate limit
    if limiter.check_rate_limit(&client_ip) {
        Ok(true)
    } else {
        // Log rate limit exceeded
        log::warn!("Rate limit exceeded for IP: {}", client_ip);
        Ok(false)
    }
}

/// Extract client IP from request
fn get_client_ip(req: &esp_idf_svc::http::server::Request<&mut esp_idf_svc::http::server::EspHttpConnection>) -> Option<String> {
    // Try to get from X-Forwarded-For header first
    if let Some(forwarded) = req.header("X-Forwarded-For") {
        if let Some(ip) = forwarded.split(',').next() {
            return Some(ip.trim().to_string());
        }
    }
    
    // Try X-Real-IP
    if let Some(real_ip) = req.header("X-Real-IP") {
        return Some(real_ip.to_string());
    }
    
    // For ESP32, we might not have direct socket access
    // Return a placeholder for now
    Some("local".to_string())
}

/// Helper to send rate limit error response
pub fn send_rate_limit_error(req: esp_idf_svc::http::server::Request<&mut esp_idf_svc::http::server::EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    use esp_idf_svc::io::Write;
    
    let headers = [
        ("Content-Type", "application/json"),
        ("Retry-After", "60"),
    ];
    
    let mut response = req.into_response(429, Some("Too Many Requests"), &headers)?;
    response.write_all(br#"{"error":"Rate limit exceeded. Please try again later."}"#)?;
    
    Ok(())
}