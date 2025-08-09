use core::sync::atomic::{AtomicU32, Ordering};
use esp_idf_hal::delay::FreeRtos;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

// Lightweight, fixed-memory observability with near-zero hot-path overhead

#[derive(Copy, Clone, Debug)]
pub enum Endpoint {
    Ping,
    Health,
    Other,
}

#[derive(Default, Serialize, Clone)]
pub struct HttpEndpointStats {
    pub total: u32,
    pub ok_2xx: u32,
    pub err_4xx_5xx: u32,
    pub total_duration_ms: u32,
    pub p95_hint_ms: u32, // simple max-of-recent hint (not exact)
}

#[derive(Default, Serialize, Clone)]
pub struct HttpStatsSnapshot {
    pub active_requests: u32,
    pub active_high_watermark: u32,
    pub ping: HttpEndpointStats,
    pub health: HttpEndpointStats,
}

static ACTIVE_REQUESTS: AtomicU32 = AtomicU32::new(0);
static ACTIVE_HIGH_WATERMARK: AtomicU32 = AtomicU32::new(0);

static PING_TOTAL: AtomicU32 = AtomicU32::new(0);
static PING_OK: AtomicU32 = AtomicU32::new(0);
static PING_ERR: AtomicU32 = AtomicU32::new(0);
static PING_DUR_MS: AtomicU32 = AtomicU32::new(0);
static PING_P95HINT_MS: AtomicU32 = AtomicU32::new(0);

static HEALTH_TOTAL: AtomicU32 = AtomicU32::new(0);
static HEALTH_OK: AtomicU32 = AtomicU32::new(0);
static HEALTH_ERR: AtomicU32 = AtomicU32::new(0);
static HEALTH_DUR_MS: AtomicU32 = AtomicU32::new(0);
static HEALTH_P95HINT_MS: AtomicU32 = AtomicU32::new(0);

#[inline]
pub fn begin_request() -> u64 {
    let now_us = unsafe { esp_idf_sys::esp_timer_get_time() as u64 };
    let cur = ACTIVE_REQUESTS.fetch_add(1, Ordering::Relaxed) + 1;
    // track high-watermark
    let mut hwm = ACTIVE_HIGH_WATERMARK.load(Ordering::Relaxed);
    while cur > hwm {
        if ACTIVE_HIGH_WATERMARK
            .compare_exchange(hwm, cur, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break;
        }
        hwm = ACTIVE_HIGH_WATERMARK.load(Ordering::Relaxed);
    }
    now_us
}

#[inline]
pub fn end_request(start_us: u64, ep: Endpoint, status: u16) {
    let end_us = unsafe { esp_idf_sys::esp_timer_get_time() as u64 };
    let dur_ms = ((end_us - start_us) / 1000) as u32;
    ACTIVE_REQUESTS.fetch_sub(1, Ordering::Relaxed);

    let is_ok = (200..300).contains(&status);
    match ep {
        Endpoint::Ping => {
            PING_TOTAL.fetch_add(1, Ordering::Relaxed);
            if is_ok { PING_OK.fetch_add(1, Ordering::Relaxed); } else { PING_ERR.fetch_add(1, Ordering::Relaxed); }
            PING_DUR_MS.fetch_add(dur_ms, Ordering::Relaxed);
            // simple max-of-recent hint
            let mut cur = PING_P95HINT_MS.load(Ordering::Relaxed);
            while dur_ms > cur {
                match PING_P95HINT_MS.compare_exchange(cur, dur_ms, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(v) => cur = v,
                }
            }
        }
        Endpoint::Health => {
            HEALTH_TOTAL.fetch_add(1, Ordering::Relaxed);
            if is_ok { HEALTH_OK.fetch_add(1, Ordering::Relaxed); } else { HEALTH_ERR.fetch_add(1, Ordering::Relaxed); }
            HEALTH_DUR_MS.fetch_add(dur_ms, Ordering::Relaxed);
            let mut cur = HEALTH_P95HINT_MS.load(Ordering::Relaxed);
            while dur_ms > cur {
                match HEALTH_P95HINT_MS.compare_exchange(cur, dur_ms, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(v) => cur = v,
                }
            }
        }
        Endpoint::Other => {}
    }
}

