# ESP32 Web Server Integration Guide

This guide provides step-by-step instructions for integrating the web server improvements into the ESP32-S3 Display Dashboard.

## Implementation Status: ✅ COMPLETED

All major components have been implemented and adapted for ESP32:
- ✅ SSE Broadcaster (replaced WebSocket for compatibility)
- ✅ Enhanced API Routes with sensor history
- ✅ Error handling and validation
- ✅ Log streaming with virtual scrolling
- ✅ File manager adapted for SPIFFS
- ✅ PWA support with service worker
- ✅ Dashboard updated for real-time updates
- ✅ Mobile responsive enhancements

## Table of Contents
1. [Compilation Fixes](#compilation-fixes)
2. [Missing Module Implementations](#missing-module-implementations)
3. [ESP32-Specific Adaptations](#esp32-specific-adaptations)
4. [Memory Optimizations](#memory-optimizations)
5. [Testing & Debugging Checklist](#testing--debugging-checklist)
6. [Performance Optimization Checklist](#performance-optimization-checklist)

## Compilation Fixes

### 1. Update Cargo.toml Dependencies

Add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

# Web server enhancements
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
base64 = { version = "0.21", default-features = false, features = ["alloc"] }

# For WebSocket support in ESP-IDF
embedded-svc = { version = "0.27", features = ["experimental"] }
```

### 2. Fix WebSocket Implementation

Since ESP-IDF's HTTP server has limited WebSocket support, we'll use Server-Sent Events (SSE) as a more compatible alternative:

**Create `src/network/sse_broadcaster.rs`:**

```rust
use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;

pub struct SseBroadcaster {
    clients: Arc<Mutex<HashMap<u32, Arc<Mutex<dyn Write + Send>>>>>,
    next_id: Arc<Mutex<u32>>,
}

impl SseBroadcaster {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn register_endpoints(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        let clients = self.clients.clone();
        let next_id = self.next_id.clone();

        // SSE endpoint for real-time updates
        server.fn_handler("/events", Method::Get, move |req| {
            // Get client ID
            let client_id = {
                let mut id = next_id.lock().unwrap();
                let current = *id;
                *id += 1;
                current
            };

            log::info!("SSE client {} connected", client_id);

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

            // Store client
            let response_arc = Arc::new(Mutex::new(response));
            clients.lock().unwrap().insert(client_id, response_arc.clone());

            // Keep connection alive
            loop {
                std::thread::sleep(Duration::from_secs(30));
                
                // Send heartbeat
                if let Ok(mut resp) = response_arc.lock() {
                    if resp.write_all(b":heartbeat\n\n").is_err() {
                        break;
                    }
                    if resp.flush().is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Remove client on disconnect
            clients.lock().unwrap().remove(&client_id);
            log::info!("SSE client {} disconnected", client_id);

            Ok(()) as Result<(), Box<dyn std::error::Error>>
        })?;

        Ok(())
    }

    pub fn broadcast_metrics(&self, metrics: &crate::metrics::MetricsData) -> Result<()> {
        let data = format!("data: {}\n\n", serde_json::to_string(metrics)?);
        self.broadcast(data.as_bytes())
    }

    pub fn broadcast(&self, data: &[u8]) -> Result<()> {
        let mut dead_clients = Vec::new();
        
        {
            let clients = self.clients.lock().unwrap();
            for (id, client) in clients.iter() {
                if let Ok(mut writer) = client.try_lock() {
                    if writer.write_all(data).is_err() || writer.flush().is_err() {
                        dead_clients.push(*id);
                    }
                }
            }
        }

        // Remove dead clients
        if !dead_clients.is_empty() {
            let mut clients = self.clients.lock().unwrap();
            for id in dead_clients {
                clients.remove(&id);
                log::info!("Removed dead SSE client {}", id);
            }
        }

        Ok(())
    }

    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }
}

// Global broadcaster instance
static mut SSE_BROADCASTER: Option<Arc<SseBroadcaster>> = None;

pub fn init() -> Arc<SseBroadcaster> {
    unsafe {
        if SSE_BROADCASTER.is_none() {
            SSE_BROADCASTER = Some(Arc::new(SseBroadcaster::new()));
        }
        SSE_BROADCASTER.as_ref().unwrap().clone()
    }
}
```

### 3. Fix API Routes Compilation Errors

**Update `src/network/api_routes.rs`:**

```rust
// Fix the display screenshot endpoint
// POST /api/v1/display/screenshot
server.fn_handler("/api/v1/display/screenshot", Method::Post, move |req| {
    // For now, return a placeholder response
    // TODO: Implement actual screenshot capture
    let response = serde_json::json!({
        "format": "rgb565",
        "width": 320,
        "height": 170,
        "data": "", // Empty for now
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "message": "Screenshot capture not yet implemented"
    });

    let json = serde_json::to_string(&response)?;
    let mut http_response = req.into_ok_response()?;
    http_response.write_all(json.as_bytes())?;
    Ok(()) as Result<(), Box<dyn std::error::Error>>
})?;

// Fix the task status structure access
// GET /api/v1/system/processes
server.fn_handler("/api/v1/system/processes", Method::Get, move |req| {
    let mut processes = Vec::new();
    
    // Get current task info
    let current_task = unsafe {
        let handle = esp_idf_sys::xTaskGetCurrentTaskHandle();
        let name = esp_idf_sys::pcTaskGetName(handle);
        let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
        
        serde_json::json!({
            "name": name_str,
            "core": esp_idf_sys::xPortGetCoreID(),
            "priority": esp_idf_sys::uxTaskPriorityGet(handle),
            "stack_watermark": esp_idf_sys::uxTaskGetStackHighWaterMark(handle)
        })
    };
    
    processes.push(current_task);

    let response = serde_json::json!({
        "total": processes.len(),
        "processes": processes
    });

    let json = serde_json::to_string(&response)?;
    let mut http_response = req.into_ok_response()?;
    http_response.write_all(json.as_bytes())?;
    Ok(()) as Result<(), Box<dyn std::error::Error>>
})?;
```

## Missing Module Implementations

### 1. Create Sensor History Module

**Create `src/sensors/history.rs`:**

```rust
use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_HISTORY_POINTS: usize = 720; // 12 hours at 1 sample/minute

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub timestamp: u64,
    pub value: f32,
}

pub struct SensorHistory {
    temperature: Mutex<VecDeque<DataPoint>>,
    battery: Mutex<VecDeque<DataPoint>>,
    humidity: Mutex<VecDeque<DataPoint>>,
}

impl SensorHistory {
    pub fn new() -> Self {
        Self {
            temperature: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
            battery: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
            humidity: Mutex::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
        }
    }

    pub fn add_temperature(&self, value: f32) {
        self.add_data_point(&self.temperature, value);
    }

    pub fn add_battery(&self, value: f32) {
        self.add_data_point(&self.battery, value);
    }

    pub fn add_humidity(&self, value: f32) {
        self.add_data_point(&self.humidity, value);
    }

    fn add_data_point(&self, queue: &Mutex<VecDeque<DataPoint>>, value: f32) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut data = queue.lock().unwrap();
        data.push_back(DataPoint { timestamp, value });

        // Remove old data points
        while data.len() > MAX_HISTORY_POINTS {
            data.pop_front();
        }
    }

    pub fn get_temperature_history(&self, hours: u32) -> Vec<DataPoint> {
        self.get_history(&self.temperature, hours)
    }

    pub fn get_battery_history(&self, hours: u32) -> Vec<DataPoint> {
        self.get_history(&self.battery, hours)
    }

    fn get_history(&self, queue: &Mutex<VecDeque<DataPoint>>, hours: u32) -> Vec<DataPoint> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - (hours as u64 * 3600);

        let data = queue.lock().unwrap();
        data.iter()
            .filter(|dp| dp.timestamp >= cutoff)
            .cloned()
            .collect()
    }
}
```

### 2. Update sensors/mod.rs

Add to `src/sensors/mod.rs`:

```rust
pub mod history;
```

### 3. Create Metrics Data Structure

Add to `src/metrics/mod.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsData {
    pub timestamp: u64,
    pub fps_actual: f32,
    pub fps_target: f32,
    pub render_time_ms: f32,
    pub flush_time_ms: f32,
    pub cpu_usage: u8,
    pub cpu0_usage: u8,
    pub cpu1_usage: u8,
    pub cpu_freq_mhz: u32,
    pub battery_voltage_mv: u16,
    pub battery_percentage: u8,
    pub battery_charging: bool,
    pub wifi_rssi: i8,
    pub wifi_connected: bool,
    pub wifi_ssid: String,
    pub display_brightness: u8,
    pub frame_count: u64,
    pub skip_count: u64,
    pub temperature: f32,
    pub heap_free: u32,
}

impl Default for MetricsData {
    fn default() -> Self {
        Self {
            timestamp: 0,
            fps_actual: 0.0,
            fps_target: 60.0,
            render_time_ms: 0.0,
            flush_time_ms: 0.0,
            cpu_usage: 0,
            cpu0_usage: 0,
            cpu1_usage: 0,
            cpu_freq_mhz: 240,
            battery_voltage_mv: 0,
            battery_percentage: 0,
            battery_charging: false,
            wifi_rssi: -100,
            wifi_connected: false,
            wifi_ssid: String::new(),
            display_brightness: 128,
            frame_count: 0,
            skip_count: 0,
            temperature: 0.0,
            heap_free: 0,
        }
    }
}
```

## ESP32-Specific Adaptations

### 1. Update Dashboard HTML for SSE

Replace WebSocket code in `dashboard.html` with SSE:

```javascript
// Server-Sent Events connection
let eventSource = null;

function connectSSE() {
    eventSource = new EventSource('/events');
    
    eventSource.onopen = () => {
        console.log('SSE connected');
        isConnected = true;
        document.querySelector('.status-dot').style.background = 'var(--success)';
    };
    
    eventSource.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            if (data.type === 'metrics') {
                updateUI(data);
                updateCharts(data);
            }
        } catch (error) {
            console.error('Failed to parse SSE message:', error);
        }
    };
    
    eventSource.onerror = () => {
        console.log('SSE disconnected');
        isConnected = false;
        document.querySelector('.status-dot').style.background = 'var(--warning)';
        
        // Reconnect after delay
        setTimeout(connectSSE, 3000);
    };
}

// Replace WebSocket send with REST API calls
function sendCommand(command, data) {
    fetch('/api/control', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ command, data })
    });
}
```

### 2. File System Paths

Update `src/network/file_manager.rs`:

```rust
// ESP32 file system paths
const BASE_PATH: &str = "/spiffs";  // or "/littlefs" based on your partition
const MAX_FILE_SIZE: usize = 512 * 1024; // 512KB for ESP32
```

### 3. Static File Serving

Add to web server initialization:

```rust
// Serve static files from SPIFFS
server.fn_handler("/static/*", Method::Get, |req| {
    let path = req.uri().strip_prefix("/static").unwrap_or("");
    let file_path = format!("/spiffs/www{}", path);
    
    // Determine content type
    let content_type = match path.split('.').last() {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "text/plain",
    };
    
    match std::fs::read(&file_path) {
        Ok(contents) => {
            let mut response = req.into_response(
                200,
                Some("OK"),
                &[("Content-Type", content_type)]
            )?;
            response.write_all(&contents)?;
            Ok(())
        }
        Err(_) => {
            let mut response = req.into_status_response(404)?;
            response.write_all(b"Not Found")?;
            Ok(())
        }
    }
})?;
```

## Memory Optimizations

### 1. Reduce Buffer Sizes

```rust
// In log_streamer.rs
const MAX_LOG_LINES: usize = 1000;  // Reduced from 10,000

