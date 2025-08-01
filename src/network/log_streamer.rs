use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

const MAX_LOG_LINES: usize = 10000;

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

    pub fn get_recent_logs(&self, count: usize) -> Vec<LogEntry> {
        let buffer = self.buffer.lock().unwrap();
        buffer.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }
}

// Global log streamer instance using OnceLock for safety
use std::sync::OnceLock;
static LOG_STREAMER: OnceLock<Arc<LogStreamer>> = OnceLock::new();

pub fn init(telnet_buffer: Option<Arc<Mutex<VecDeque<String>>>>) -> Arc<LogStreamer> {
    LOG_STREAMER.get_or_init(|| Arc::new(LogStreamer::new(telnet_buffer))).clone()
}