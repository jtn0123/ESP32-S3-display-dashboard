use esp_idf_svc::http::server::Configuration;

/// Create optimized HTTP server configuration to prevent socket exhaustion
pub fn create_http_config() -> Configuration {
    Configuration {
        stack_size: 8192,        // Stack size for request handlers
        max_uri_handlers: 40,    // Support all routes with headroom
        max_open_sockets: 7,     // LWIP allows max 10, HTTP server uses 3 internally
        max_resp_headers: 10,    // Reasonable limit for response headers
        
        // Enable LRU purging to automatically close old connections
        lru_purge_enable: true,
        
        // Note: keep_alive_enable, recv_wait_timeout, send_wait_timeout
        // are not available in the Rust bindings. We handle connection
        // management by adding "Connection: close" headers instead.
        
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