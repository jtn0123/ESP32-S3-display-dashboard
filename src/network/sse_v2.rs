use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use log::{info, warn, error};

// SSE configuration constants
const MAX_SSE_CONNECTIONS: u32 = 2;  // Per test requirements
const SSE_TIMEOUT_SECS: u64 = 300;   // 5 minutes
const HEARTBEAT_INTERVAL_SECS: u64 = 30;
const METRICS_UPDATE_INTERVAL_SECS: u64 = 1;

#[derive(Debug)]
struct ConnectionInfo {
    id: u32,
    _start_time: Instant,
    _endpoint: String,
}

pub struct SseManager {
    connections: Arc<Mutex<Vec<ConnectionInfo>>>,
    next_id: Arc<Mutex<u32>>,
}

impl SseManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    fn add_connection(&self, endpoint: &str) -> Result<u32> {
        let mut connections = self.connections.lock().map_err(|e| anyhow::anyhow!("connections lock poisoned: {}", e))?;
        
        // Check connection limit
        if connections.len() >= MAX_SSE_CONNECTIONS as usize {
            crate::diagnostics::log_sse_event("limit_reject", None);
            return Err(anyhow::anyhow!("Connection limit reached"));
        }
        
        // Check heap before accepting connection
        let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
        if free_heap < 80 * 1024 { // 80KB minimum
            crate::diagnostics::log_sse_event("heap_reject", None);
            return Err(anyhow::anyhow!("Insufficient heap memory"));
        }
        
        let mut next_id = self.next_id.lock().map_err(|e| anyhow::anyhow!("next_id lock poisoned: {}", e))?;
        let id = *next_id;
        *next_id += 1;
        
        connections.push(ConnectionInfo {
            id,
            _start_time: Instant::now(),
            _endpoint: endpoint.to_string(),
        });
        
        info!("SSE: Connection {} added to {} (total: {})", 
              id, endpoint, connections.len());
        crate::diagnostics::log_sse_event("connect", Some(id));
        Ok(id)
    }

    fn remove_connection(&self, id: u32) {
        let remaining = match self.connections.lock() {
            Ok(mut connections) => {
                connections.retain(|c| c.id != id);
                connections.len()
            }
            Err(e) => {
                error!("SSE: connections lock poisoned during remove: {}", e);
                0
            }
        };
        info!("SSE: Connection {} removed (remaining: {})", id, remaining);
        crate::diagnostics::log_sse_event("disconnect", Some(id));
    }

    pub fn register_endpoints(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        // Check feature gates before registering
        let features = crate::feature_gates::FeatureStatus::check();
        if !features.sse_enabled {
            warn!("SSE: Disabled by feature gates (insufficient heap)");
            return Ok(());
        }

        // Register /sse/logs endpoint
        self.register_logs_endpoint(server)?;
        
        // Register /sse/stats endpoint
        self.register_stats_endpoint(server)?;
        
        // Keep /api/events for backward compatibility
        self.register_events_endpoint(server)?;
        
        info!("SSE: All endpoints registered (max {} clients)", features.max_sse_clients);
        Ok(())
    }

    fn register_logs_endpoint(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        let manager = self.clone();
        
        server.fn_handler("/sse/logs", Method::Get, move |req| {
            handle_sse_connection(req, &manager, "logs", |response, _heartbeat_count| {
                // Get recent log entries from the in-memory log streamer
                let streamer = crate::network::log_streamer::init(None);
                let logs = streamer.get_recent_logs(50);
                for entry in logs {
                    let event = format!(
                        "event: log\ndata: {}\n\n",
                        serde_json::json!({
                            "level": entry.level,
                            "module": entry.module.unwrap_or_else(|| "unknown".to_string()),
                            "msg": entry.message,
                            "timestamp_ms": entry.timestamp
                        })
                    );
                    if response.write_all(event.as_bytes()).is_err() {
                        return Err(anyhow::anyhow!("Failed to write log event"));
                    }
                }
                Ok(())
            })
        })?;
        
        Ok(())
    }

    fn register_stats_endpoint(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        let manager = self.clone();
        
        server.fn_handler("/sse/stats", Method::Get, move |req| {
            handle_sse_connection(req, &manager, "stats", |response, _heartbeat_count| {
                // Send system stats
                let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
                let uptime = unsafe { esp_idf_sys::esp_timer_get_time() } / 1_000_000; // Convert to seconds
                
                let event = format!(
                    "event: stats\ndata: {}\n\n",
                    serde_json::json!({
                        "heap": heap_free,
                        "uptime": uptime,
                        "count": 1,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    })
                );
                
                if response.write_all(event.as_bytes()).is_err() {
                    return Err(anyhow::anyhow!("Failed to write stats event"));
                }
                
                Ok(())
            })
        })?;
        
        Ok(())
    }

    fn register_events_endpoint(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        let manager = self.clone();
        
        server.fn_handler("/api/events", Method::Get, move |req| {
            handle_sse_connection(req, &manager, "events", |response, _heartbeat_count| {
                // Send comprehensive metrics data for dashboard
                if let Ok(metrics) = crate::metrics::metrics().try_lock() {
                    // Get system info
                    let uptime_ms = unsafe { esp_idf_sys::esp_timer_get_time() / 1000 };
                    let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
                    let psram_free = unsafe { esp_idf_sys::heap_caps_get_free_size(esp_idf_sys::MALLOC_CAP_SPIRAM) };
                    
                    // Calculate heap fragmentation
                    let largest_free = unsafe { esp_idf_sys::heap_caps_get_largest_free_block(esp_idf_sys::MALLOC_CAP_INTERNAL) };
                    let fragmentation = if heap_free > 0 && largest_free > 0 {
                        ((1.0 - (largest_free as f32 / heap_free as f32)) * 100.0) as u32
                    } else {
                        0
                    };
                    
                    let event = format!(
                        "data: {}\n\n",
                        serde_json::json!({
                            "type": "metrics",
                            "uptime_ms": uptime_ms,
                            "temperature": (metrics.temperature * 10.0).round() / 10.0,
                            "fps_actual": (metrics.fps_actual * 10.0).round() / 10.0,
                            "cpu_usage": metrics.cpu_usage,
                            "cpu0_usage": metrics.cpu0_usage,
                            "cpu1_usage": metrics.cpu1_usage,
                            "cpu_freq_mhz": metrics.cpu_freq_mhz,
                            "wifi_rssi": metrics.wifi_rssi,
                            "wifi_connected": metrics.wifi_connected,
                            "wifi_ssid": metrics.wifi_ssid.clone(),
                            "battery_percentage": metrics.battery_percentage,
                            "heap_free_kb": heap_free / 1024,
                            "psram_free_kb": psram_free / 1024,
                            "heap_fragmentation": fragmentation,
                            "skip_rate": if metrics.frame_count > 0 {
                                metrics.skip_count as f32 / metrics.frame_count as f32 * 100.0
                            } else { 0.0 },
                            "render_time_ms": metrics.render_time_ms,
                            // ip_address intentionally omitted here to avoid stale values
                        })
                    );
                    
                    if response.write_all(event.as_bytes()).is_err() {
                        return Err(anyhow::anyhow!("Failed to write metrics event"));
                    }
                }
                Ok(())
            })
        })?;
        
        Ok(())
    }
}

