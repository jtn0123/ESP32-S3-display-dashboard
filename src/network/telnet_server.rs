use anyhow::Result;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Ring buffer for storing recent log messages
pub(super) struct LogBuffer {
    buffer: Vec<String>,
    capacity: usize,
    write_index: usize,
}

impl LogBuffer {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            write_index: 0,
        }
    }
    
    pub(super) fn push(&mut self, message: String) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(message);
        } else {
            self.buffer[self.write_index] = message;
            self.write_index = (self.write_index + 1) % self.capacity;
        }
    }
    
    pub(super) fn get_all(&self) -> Vec<String> {
        if self.buffer.len() < self.capacity {
            self.buffer.clone()
        } else {
            // Return in correct order when buffer has wrapped
            let mut result = Vec::with_capacity(self.capacity);
            for i in 0..self.capacity {
                let idx = (self.write_index + i) % self.capacity;
                result.push(self.buffer[idx].clone());
            }
            result
        }
    }
    
    pub(super) fn get_recent(&self, count: usize) -> Vec<String> {
        let all_logs = self.get_all();
        let start = all_logs.len().saturating_sub(count);
        all_logs[start..].to_vec()
    }
}

/// Telnet server for remote log streaming
pub struct TelnetLogServer {
    log_buffer: Arc<Mutex<LogBuffer>>,
    clients: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
    port: u16,
    shutdown_signal: Option<crate::system::ShutdownSignal>,
    total_connections: Arc<Mutex<u64>>,
}

impl TelnetLogServer {
    pub fn new(port: u16) -> Self {
        Self {
            log_buffer: Arc::new(Mutex::new(LogBuffer::new(100))), // Keep last 100 messages
            clients: Arc::new(Mutex::new(Vec::new())),
            port,
            shutdown_signal: None,
            total_connections: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Set shutdown signal for graceful shutdown
    pub fn set_shutdown_signal(&mut self, signal: crate::system::ShutdownSignal) {
        self.shutdown_signal = Some(signal);
    }
    
    /// Start the telnet server in a background thread
    pub fn start(self: Arc<Self>) -> Result<()> {
        let server = self.clone();
        
        // Start the TCP listener thread
        thread::Builder::new()
            .name("telnet-server".to_string())
            .stack_size(4096)
            .spawn(move || {
                if let Err(e) = server.run_server() {
                    log::error!("Telnet server error: {:?}", e);
                }
            })?;
        
        log::info!("Telnet log server started on port {}", self.port);
        log::info!("Connect with: telnet <device-ip> {}", self.port);
        
        Ok(())
    }
    
    /// Main server loop
    fn run_server(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
        listener.set_nonblocking(true)?; // Non-blocking for shutdown check
        
        log::info!("Telnet server listening on port {}", self.port);
        
        loop {
            // Check for shutdown signal
            if let Some(ref signal) = self.shutdown_signal {
                if signal.is_shutdown_requested() {
                    log::info!("Telnet server shutting down...");
                    break;
                }
            }
            match listener.accept() {
                Ok((stream, addr)) => {
                    log::info!("Telnet client connected from {}", addr);
                    
                    // Set TCP options for better real-time performance
                    stream.set_nodelay(true)?;
                    stream.set_nonblocking(false)?;
                    
                    let stream = Arc::new(Mutex::new(stream));
                    
                    // Send welcome message and recent logs
                    if let Ok(mut s) = stream.lock() {
                        let _ = writeln!(s, "\r\n=== ESP32-S3 Dashboard Remote Log ===\r\n");
                        let _ = writeln!(s, "Firmware: {}\r", crate::version::DISPLAY_VERSION);
                        let _ = writeln!(s, "Free heap: {} KB\r", unsafe { esp_idf_sys::esp_get_free_heap_size() } / 1024);
                        let _ = writeln!(s, "\r\nConnected to device. Streaming live logs...\r\n");
                        let _ = writeln!(s, "TIP: Use monitor-telnet.py for filtering and commands\r\n");
                        
                        // Send recent log history
                        if let Ok(buffer) = self.log_buffer.lock() {
                            let _ = writeln!(s, "--- Recent log history ---");
                            for msg in buffer.get_all() {
                                let _ = write!(s, "{msg}");
                            }
                            let _ = writeln!(s, "--- End of history ---\r\n");
                        }
                    }
                    
                    // Add to active clients
                    if let Ok(mut clients) = self.clients.lock() {
                        clients.push(stream);
                    }
                    
                    // Increment total connections
                    if let Ok(mut total) = self.total_connections.lock() {
                        *total += 1;
                    }
                    
                    // Update metrics
                    self.update_metrics();
                    
                    // Clean up disconnected clients
                    self.cleanup_clients();
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        log::error!("Accept error: {:?}", e);
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
        
        // Disconnect all clients on shutdown
        if let Ok(mut clients) = self.clients.lock() {
            for client in clients.iter() {
                if let Ok(mut stream) = client.lock() {
                    let _ = writeln!(stream, "\r\n\r\n=== Server shutting down ===\r\n");
                }
            }
            clients.clear();
        }
        
        log::info!("Telnet server stopped");
        Ok(())
    }
    
    /// Remove disconnected clients
    fn cleanup_clients(&self) {
        if let Ok(mut clients) = self.clients.lock() {
            clients.retain(|client| {
                if let Ok(stream) = client.lock() {
                    // Try to peek to check if connection is alive
                    let mut buf = [0; 1];
                    match stream.peek(&mut buf) {
                        Ok(0) => false, // Connection closed
                        Ok(_) => true,  // Data available or would block
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
                        Err(_) => false, // Other errors mean disconnected
                    }
                } else {
                    false
                }
            });
            
            // Update metrics after cleanup
            self.update_metrics();
        }
    }
    
    /// Log a message to buffer and all connected clients
    pub fn log_message(&self, level: &str, message: &str) {
        let timestamp = esp_idf_svc::systime::EspSystemTime.now().as_secs();
        let formatted = format!("[{timestamp:10}] {level:5} {message}\r\n");
        
        // Add to buffer
        if let Ok(mut buffer) = self.log_buffer.lock() {
            buffer.push(formatted.clone());
        }
        
        // Send to all connected clients
        if let Ok(clients) = self.clients.lock() {
            for client in clients.iter() {
                if let Ok(mut stream) = client.lock() {
                    let _ = stream.write_all(formatted.as_bytes());
                    let _ = stream.flush();
                }
            }
        }
    }
    
    /// Get recent logs from the buffer
    #[allow(dead_code)]
    pub fn get_recent_logs(&self, count: usize) -> Vec<String> {
        if let Ok(buffer) = self.log_buffer.lock() {
            buffer.get_recent(count)
        } else {
            Vec::new()
        }
    }
    
    /// Update telnet connection metrics
    fn update_metrics(&self) {
        let active_clients = self.clients.lock().map(|c| c.len() as u32).unwrap_or(0);
        let total_connections = self.total_connections.lock().map(|t| *t).unwrap_or(0);
        
        // Update metrics
        if let Ok(mut metrics) = crate::metrics::metrics().lock() {
            metrics.update_telnet_connections(active_clients, total_connections);
        }
    }
}

/// Macro to log to telnet server if available
#[macro_export]
macro_rules! telnet_log {
    ($server:expr, $level:expr, $($arg:tt)*) => {
        if let Some(ref server) = $server {
            server.log_message($level, &format!($($arg)*));
        }
    };
}