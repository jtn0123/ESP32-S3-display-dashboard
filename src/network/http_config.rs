use esp_idf_svc::http::server::Configuration;

/// Create optimized HTTP server configuration to prevent socket exhaustion and memory issues
pub fn create_http_config() -> Configuration {
    // Delegate to our centralized stable server config
    crate::network::server_config::StableServerConfig::create()
}