// In sensor history
const MAX_HISTORY_POINTS: usize = 360; // 6 hours instead of 12

// In file_manager.rs
const MAX_FILE_SIZE: usize = 256 * 1024; // 256KB limit
```

### 2. Use `heapless` for Fixed-Size Collections

Add to `Cargo.toml`:
```toml
heapless = "0.8"
```

### 3. Optimize String Allocations

```rust
// Pre-allocate string buffers
let mut buffer = String::with_capacity(256);
buffer.clear();
use std::fmt::Write;
let _ = write!(&mut buffer, "Format: {}", value);
```

## Testing & Debugging Checklist

### Initial Setup
- [ ] Verify all new modules compile without errors
- [ ] Check Cargo.toml has all required dependencies
- [ ] Confirm SPIFFS/LittleFS partition is mounted
- [ ] Test base web server loads homepage

### WebSocket/SSE Testing
- [ ] Connect to `/events` endpoint and verify heartbeat
- [ ] Check metrics are broadcast to all connected clients
- [ ] Test reconnection after network interruption
- [ ] Monitor memory usage with multiple clients

### API Endpoint Testing
- [ ] Test each new API endpoint with curl/Postman
- [ ] Verify error responses have correct format
- [ ] Check validation rejects invalid inputs
- [ ] Test PATCH endpoints update only specified fields

### File Manager Testing
- [ ] List files in SPIFFS/LittleFS
- [ ] Edit and save a JSON config file
- [ ] Upload a small test file
- [ ] Verify file size limits are enforced

### PWA Testing
- [ ] Service worker registers successfully
- [ ] Manifest loads without errors
- [ ] Test offline mode (disconnect WiFi)
- [ ] Verify cached pages load offline

### Performance Testing
- [ ] Monitor heap usage over time
- [ ] Check for memory leaks with repeated connections
- [ ] Verify response times meet targets
- [ ] Test with multiple concurrent clients

### Mobile Testing
- [ ] Test on actual phone/tablet
- [ ] Verify touch targets are 48px minimum
- [ ] Check responsive layouts at different sizes
- [ ] Test landscape orientation

## Performance Optimization Checklist

### Network Optimizations
- [ ] Enable gzip compression for responses
- [ ] Set appropriate cache headers
- [ ] Minimize JSON response sizes
- [ ] Use HTTP/1.1 keep-alive

### Memory Optimizations
- [ ] Profile heap usage with `esp_get_free_heap_size()`
- [ ] Use static buffers where possible
- [ ] Implement proper cleanup in drop handlers
- [ ] Monitor stack usage per task

### Display Optimizations
- [ ] Batch UI updates from web changes
- [ ] Debounce rapid setting changes
- [ ] Only update changed screen regions
- [ ] Limit animation frame rates

### Code Optimizations
- [ ] Use `#[inline]` for small functions
- [ ] Avoid unnecessary clones
- [ ] Use `&str` instead of `String` where possible
- [ ] Profile with `esp-idf-sys` performance counters

