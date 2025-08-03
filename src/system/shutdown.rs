/// Graceful shutdown management for ESP32 services
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
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