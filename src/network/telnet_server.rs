use anyhow::Result;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Ring buffer for storing recent log messages
struct LogBuffer {
    buffer: Vec<String>,
    capacity: usize,
    write_index: usize,
}

impl LogBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            write_index: 0,
        }
    }
    
    fn push(&mut self, message: String) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(message);
        } else {
            self.buffer[self.write_index] = message;
            self.write_index = (self.write_index + 1) % self.capacity;
        }
    }
    
    fn get_all(&self) -> Vec<String> {
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
}

/// Telnet server for remote log streaming
pub struct TelnetLogServer {
    log_buffer: Arc<Mutex<LogBuffer>>,
    clients: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
    port: u16,
}

impl TelnetLogServer {
    pub fn new(port: u16) -> Self {
        Self {
            log_buffer: Arc::new(Mutex::new(LogBuffer::new(100))), // Keep last 100 messages
            clients: Arc::new(Mutex::new(Vec::new())),
            port,
        }
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
        listener.set_nonblocking(false)?;
        
        log::info!("Telnet server listening on port {}", self.port);
        
        loop {
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
                        let _ = writeln!(s, "Connected to device. Streaming live logs...\r\n");
                        
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