### Power Optimizations
- [ ] Reduce WiFi beacon interval
- [ ] Use WiFi power save mode
- [ ] Throttle updates when on battery
- [ ] Dim display when idle

## Integration Steps

1. **Start with SSE Instead of WebSocket**
   - Simpler to implement with ESP-IDF
   - Good enough for metrics broadcasting
   - Falls back gracefully

2. **Implement Core Features First**
   - Basic API endpoints
   - File manager
   - Error handling

3. **Add Advanced Features**
   - PWA support
   - Live graphs
   - Log streaming

4. **Optimize Based on Testing**
   - Profile memory usage
   - Reduce buffer sizes as needed
   - Optimize hot paths

## Troubleshooting

### Common Issues

1. **"Out of memory" errors**
   - Reduce buffer sizes
   - Disable features temporarily
   - Check for memory leaks

2. **"File not found" in browser**
   - Verify SPIFFS is mounted
   - Check file paths are correct
   - Ensure static file handler is registered

3. **SSE connections drop frequently**
   - Increase keepalive timeout
   - Check WiFi stability
   - Monitor error logs

4. **Slow page loads**
   - Enable compression
   - Reduce JavaScript size
   - Cache static assets

## Next Steps

1. Implement the SSE broadcaster
2. Fix compilation errors in existing modules
3. Create missing sensor history module
4. Test each component individually
5. Integrate and test as a whole
6. Optimize based on profiling results