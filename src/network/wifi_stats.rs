use core::sync::atomic::{AtomicI32, AtomicU32, AtomicBool, Ordering};

static WIFI_CONNECTED: AtomicBool = AtomicBool::new(false);
static WIFI_DISCONNECTS: AtomicU32 = AtomicU32::new(0);
static WIFI_RECONNECTS: AtomicU32 = AtomicU32::new(0);
static WIFI_RSSI_DBM: AtomicI32 = AtomicI32::new(0);
static WIFI_CHANNEL: AtomicU32 = AtomicU32::new(0);

pub fn set_connected(connected: bool) {
    WIFI_CONNECTED.store(connected, Ordering::Relaxed);
}

pub fn record_disconnect() {
    WIFI_DISCONNECTS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_reconnect() {
    WIFI_RECONNECTS.fetch_add(1, Ordering::Relaxed);
}

pub fn set_rssi_dbm(rssi: i32) {
    WIFI_RSSI_DBM.store(rssi, Ordering::Relaxed);
}

pub fn set_channel(ch: u32) {
    WIFI_CHANNEL.store(ch, Ordering::Relaxed);
}

#[derive(serde::Serialize)]
pub struct WifiStatsSnapshot {
    pub connected: bool,
    pub disconnects: u32,
    pub reconnects: u32,
    pub rssi_dbm: i32,
    pub channel: u32,
}

pub fn snapshot() -> WifiStatsSnapshot {
    WifiStatsSnapshot {
        connected: WIFI_CONNECTED.load(Ordering::Relaxed),
        disconnects: WIFI_DISCONNECTS.load(Ordering::Relaxed),
        reconnects: WIFI_RECONNECTS.load(Ordering::Relaxed),
        rssi_dbm: WIFI_RSSI_DBM.load(Ordering::Relaxed),
        channel: WIFI_CHANNEL.load(Ordering::Relaxed),
    }
}

