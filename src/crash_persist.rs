use serde::{Deserialize, Serialize};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};

const CRASH_NS: &str = "crash";
const CRASH_KEY: &str = "last";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashLogEntry {
    pub ts: u64,
    pub level: String,
    pub module: Option<String>,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastCrash {
    pub panic_reason: String,
    pub timestamp_unix: u64,
    pub uptime_seconds: u64,
    pub heap_free: u32,
    pub heap_min: u32,
    pub psram_free: u32,
    pub log_excerpt: Vec<CrashLogEntry>,
}

/// Best-effort save of last crash information. Never panics.
pub fn save_last_crash(panic_reason: &str) {
    let timestamp_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let uptime_seconds = unsafe { esp_idf_sys::esp_timer_get_time() as u64 / 1_000_000 };
    let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() as u32 };
    let heap_min = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() as u32 };
    let psram_free = unsafe { esp_idf_sys::heap_caps_get_free_size(esp_idf_sys::MALLOC_CAP_SPIRAM) as u32 };

    // Collect a small excerpt of recent logs (non-blocking in logger path)
    let log_excerpt = {
        let streamer = crate::network::log_streamer::init(None);
        let logs = streamer.get_recent_logs(50);
        logs.into_iter()
            .map(|e| CrashLogEntry {
                ts: e.timestamp,
                level: e.level,
                module: e.module,
                msg: e.message,
            })
            .collect::<Vec<_>>()
    };

    let record = LastCrash {
        panic_reason: panic_reason.to_string(),
        timestamp_unix,
        uptime_seconds,
        heap_free,
        heap_min,
        psram_free,
        log_excerpt,
    };

    if let Ok(bytes) = serde_json::to_vec(&record) {
        if let Ok(nvs_part) = EspDefaultNvsPartition::take() {
            if let Ok(mut nvs) = EspNvs::new(nvs_part, CRASH_NS, true) {
                let _ = nvs.set_blob(CRASH_KEY, &bytes);
            }
        }
    }
}

pub fn read_last_crash() -> anyhow::Result<Option<LastCrash>> {
    let nvs_part = match EspDefaultNvsPartition::take() {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    let nvs = match EspNvs::new(nvs_part, CRASH_NS, true) {
        Ok(n) => n,
        Err(_) => return Ok(None),
    };

    let mut buf = vec![0u8; 4096];
    let data_opt = nvs.get_blob(CRASH_KEY, &mut buf)?;
    let data = match data_opt {
        Some(d) if !d.is_empty() => d,
        _ => return Ok(None),
    };
    match serde_json::from_slice::<LastCrash>(data) {
        Ok(val) => Ok(Some(val)),
        Err(_) => Ok(None),
    }
}

pub fn clear_last_crash() -> anyhow::Result<()> {
    let nvs_part = EspDefaultNvsPartition::take()?;
    let mut nvs = EspNvs::new(nvs_part, CRASH_NS, true)?;
    // Write minimal empty JSON to logically clear
    nvs.set_blob(CRASH_KEY, b"{}")?;
    Ok(())
}


