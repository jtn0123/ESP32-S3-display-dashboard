use log::{Level, Metadata, Record};
use std::sync::{Arc, Mutex, OnceLock};
use crate::network::telnet_server::TelnetLogServer;

static TELNET_SERVER: OnceLock<Arc<TelnetLogServer>> = OnceLock::new();

/// Custom logger that forwards to both ESP console and telnet server
struct DualLogger;

impl log::Log for DualLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Filter out debug messages from display module
        if metadata.target().starts_with("esp32_s3_dashboard::display") 
            && metadata.level() == Level::Debug {
            return false;
        }
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Format the message
            let level_str = match record.level() {
                Level::Error => "ERROR",
                Level::Warn => "WARN ",
                Level::Info => "INFO ",
                Level::Debug => "DEBUG",
                Level::Trace => "TRACE",
            };
            
            let message = format!("{}", record.args());
            
            // Filter out display debug spam
            if message.contains("[ST7789] COLOR") || 
               message.contains("0xFFFFFFFF") ||
               message.contains("Chunking large transfer") ||
               message.contains("Successfully completed chunked transfer") {
                return;
            }
            
            // Print to console (ESP-IDF serial)
            println!("[{}] {}", level_str, message);
            
            // Send to telnet server if available
            if let Some(server) = TELNET_SERVER.get() {
                server.log_message(level_str, &message);
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: DualLogger = DualLogger;

/// Initialize the dual logger (console + telnet)
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER)?;
    log::set_max_level(log::LevelFilter::Debug);
    Ok(())
}

/// Set the telnet server for log forwarding
pub fn set_telnet_server(server: Arc<TelnetLogServer>) {
    let _ = TELNET_SERVER.set(server);
}