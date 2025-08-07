use log::{Level, LevelFilter, Metadata, Record};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use crate::network::telnet_server::TelnetLogServer;
use crate::network::log_streamer;

static TELNET_SERVER: OnceLock<Arc<TelnetLogServer>> = OnceLock::new();
static BOOT_TIME: OnceLock<SystemTime> = OnceLock::new();

#[allow(dead_code)]
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const GRAY: &str = "\x1b[90m";
}

/// Enhanced logger that prints colored, timestamped lines and forwards to telnet/log streamer
struct EnhancedLogger;

impl log::Log for EnhancedLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Time since boot
        let boot_time = BOOT_TIME.get_or_init(|| SystemTime::now());
        let elapsed = SystemTime::now()
            .duration_since(*boot_time)
            .unwrap_or_default();
        let seconds = elapsed.as_secs();
        let millis = elapsed.subsec_millis();
        let ts_compact = if seconds < 60 {
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
        let ts_ms = seconds.saturating_mul(1000) + millis as u64;

        // Level, color and module
        let (color, level_str, level_char) = match record.level() {
            Level::Error => (colors::BRIGHT_RED, "ERROR", 'E'),
            Level::Warn => (colors::BRIGHT_YELLOW, "WARN ", 'W'),
            Level::Info => (colors::BRIGHT_GREEN, "INFO ", 'I'),
            Level::Debug => (colors::BRIGHT_BLUE, "DEBUG", 'D'),
            Level::Trace => (colors::GRAY, "TRACE", 'T'),
        };
        let module = record
            .module_path()
            .unwrap_or("unknown")
            .split("::")
            .last()
            .unwrap_or("unknown");
        let module_display = if module.len() > 12 { &module[..12] } else { module };

        // Message
        let message = format!("{}", record.args());

        // Console output (serial). ANSI colors are fine over serial; telnet gets plain text below.
        println!(
            "{}{} [{}] {:>12} | {}{}",
            color, ts_compact, level_char, module_display, message, colors::RESET
        );

        // Telnet (no colors)
        if let Some(server) = TELNET_SERVER.get() {
            let telnet_msg = format!(
                "{} [{}] {:>12} | {}",
                ts_compact, level_str, module_display, message
            );
            server.log_message(level_str, &telnet_msg);
        }

        // Append to in-memory log streamer (non-blocking; drop on contention)
        log_streamer::append(level_str, Some(module), &message, ts_ms);
    }

    fn flush(&self) {}
}

static LOGGER: EnhancedLogger = EnhancedLogger;

/// Initialize the enhanced logger with colors and timestamps
pub fn init_logger() -> Result<(), log::SetLoggerError> {
    let _ = BOOT_TIME.set(SystemTime::now());
    log::set_logger(&LOGGER)?;
    log::set_max_level(LevelFilter::Debug);

    // Friendly startup banner
    println!("{}┌─────────────────────────────────────────┐{}", colors::BRIGHT_GREEN, colors::RESET);
    println!("{}│   ESP32-S3 Dashboard Enhanced Logger    │{}", colors::BRIGHT_GREEN, colors::RESET);
    println!(
        "{}│   Levels: {}E{}rror {}W{}arn {}I{}nfo {}D{}ebug {}T{}race   │{}",
        colors::BRIGHT_GREEN,
        colors::BRIGHT_RED,
        colors::BRIGHT_GREEN,
        colors::BRIGHT_YELLOW,
        colors::BRIGHT_GREEN,
        colors::BRIGHT_GREEN,
        colors::BRIGHT_GREEN,
        colors::BRIGHT_BLUE,
        colors::BRIGHT_GREEN,
        colors::GRAY,
        colors::BRIGHT_GREEN,
        colors::RESET
    );
    println!("{}└─────────────────────────────────────────┘{}", colors::BRIGHT_GREEN, colors::RESET);
    Ok(())
}

/// Set the telnet server for log forwarding
pub fn set_telnet_server(server: Arc<TelnetLogServer>) {
    let _ = TELNET_SERVER.set(server);
}

/// Get the telnet server if it's been set (used in a few places)
pub fn get_telnet_server() -> Option<Arc<TelnetLogServer>> {
    TELNET_SERVER.get().cloned()
}

/// Change log level at runtime (e.g., via telnet command handler)
pub fn set_max_level_runtime(level: LevelFilter) {
    log::set_max_level(level);
}

/// Parse and set log level from a string; returns true if applied
pub fn set_max_level_from_str(level: &str) -> bool {
    let lf = match level.to_ascii_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" | "warning" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => return false,
    };
    set_max_level_runtime(lf);
    true
}

/// Current global max level
pub fn current_max_level() -> LevelFilter {
    log::max_level()
}

/// Convenience macros with subtle tags
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { log::error!($($arg)*); };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { log::warn!($($arg)*); };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { log::info!($($arg)*); };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => { log::debug!($($arg)*); };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => { log::trace!($($arg)*); };
}