pub fn http_snapshot() -> HttpStatsSnapshot {
    HttpStatsSnapshot {
        active_requests: ACTIVE_REQUESTS.load(Ordering::Relaxed),
        active_high_watermark: ACTIVE_HIGH_WATERMARK.load(Ordering::Relaxed),
        ping: HttpEndpointStats {
            total: PING_TOTAL.load(Ordering::Relaxed),
            ok_2xx: PING_OK.load(Ordering::Relaxed),
            err_4xx_5xx: PING_ERR.load(Ordering::Relaxed),
            total_duration_ms: PING_DUR_MS.load(Ordering::Relaxed),
            p95_hint_ms: PING_P95HINT_MS.load(Ordering::Relaxed),
        },
        health: HttpEndpointStats {
            total: HEALTH_TOTAL.load(Ordering::Relaxed),
            ok_2xx: HEALTH_OK.load(Ordering::Relaxed),
            err_4xx_5xx: HEALTH_ERR.load(Ordering::Relaxed),
            total_duration_ms: HEALTH_DUR_MS.load(Ordering::Relaxed),
            p95_hint_ms: HEALTH_P95HINT_MS.load(Ordering::Relaxed),
        },
    }
}

// ---- Event rings (try_lock; drop on contention) ----

#[derive(Serialize, Clone, Default)]
pub struct WifiEventEntry {
    pub ts_ms: u64,
    pub kind: &'static str,
    pub reason: u32,
    pub rssi_dbm: i32,
    pub channel: u32,
}

#[derive(Serialize, Clone, Default)]
pub struct HttpErrorEntry {
    pub ts_ms: u64,
    pub path: &'static str,
    pub status: u16,
    pub dur_ms: u32,
}

const WIFI_RING_CAP: usize = 32;
const HTTP_ERR_RING_CAP: usize = 16;

static WIFI_RING: OnceLock<Mutex<VecDeque<WifiEventEntry>>> = OnceLock::new();
static HTTP_RING: OnceLock<Mutex<VecDeque<HttpErrorEntry>>> = OnceLock::new();

fn wifi_ring() -> &'static Mutex<VecDeque<WifiEventEntry>> {
    WIFI_RING.get_or_init(|| Mutex::new(VecDeque::with_capacity(WIFI_RING_CAP)))
}

fn http_ring() -> &'static Mutex<VecDeque<HttpErrorEntry>> {
    HTTP_RING.get_or_init(|| Mutex::new(VecDeque::with_capacity(HTTP_ERR_RING_CAP)))
}

#[inline]
pub fn record_wifi_event(kind: &'static str, reason: u32, rssi_dbm: i32, channel: u32) {
    let ts_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
    if let Ok(mut q) = wifi_ring().try_lock() {
        if q.len() == WIFI_RING_CAP { q.pop_front(); }
        q.push_back(WifiEventEntry { ts_ms, kind, reason, rssi_dbm, channel });
    }
}

#[inline]
pub fn record_http_error(path: &'static str, status: u16, dur_ms: u32) {
    let ts_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
    if let Ok(mut q) = http_ring().try_lock() {
        if q.len() == HTTP_ERR_RING_CAP { q.pop_front(); }
        q.push_back(HttpErrorEntry { ts_ms, path, status, dur_ms });
    }
}

#[derive(Serialize, Default)]
pub struct EventsSnapshot {
    pub boot_id: u32,
    pub uptime_ms: u64,
    pub wifi_events: Vec<WifiEventEntry>,
    pub http_errors: Vec<HttpErrorEntry>,
}

static BOOT_ID: OnceLock<u32> = OnceLock::new();

pub fn boot_id() -> u32 {
    *BOOT_ID.get_or_init(|| {
        // Derive a simple boot id from timer and MAC LSB if available (best-effort)
        let t = unsafe { esp_idf_sys::esp_timer_get_time() as u64 } as u32;
        t ^ 0xA5A5_5A5A
    })
}

pub fn events_snapshot() -> EventsSnapshot {
    let uptime_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
    let wifi = if let Ok(q) = wifi_ring().try_lock() {
        q.iter().cloned().collect()
    } else { vec![] };
    let http = if let Ok(q) = http_ring().try_lock() {
        q.iter().cloned().collect()
    } else { vec![] };
    EventsSnapshot { boot_id: boot_id(), uptime_ms, wifi_events: wifi, http_errors: http }
}

// Optional helper to lightly yield if system appears slammed (not used by default)
#[allow(dead_code)]
pub fn maybe_yield_on_pressure() {
    let act = ACTIVE_REQUESTS.load(Ordering::Relaxed);
    if act > 8 { FreeRtos::delay_ms(1); }
}


