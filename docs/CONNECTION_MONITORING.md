# Connection Monitoring

## Overview

The ESP32-S3 Dashboard includes comprehensive connection monitoring capabilities to track network connectivity, service usage, and uptime. All connection metrics are exposed via the Prometheus-compatible `/metrics` endpoint for monitoring and alerting.

## Metrics Available

### HTTP Connection Metrics
- `esp32_http_connections_active` - Currently active HTTP connections
- `esp32_http_connections_total` - Total HTTP connections handled since boot

### Telnet Connection Metrics
- `esp32_telnet_connections_active` - Currently active telnet connections
- `esp32_telnet_connections_total` - Total telnet connections handled since boot

### WiFi Connectivity Metrics
- `esp32_wifi_disconnects_total` - Total number of WiFi disconnections
- `esp32_wifi_reconnects_total` - Total number of successful WiFi reconnections
- `esp32_wifi_connected` - Current WiFi connection status (0=disconnected, 1=connected)
- `esp32_wifi_rssi_dbm` - WiFi signal strength in dBm

### Uptime Metrics
- `esp32_session_uptime_seconds` - Current session uptime in seconds
- `esp32_uptime_seconds` - Total device uptime (persists across reboots)

## Implementation Details

### HTTP Connection Tracking
HTTP connections are tracked by the web server infrastructure. The metrics are updated:
- When a new connection is accepted
- When a connection is closed
- During periodic cleanup of stale connections

### Telnet Connection Tracking
The telnet server maintains its own connection tracking:
```rust
// In TelnetLogServer
total_connections: Arc<Mutex<u64>>,
clients: Arc<Mutex<Vec<Arc<Mutex<TcpStream>>>>>,
```

Metrics are updated:
- When a client connects
- When a client disconnects
- During periodic cleanup

### WiFi Reconnection Tracking
The NetworkManager tracks WiFi events:
```rust
disconnect_count: Arc<Mutex<u32>>,
reconnect_count: Arc<Mutex<u32>>,
```

These counters are incremented by the WiFi event handlers when disconnection/reconnection events occur.

### Uptime Tracking
Session uptime is tracked from boot and updated every metrics report cycle (typically every 5 seconds).

## Testing

### Manual Testing
1. Access the metrics endpoint:
   ```bash
   curl http://<device-ip>/metrics | grep -E "(connections|uptime|wifi)"
   ```

2. Monitor connections in real-time:
   ```bash
   watch -n 1 'curl -s http://<device-ip>/metrics | grep connections'
   ```

### Automated Testing
Run the connection monitoring test script:
```bash
cd tests/python
./test_connection_monitoring.py <device-ip>
```

This script will:
- Create multiple HTTP connections concurrently
- Establish telnet connections
- Verify metrics are updating correctly
- Test connection cleanup
- Verify uptime tracking

## Grafana Dashboard

Create a Grafana dashboard with these queries:

### Connection Overview
```promql
# Active connections
esp32_http_connections_active
esp32_telnet_connections_active

# Connection rate
rate(esp32_http_connections_total[5m])
rate(esp32_telnet_connections_total[5m])
```

### WiFi Stability
```promql
# Disconnection rate
rate(esp32_wifi_disconnects_total[1h])

# Reconnection success rate
rate(esp32_wifi_reconnects_total[1h]) / rate(esp32_wifi_disconnects_total[1h])
```

### Uptime
```promql
# Current session uptime in hours
esp32_session_uptime_seconds / 3600

# Total uptime in days
esp32_uptime_seconds / 86400
```

## Alert Examples

### High Connection Rate Alert
```yaml
alert: HighHTTPConnectionRate
expr: rate(esp32_http_connections_total[5m]) > 10
for: 5m
annotations:
  summary: "High HTTP connection rate on ESP32"
  description: "{{ $value }} connections per second"
```

### WiFi Instability Alert
```yaml
alert: WiFiInstability
expr: rate(esp32_wifi_disconnects_total[1h]) > 5
for: 10m
annotations:
  summary: "WiFi connection unstable"
  description: "{{ $value }} disconnections per hour"
```

### No Active Connections Alert
```yaml
alert: NoActiveConnections
expr: esp32_http_connections_active == 0 and esp32_telnet_connections_active == 0
for: 30m
annotations:
  summary: "No active connections to ESP32"
  description: "Device may be unreachable"
```

## Performance Impact

Connection monitoring has minimal performance impact:
- Atomic counters for most metrics (lock-free)
- Metrics updated only on connection events
- No polling or background tasks required
- Memory usage: ~100 bytes for tracking structures

## Troubleshooting

### Metrics Not Updating
1. Check if metrics endpoint is accessible
2. Verify services are running (telnet on port 23)
3. Check logs for connection errors
4. Ensure WiFi is stable

### High Connection Counts
1. Check for connection leaks in client code
2. Verify HTTP keep-alive settings
3. Monitor for network scanning/attacks
4. Check web server max connection limits

### WiFi Disconnect Tracking
1. Monitor signal strength (RSSI)
2. Check for interference
3. Verify router stability
4. Review power management settings