impl Clone for SseManager {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            next_id: self.next_id.clone(),
        }
    }
}

// Generic SSE connection handler
fn handle_sse_connection<F>(
    req: esp_idf_svc::http::server::Request<&mut esp_idf_svc::http::server::EspHttpConnection>,
    manager: &SseManager,
    endpoint_name: &str,
    mut data_sender: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&mut esp_idf_svc::http::server::Response<&mut esp_idf_svc::http::server::EspHttpConnection>, u32) -> Result<()>,
{
    // Try to add connection
    let conn_id = match manager.add_connection(&format!("/sse/{}", endpoint_name)) {
        Ok(id) => id,
        Err(e) => {
            warn!("SSE: Connection rejected: {}", e);
            let mut response = req.into_status_response(503)?;
            response.write_all(b"Service Unavailable: Connection limit reached")?;
            return Ok(());
        }
    };
    
    // Set up cleanup on exit
    let _cleanup = ConnectionCleanup::new(manager.clone(), conn_id);
    
    // Set SSE headers
    let headers = [
        ("Content-Type", "text/event-stream"),
        ("Cache-Control", "no-cache"),
        ("Connection", "keep-alive"),
        ("Access-Control-Allow-Origin", "*"),
        ("X-Accel-Buffering", "no"), // Disable proxy buffering
    ];
    
    let mut response = req.into_response(200, Some("OK"), &headers)?;
    
    // Send initial connection event
    let init_event = format!(
        "event: connected\ndata: {{\"connection_id\":{}}}\n\n",
        conn_id
    );
    safe_write(&mut response, init_event.as_bytes())?;
    response.flush()?;
    
    // Main event loop
    let start_time = Instant::now();
    let mut last_update = Instant::now();
    let mut heartbeat_count = 0u32;
    
    loop {
        // Check timeout
        if start_time.elapsed() > Duration::from_secs(SSE_TIMEOUT_SECS) {
            info!("SSE: Connection {} timeout after 5 minutes", conn_id);
            break;
        }
        
        // Send data updates every second
        if last_update.elapsed() >= Duration::from_secs(METRICS_UPDATE_INTERVAL_SECS) {
            match data_sender(&mut response, heartbeat_count) {
                Ok(_) => {
                    if response.flush().is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("SSE: Data send error: {}", e);
                    break;
                }
            }
            last_update = Instant::now();
        }
        
        // Send heartbeat every 30 seconds
        heartbeat_count += 1;
        if heartbeat_count >= HEARTBEAT_INTERVAL_SECS as u32 {
            heartbeat_count = 0;
            if safe_write(&mut response, b":heartbeat\n\n").is_err() {
                break;
            }
            if response.flush().is_err() {
                break;
            }
        }
        
        // Sleep to prevent CPU hogging
        std::thread::sleep(Duration::from_secs(1));
    }
    
    Ok(())
}

// Safe write wrapper with error handling
fn safe_write(
    response: &mut esp_idf_svc::http::server::Response<&mut esp_idf_svc::http::server::EspHttpConnection>,
    data: &[u8],
) -> Result<()> {
    match response.write_all(data) {
        Ok(_) => {
            crate::diagnostics::log_sse_data(data.len(), 1);
            Ok(())
        },
        Err(e) => {
            error!("SSE: Write failed: {}", e);
            Err(anyhow::anyhow!("Write failed: {}", e))
        }
    }
}

// RAII cleanup guard
struct ConnectionCleanup {
    manager: SseManager,
    conn_id: u32,
}

impl ConnectionCleanup {
    fn new(manager: SseManager, conn_id: u32) -> Self {
        Self { manager, conn_id }
    }
}

impl Drop for ConnectionCleanup {
    fn drop(&mut self) {
        self.manager.remove_connection(self.conn_id);
    }
}

// Global SSE manager instance
use std::sync::OnceLock;
static SSE_MANAGER: OnceLock<Arc<SseManager>> = OnceLock::new();

pub fn init() -> Arc<SseManager> {
    SSE_MANAGER.get_or_init(|| Arc::new(SseManager::new())).clone()
}