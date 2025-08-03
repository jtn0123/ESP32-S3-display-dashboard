use esp_idf_svc::http::server::Configuration;
use std::time::Duration;

/// Create optimized HTTP server configuration to prevent socket exhaustion and memory issues
pub fn create_http_config() -> Configuration {
    Configuration {
        stack_size: 16384,       // CRITICAL: Increased from 8KB to prevent stack overflow
        max_uri_handlers: 40,    // Support all routes with headroom
        max_open_sockets: 4,     // Reduced from 7 to save memory
        max_resp_headers: 8,     // Reduced from 10 to save memory
        
        // Enable LRU purging to automatically close old connections
        lru_purge_enable: true,
        
        // Add timeouts if available in the bindings
        // recv_wait_timeout: Some(Duration::from_secs(5)),
        // send_wait_timeout: Some(Duration::from_secs(5)),
        
        // Note: Some fields like recv_wait_timeout might not be available
        // in the Rust bindings. We handle connection management by adding
        // "Connection: close" headers instead.
        
        // Other settings use defaults
        ..Default::default()
    }
}

/// Create enhanced HTTP server configuration with larger stack for complex handlers
pub fn create_enhanced_http_config() -> Configuration {
    let mut config = create_http_config();
    config.stack_size = 16384; // Larger stack for enhanced handlers
    config
}