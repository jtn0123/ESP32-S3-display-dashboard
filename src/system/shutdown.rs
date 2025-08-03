/// Graceful shutdown management for ESP32 services
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use anyhow::Result;

/// Shutdown signal that can be shared across threads
#[derive(Clone)]
pub struct ShutdownSignal {
    shutdown_requested: Arc<AtomicBool>,
    shutdown_complete: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            shutdown_complete: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }
    
    /// Request shutdown of all services
    pub fn request_shutdown(&self) {
        log::info!("🛑 Shutdown requested");
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }
    
    /// Mark shutdown as complete
    pub fn mark_complete(&self) {
        self.shutdown_complete.store(true, Ordering::Relaxed);
    }
    
    /// Wait for shutdown to complete
    pub fn wait_for_completion(&self, timeout: Duration) -> bool {
        let start = std::time::Instant::now();
        while !self.shutdown_complete.load(Ordering::Relaxed) {
            if start.elapsed() > timeout {
                return false;
            }
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
        }
        true
    }
}

/// Manager for coordinating graceful shutdown
pub struct ShutdownManager {
    signal: ShutdownSignal,
    services: Vec<Box<dyn ShutdownHandler>>,
}

impl ShutdownManager {
    pub fn new() -> Self {
        Self {
            signal: ShutdownSignal::new(),
            services: Vec::new(),
        }
    }
    
    /// Get a clone of the shutdown signal
    pub fn get_signal(&self) -> ShutdownSignal {
        self.signal.clone()
    }
    
    /// Register a service for shutdown
    pub fn register_service(&mut self, service: Box<dyn ShutdownHandler>) {
        self.services.push(service);
    }
    
    /// Perform graceful shutdown of all services
    pub fn shutdown(&mut self) -> Result<()> {
        log::info!("🛑 Beginning graceful shutdown sequence...");
        crate::memory_diagnostics::log_memory_state("Shutdown - start");
        
        // Request shutdown
        self.signal.request_shutdown();
        
        // Give services time to see the signal
        esp_idf_hal::delay::FreeRtos::delay_ms(100);
        
        // Shutdown services in reverse order (last registered first)
        while let Some(mut service) = self.services.pop() {
            match service.shutdown() {
                Ok(_) => log::info!("✅ {} shutdown complete", service.name()),
                Err(e) => log::error!("❌ {} shutdown failed: {:?}", service.name(), e),
            }
        }
        
        // Final cleanup
        log::info!("🛑 All services shut down");
        crate::memory_diagnostics::log_memory_state("Shutdown - complete");
        
        // Mark complete
        self.signal.mark_complete();
        
        Ok(())
    }
}

/// Trait for services that need graceful shutdown
pub trait ShutdownHandler: Send {
    /// Service name for logging
    fn name(&self) -> &str;
    
    /// Perform shutdown
    fn shutdown(&mut self) -> Result<()>;
}

/// Web server shutdown handler
pub struct WebServerShutdown {
    server_handle: Option<esp_idf_svc::http::server::EspHttpServer>,
}

impl WebServerShutdown {
    pub fn new(server: esp_idf_svc::http::server::EspHttpServer) -> Self {
        Self {
            server_handle: Some(server),
        }
    }
}

impl ShutdownHandler for WebServerShutdown {
    fn name(&self) -> &str {
        "WebServer"
    }
    
    fn shutdown(&mut self) -> Result<()> {
        if let Some(server) = self.server_handle.take() {
            // Server drops automatically when out of scope
            drop(server);
            log::info!("Web server stopped");
        }
        Ok(())
    }
}

/// Telnet server shutdown handler
pub struct TelnetServerShutdown {
    shutdown_signal: ShutdownSignal,
}

impl TelnetServerShutdown {
    pub fn new(signal: ShutdownSignal) -> Self {
        Self {
            shutdown_signal: signal,
        }
    }
}

impl ShutdownHandler for TelnetServerShutdown {
    fn name(&self) -> &str {
        "TelnetServer"
    }
    
    fn shutdown(&mut self) -> Result<()> {
        // Telnet server checks shutdown signal in its loop
        log::info!("Telnet server signaled to stop");
        Ok(())
    }
}

/// WiFi shutdown handler
pub struct WiFiShutdown {
    wifi_handle: Option<Box<esp_idf_svc::wifi::EspWifi<'static>>>,
}

impl WiFiShutdown {
    pub fn new(wifi: Box<esp_idf_svc::wifi::EspWifi<'static>>) -> Self {
        Self {
            wifi_handle: Some(wifi),
        }
    }
}

impl ShutdownHandler for WiFiShutdown {
    fn name(&self) -> &str {
        "WiFi"
    }
    
    fn shutdown(&mut self) -> Result<()> {
        if let Some(mut wifi) = self.wifi_handle.take() {
            log::info!("Disconnecting WiFi...");
            let _ = wifi.disconnect();
            let _ = wifi.stop();
            log::info!("WiFi stopped");
        }
        Ok(())
    }
}

/// Display shutdown handler
pub struct DisplayShutdown {
    power_off_on_shutdown: bool,
}

impl DisplayShutdown {
    pub fn new(power_off: bool) -> Self {
        Self {
            power_off_on_shutdown: power_off,
        }
    }
}

impl ShutdownHandler for DisplayShutdown {
    fn name(&self) -> &str {
        "Display"
    }
    
    fn shutdown(&mut self) -> Result<()> {
        if self.power_off_on_shutdown {
            log::info!("Powering off display...");
            // Clear display
            // Note: Actual display manager would be passed in real implementation
        }
        Ok(())
    }
}

/// Helper to handle shutdown signals (Ctrl+C, panic, etc)
pub fn setup_shutdown_handler(shutdown_manager: Arc<Mutex<ShutdownManager>>) {
    // Register panic handler
    let shutdown_mgr = shutdown_manager.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!("🚨 PANIC: {:?}", panic_info);
        
        // Try graceful shutdown
        if let Ok(mut mgr) = shutdown_mgr.lock() {
            let _ = mgr.shutdown();
        }
        
        // Force restart after delay
        esp_idf_hal::delay::FreeRtos::delay_ms(2000);
        unsafe { esp_idf_sys::esp_restart(); }
    }));
    
    // Note: Signal handling (SIGINT, etc) doesn't apply to embedded systems
    // but we could add button press handlers here
}

/// Macro to safely shutdown on error
#[macro_export]
macro_rules! shutdown_on_error {
    ($result:expr, $shutdown_mgr:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => {
                log::error!("Fatal error: {:?}", e);
                if let Ok(mut mgr) = $shutdown_mgr.lock() {
                    let _ = mgr.shutdown();
                }
                return Err(e.into());
            }
        }
    };
}