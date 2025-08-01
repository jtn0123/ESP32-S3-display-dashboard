use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use crate::network::sse_broadcaster;

const MAX_LOG_LINES: usize = 10000;
const LOG_BATCH_SIZE: usize = 50;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub module: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::Debug => "#6b7280",
            LogLevel::Info => "#10b981",
            LogLevel::Warn => "#f59e0b",
            LogLevel::Error => "#ef4444",
        }
    }
}

pub struct LogStreamer {
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    telnet_buffer: Option<Arc<Mutex<VecDeque<String>>>>,
}

impl LogStreamer {
    pub fn new(telnet_buffer: Option<Arc<Mutex<VecDeque<String>>>>) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES))),
            telnet_buffer,
        }
    }

    pub fn add_entry(&self, level: LogLevel, message: String, module: Option<String>) {
        let entry = LogEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            level,
            message,
            module,
        };

        let mut buffer = self.buffer.lock().unwrap();
        if buffer.len() >= MAX_LOG_LINES {
            buffer.pop_front();
        }
        buffer.push_back(entry.clone());

        // Broadcast to WebSocket clients
        self.broadcast_log_entry(&entry);
    }

    pub fn get_recent_logs(&self, count: usize) -> Vec<LogEntry> {
        let buffer = self.buffer.lock().unwrap();
        buffer.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }

    pub fn clear_logs(&self) {
        self.buffer.lock().unwrap().clear();
    }

    pub fn sync_from_telnet(&self) {
        if let Some(ref telnet_buffer) = self.telnet_buffer {
            if let Ok(telnet_logs) = telnet_buffer.try_lock() {
                // Parse telnet logs and add to our buffer
                let lines: Vec<String> = telnet_logs.iter().take(100).cloned().collect();
                for line in lines {
                    self.parse_and_add_telnet_log(&line);
                }
            }
        }
    }

    fn parse_and_add_telnet_log(&self, line: &str) {
        // Parse telnet log format: "[timestamp] LEVEL message"
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            let level = match parts[1] {
                "DEBUG" => LogLevel::Debug,
                "INFO" => LogLevel::Info,
                "WARN" => LogLevel::Warn,
                "ERROR" => LogLevel::Error,
                _ => LogLevel::Info,
            };

            self.add_entry(level, parts[2].to_string(), None);
        }
    }

    fn broadcast_log_entry(&self, entry: &LogEntry) {
        let message = serde_json::json!({
            "type": "log",
            "data": {
                "timestamp": entry.timestamp,
                "level": entry.level.as_str(),
                "color": entry.level.color(),
                "message": entry.message,
                "module": entry.module,
            }
        });

        if let Ok(_json) = serde_json::to_string(&message) {
            let sse_broadcaster = sse_broadcaster::init();
            sse_broadcaster.send_log_event(&entry);
        }
    }

    pub fn start_telnet_sync_task(&self) {
        let _buffer = self.buffer.clone();
        let _telnet_buffer = self.telnet_buffer.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                if let Some(ref tb) = _telnet_buffer {
                    if let Ok(_telnet_logs) = tb.try_lock() {
                        // Sync new logs from telnet
                        // Implementation depends on telnet buffer structure
                    }
                }
            }
        });
    }
}

// Global log streamer instance
static mut LOG_STREAMER: Option<Arc<LogStreamer>> = None;

pub fn init(telnet_buffer: Option<Arc<Mutex<VecDeque<String>>>>) -> Arc<LogStreamer> {
    unsafe {
        if LOG_STREAMER.is_none() {
            LOG_STREAMER = Some(Arc::new(LogStreamer::new(telnet_buffer)));
        }
        LOG_STREAMER.as_ref().unwrap().clone()
    }
}

pub fn log_info(message: impl Into<String>) {
    unsafe {
        if let Some(ref streamer) = LOG_STREAMER {
            streamer.add_entry(LogLevel::Info, message.into(), None);
        }
    }
}

pub fn log_error(message: impl Into<String>) {
    unsafe {
        if let Some(ref streamer) = LOG_STREAMER {
            streamer.add_entry(LogLevel::Error, message.into(), None);
        }
    }
}

pub fn log_warn(message: impl Into<String>) {
    unsafe {
        if let Some(ref streamer) = LOG_STREAMER {
            streamer.add_entry(LogLevel::Warn, message.into(), None);
        }
    }
}

pub fn log_debug(message: impl Into<String>) {
    unsafe {
        if let Some(ref streamer) = LOG_STREAMER {
            streamer.add_entry(LogLevel::Debug, message.into(), None);
        }
    }
}