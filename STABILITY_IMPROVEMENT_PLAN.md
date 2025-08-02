# ESP32-S3 Dashboard Stability Improvement Plan

## Current Issues Analysis

### 1. **Compilation Warnings Impact**
Total warnings: 11 (could increase binary size and reduce stability)

#### High Priority Warnings:
- **Unused retry mechanisms**: Created but never integrated, causing web server to not recover
- **Unused power management**: Device stays at full power, causing heat and battery drain
- **Unused startup variables**: Incomplete startup grace period implementation

#### Medium Priority:
- Dead code increasing binary size (1.42 MB is large for ESP32)
- Unused functions consuming RAM

### 2. **Runtime Stability Issues**

#### Critical:
1. **mDNS Failure**: `ESP_ERR_INVALID_STATE` - prevents reliable discovery
2. **Long boot time**: 42 seconds from power to web ready
3. **Watchdog disabled during WiFi scan**: Risk of permanent hang
4. **No recovery mechanism**: If web server crashes, device needs manual restart

#### Important:
1. **Memory fragmentation**: No heap monitoring
2. **No panic handler**: Device just freezes on panic
3. **Single-threaded web server**: Can block on long requests

## Fixes Applied Today

1. ✅ **mDNS fix**: Handle already-initialized state gracefully
2. ✅ **Web server retry**: Integrated retry mechanism into main.rs
3. ✅ **Removed unused modules**: Deleted web_server_manager.rs and web_server_retry.rs
4. ✅ **Fixed unused variables**: Added underscore prefix

## Immediate Actions Needed

### 1. **Implement Watchdog Feeding During WiFi Scan**
```rust
// In wifi.rs - replace watchdog disable with feeding
pub fn scan_with_watchdog() -> Result<Vec<AccessPointInfo>> {
    let handle = thread::spawn(|| {
        for _ in 0..30 {
            FreeRtos::delay_ms(1000);
            unsafe { esp_idf_sys::esp_task_wdt_reset(); }
        }
    });
    
    let result = self.wifi.scan()?;
    handle.join().unwrap();
    Ok(result)
}
```

### 2. **Add Heap Monitoring**
```rust
// In main loop
if frame_count % 600 == 0 {  // Every 10 seconds
    let free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
    if free < 100_000 {
        log::warn!("Low heap: {} bytes", free);
        // Consider restart if < 50KB
    }
}
```

### 3. **Implement Panic Handler**
```rust
// In main.rs at startup
std::panic::set_hook(Box::new(|info| {
    log::error!("PANIC: {}", info);
    // Flush logs
    FreeRtos::delay_ms(500);
    // Restart
    unsafe { esp_idf_sys::esp_restart(); }
}));
```

### 4. **Reduce Boot Time**
- Move WiFi scan to background after initial connection
- Start web server before full sensor init
- Use static IP if available (skip DHCP wait)

## Long-term Improvements

### 1. **Memory Optimization**
- Reduce string allocations in hot paths
- Use `heapless` for fixed-size collections
- Profile memory usage with embedded-profiling

### 2. **Power Management**
- Implement the unused PowerManager properly
- Add display timeout (dim after 30s, off after 5min)
- CPU frequency scaling based on load

### 3. **Reliability Features**
- Add hardware watchdog (external timer)
- Implement brown-out detection
- Add crash reporting to flash

### 4. **Network Stability**
- Implement exponential backoff for WiFi reconnect
- Add connection quality monitoring
- Fallback AP mode if STA fails

## Binary Size Reduction

Current: 1.42 MB → Target: < 1.0 MB

1. Remove unused dependencies
2. Use `opt-level = "z"` for size optimization
3. Strip debug symbols in release
4. Use LTO (Link Time Optimization)

## Testing Strategy

1. **Stress test**: Run for 24 hours continuously
2. **Network interruption**: Test WiFi disconnect/reconnect
3. **Memory pressure**: Allocate until near limit
4. **Power cycling**: Rapid on/off cycles
5. **OTA updates**: Verify recovery after failed update

## Monitoring Additions

Add these metrics to `/metrics`:
- `esp32_restarts_total` - Count of device restarts
- `esp32_heap_min_bytes` - Minimum heap seen
- `esp32_wifi_disconnects_total` - WiFi stability metric
- `esp32_web_requests_failed_total` - Web server errors

## Configuration Changes

In `sdkconfig.defaults`:
```
# Increase watchdog timeout for stability
CONFIG_ESP_TASK_WDT_TIMEOUT_S=15

# Enable brownout detector
CONFIG_ESP_BROWNOUT_DET=y
CONFIG_ESP_BROWNOUT_DET_LVL_SEL_0=y

# Increase WiFi buffers for stability
CONFIG_ESP32_WIFI_TX_BUFFER_TYPE=0
CONFIG_ESP32_WIFI_STATIC_TX_BUFFER_NUM=32

# Enable core dump on crash
CONFIG_ESP_COREDUMP_ENABLE_TO_FLASH=y
```

## Success Metrics

After implementing these fixes:
- Boot time: < 15 seconds
- Uptime: > 7 days without restart
- Memory: Always > 150KB free heap
- Response time: < 100ms for web requests
- Recovery: Auto-restart within 30s of crash