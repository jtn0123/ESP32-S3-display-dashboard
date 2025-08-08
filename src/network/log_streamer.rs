use std::sync::{Arc, Mutex, OnceLock};
use std::collections::VecDeque;

// Keep memory use bounded. Target ~2K entries by default; adjust if PSRAM abundant.
const MAX_LOG_LINES: usize = 2000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
    pub module: Option<String>,
}


pub struct LogStreamer {
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogStreamer {
    pub fn new(_telnet_buffer: Option<Arc<Mutex<VecDeque<String>>>>) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES))),
        }
    }

    /// Non-blocking append; if the mutex is contended, drop the entry.
    pub fn try_append(&self, entry: LogEntry) {
        // Avoid blocking logging path; if we cannot get the lock immediately, drop.
        if let Ok(mut guard) = self.buffer.try_lock() {
            if guard.len() >= MAX_LOG_LINES {
                guard.pop_front();
            }
            guard.push_back(entry);
        }
    }

    pub fn get_recent_logs(&self, count: usize) -> Vec<LogEntry> {
        let buffer = match self.buffer.lock() {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };
        buffer.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }
}

static LOG_STREAMER: OnceLock<Arc<LogStreamer>> = OnceLock::new();

pub fn init(telnet_buffer: Option<Arc<Mutex<VecDeque<String>>>>) -> Arc<LogStreamer> {
    LOG_STREAMER.get_or_init(|| Arc::new(LogStreamer::new(telnet_buffer))).clone()
}

/// Append a log entry to the global buffer (non-blocking). Safe to call from logger.
pub fn append(level: &str, module: Option<&str>, message: &str, timestamp_ms: u64) {
    if let Some(streamer) = LOG_STREAMER.get() {
        let entry = LogEntry {
            timestamp: timestamp_ms,
            level: level.to_string(),
            message: message.to_string(),
            module: module.map(|m| m.to_string()),
        };
        streamer.try_append(entry);
    }
}