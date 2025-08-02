use log::{Level, Metadata, Record};
use std::sync::{Arc, OnceLock};
use crate::network::telnet_server::TelnetLogServer;
use std::time::SystemTime;

static TELNET_SERVER: OnceLock<Arc<TelnetLogServer>> = OnceLock::new();
static BOOT_TIME: OnceLock<SystemTime> = OnceLock::new();

/// ANSI color codes for terminal output
#[allow(dead_code)]
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const GREEN: &str = "\x1b[32m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const GRAY: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
}

/// Enhanced logger with colors, timestamps, and module names
struct EnhancedLogger;

impl log::Log for EnhancedLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Get timestamp
            let boot_time = BOOT_TIME.get_or_init(|| SystemTime::now());
            let elapsed = SystemTime::now()
                .duration_since(*boot_time)
                .unwrap_or_default();
            let seconds = elapsed.as_secs();
            let millis = elapsed.subsec_millis();
            
            // Format timestamp
            let timestamp = if seconds < 60 {
                format!("{:>3}.{:03}s", seconds, millis)
            } else if seconds < 3600 {
                let minutes = seconds / 60;
                let secs = seconds % 60;
                format!("{:>2}m{:02}s", minutes, secs)
            } else {
                let hours = seconds / 3600;
                let mins = (seconds % 3600) / 60;
                format!("{:>2}h{:02}m", hours, mins)
            };
            
            // Get color and level string based on log level
            let (color, level_str, level_char) = match record.level() {
                Level::Error => (colors::BRIGHT_RED, "ERROR", "E"),
                Level::Warn => (colors::BRIGHT_YELLOW, "WARN ", "W"),
                Level::Info => (colors::BRIGHT_GREEN, "INFO ", "I"),
                Level::Debug => (colors::BRIGHT_BLUE, "DEBUG", "D"),
                Level::Trace => (colors::GRAY, "TRACE", "T"),
            };
            
            // Extract module name
            let module = record.module_path()
                .unwrap_or("unknown")
                .split("::")
                .last()
                .unwrap_or("unknown");
            
            // Truncate module name to 12 chars
            let module_display = if module.len() > 12 {
                &module[..12]
            } else {
                module
            };
            
            // Format the message
            let message = format!("{}", record.args());
            
            // Print colored output to console
            println!(
                "{}{} [{}] {:>12} | {}{}",
                color,
                timestamp,
                level_char,
                module_display,
                message,
                colors::RESET
            );
            
            // Send to telnet server if available (without colors)
            if let Some(server) = TELNET_SERVER.get() {
                let telnet_msg = format!(
                    "{} [{}] {:>12} | {}",
                    timestamp,
                    level_str,
                    module_display,
                    message
                );
                server.log_message(level_str, &telnet_msg);
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: EnhancedLogger = EnhancedLogger;

/// Initialize the enhanced logger with colors and timestamps
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    // Initialize boot time
    let _ = BOOT_TIME.set(SystemTime::now());
    
    log::set_logger(&LOGGER)?;
    log::set_max_level(log::LevelFilter::Debug);
    
    // Print startup banner
    println!("{}┌─────────────────────────────────────────┐{}", colors::BRIGHT_GREEN, colors::RESET);
    println!("{}│   ESP32-S3 Dashboard Enhanced Logger    │{}", colors::BRIGHT_GREEN, colors::RESET);
    println!("{}│   Colors: {}E{}rror {}W{}arn {}I{}nfo {}D{}ebug {}T{}race   │{}", 
        colors::BRIGHT_GREEN,
        colors::BRIGHT_RED, colors::BRIGHT_GREEN,
        colors::BRIGHT_YELLOW, colors::BRIGHT_GREEN,
        colors::BRIGHT_GREEN, colors::BRIGHT_GREEN,
        colors::BRIGHT_BLUE, colors::BRIGHT_GREEN,
        colors::GRAY, colors::BRIGHT_GREEN,
        colors::RESET
    );
    println!("{}└─────────────────────────────────────────┘{}", colors::BRIGHT_GREEN, colors::RESET);
    
    Ok(())
}

/// Set the telnet server for log forwarding
pub fn set_telnet_server(server: Arc<TelnetLogServer>) {
    let _ = TELNET_SERVER.set(server);
}

// Removed unused get_telnet_server function

/// Log macros with color hints
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        log::error!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        log::warn!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        log::info!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*);
    };
}

/// Performance logging with special formatting
#[macro_export]
macro_rules! log_perf {
    ($($arg:tt)*) => {
        log::info!("\x1b[35m[PERF]\x1b[0m {}", format!($($arg)*));
    };
}

/// Network logging with special formatting
#[macro_export]
macro_rules! log_net {
    ($($arg:tt)*) => {
        log::info!("\x1b[36m[NET]\x1b[0m {}", format!($($arg)*));
    };
}

/// System logging with special formatting
#[macro_export]
macro_rules! log_sys {
    ($($arg:tt)*) => {
        log::info!("\x1b[33m[SYS]\x1b[0m {}", format!($($arg)*));
    };
}