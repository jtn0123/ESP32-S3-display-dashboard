# ESP32-S3 Device Freeze Root Cause Analysis

## Summary
The device freezes when handling rapid HTTP connections due to socket exhaustion in the ESP-IDF HTTP server.

## Root Cause
The ESP-IDF HTTP server has several default limits that are not being configured:
- **max_open_sockets**: Default is typically 10
- **backlog_conn**: Default is typically 5
- **recv_wait_timeout**: Default timeout for receiving data
- **send_wait_timeout**: Default timeout for sending data

When clients make rapid connections (especially without proper connection reuse), the server quickly exhausts its socket pool and becomes unresponsive.

## Evidence
1. **Sequential requests work fine** - up to 8-10 requests with delays
2. **Rapid connections fail** - device freezes after ~10-15 rapid connections
3. **Concurrent connections trigger freeze** - multiple simultaneous connections exhaust sockets
4. **Recovery after 20-30s** - matches socket cleanup timeout

## Current Configuration
```rust
// src/network/web_server.rs
let server_config = Configuration {
    stack_size: 8192,
    max_uri_handlers: 40,
    ..Default::default()  // <-- Missing socket configuration!
};
```

## Recommended Fix
```rust
let server_config = Configuration {
    stack_size: 8192,
    max_uri_handlers: 40,
    max_open_sockets: 16,    // Increase from default 10
    backlog_conn: 10,        // Increase connection backlog
    recv_wait_timeout: 5,    // 5 second receive timeout
    send_wait_timeout: 5,    // 5 second send timeout
    ..Default::default()
};
```

## Additional Recommendations

### 1. Connection Management
- Implement connection rate limiting
- Add proper Connection: close headers for non-keepalive requests
- Monitor active connection count

### 2. Resource Monitoring
- Track socket usage in metrics
- Add warnings when approaching limits
- Implement graceful degradation

### 3. Client-Side Improvements
- Use connection pooling in tests
- Implement exponential backoff on failures
- Respect Keep-Alive headers

### 4. Watchdog Enhancement
- Ensure watchdog covers HTTP handler tasks
- Add HTTP-specific health checks
- Implement automatic recovery on socket exhaustion

## Test Results

### What Works
- Single sequential requests with delays
- Limited concurrent connections (< 5)
- Requests with connection reuse

### What Causes Freeze
- Rapid connection cycling (30 requests, no delay)
- 10+ persistent connections
- Concurrent requests to multiple endpoints
- Large response handling under load

## Verification
After implementing the fix, run:
```bash
python3 tests/python/stress_test.py
python3 tests/python/analyze_freeze.py
```

The device should handle significantly more concurrent connections without freezing.