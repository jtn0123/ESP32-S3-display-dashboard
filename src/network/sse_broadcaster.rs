use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Simple SSE broadcaster that doesn't try to store responses
// Instead, it relies on the metrics system to provide data when requested
pub struct SseBroadcaster {
    // We don't actually store connections in ESP32 version
    // Just keep track of active connections count
    active_connections: Arc<Mutex<u32>>,
}

impl SseBroadcaster {
    pub fn new() -> Self {
        Self {
            active_connections: Arc::new(Mutex::new(0)),
        }
    }

    pub fn register_endpoints(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        let connections = self.active_connections.clone();

        // SSE endpoint for real-time updates
        server.fn_handler("/api/events", Method::Get, move |req| {
            // Check connection limit
            const MAX_SSE_CONNECTIONS: u32 = 5;
            {
                match connections.lock() {
                    Ok(mut count) => {
                        if *count >= MAX_SSE_CONNECTIONS {
                            log::warn!("SSE connection limit reached ({} connections)", *count);
                            let mut response = req.into_status_response(503)?;
                            response.write_all(b"Too many connections")?;
                            return Ok(());
                        }
                        *count += 1;
                        log::info!("SSE client connected, total: {}", *count);
                    }
                    Err(e) => {
                        log::error!("SSE: connections lock poisoned: {}", e);
                        let mut response = req.into_status_response(500)?;
                        response.write_all(b"Internal server error")?;
                        return Ok(());
                    }
                }
            }

            // Set SSE headers
            let headers = [
                ("Content-Type", "text/event-stream"),
                ("Cache-Control", "no-cache"),
                ("Connection", "keep-alive"),
                ("Access-Control-Allow-Origin", "*"),
            ];

            let mut response = req.into_response(200, Some("OK"), &headers)?;
            
            // Send initial connection message
            response.write_all(b"data: {\"type\":\"connected\"}\n\n")?;
            response.flush()?;

            // Send periodic updates with timeout
            let start_time = Instant::now();
            let max_duration = Duration::from_secs(300); // 5 minute timeout
            let mut heartbeat_counter = 0u8;
            
            loop {
                // Check for timeout
                if start_time.elapsed() > max_duration {
                    log::info!("SSE connection timeout after 5 minutes");
                    break;
                }
                
                std::thread::sleep(Duration::from_secs(1));
                
                // Send metrics update
                if let Ok(metrics) = crate::metrics::metrics().try_lock() {
                    let event = serde_json::json!({
                        "type": "metrics",
                        "data": {
                            "temperature": (metrics.temperature * 10.0).round() / 10.0,
                            "fps_actual": (metrics.fps_actual * 10.0).round() / 10.0,
                            "cpu_usage": metrics.cpu_usage,
                            "wifi_rssi": metrics.wifi_rssi,
                            "battery_percentage": metrics.battery_percentage,
                        }
                    });
                    
                    let data = format!("data: {}\n\n", serde_json::to_string(&event)?);
                    if response.write_all(data.as_bytes()).is_err() {
                        break;
                    }
                    if response.flush().is_err() {
                        break;
                    }
                }
                
                // Send heartbeat every 30 iterations (30 seconds)
                heartbeat_counter += 1;
                if heartbeat_counter >= 30 {
                    heartbeat_counter = 0;
                    if response.write_all(b":heartbeat\n\n").is_err() {
                        break;
                    }
                    if response.flush().is_err() {
                        break;
                    }
                }
            }

            // Decrement connection count on disconnect
            if let Ok(mut count) = connections.lock() {
                *count = count.saturating_sub(1);
                log::info!("SSE client disconnected, remaining: {}", *count);
            } else {
                log::warn!("SSE: failed to lock connections for decrement on disconnect");
            }

            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        Ok(())
    }

}

// Global broadcaster instance using OnceLock for safety
use std::sync::OnceLock;
static SSE_BROADCASTER: OnceLock<Arc<SseBroadcaster>> = OnceLock::new();

pub fn init() -> Arc<SseBroadcaster> {
    SSE_BROADCASTER.get_or_init(|| Arc::new(SseBroadcaster::new())).clone()
}

