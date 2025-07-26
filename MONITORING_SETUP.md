# ESP32 Monitoring Setup Guide

This guide explains how to use the monitoring infrastructure for your ESP32-S3 Display Dashboard.

## Quick Links

- **Grafana Dashboard**: http://localhost:3000/d/esp32-dashboard/esp32-s3-display-dashboard
- **Prometheus**: http://localhost:9090
- **ESP32 Metrics**: http://10.27.27.201/metrics (replace with your ESP32's IP)
- **MCP Servers**: Located in `~/mcp-servers/`

## What Gets Installed Where

### On Your ESP32 (This Project)
- **Firmware v5.37-mcp** with telemetry support
- **/metrics endpoint** - Prometheus-compatible metrics
- **Telnet server** on port 23 for log streaming

### On Your Mac (Separate Directory)
- **~/mcp-servers/** - All MCP server installations
  - `mcp-telemetry/` - OpenTelemetry tracing
  - `mcp-monitor/` - System monitoring
  - `prometheus.yml` - Prometheus config
  - `claude-mcp-config.json` - Claude Desktop config

### System Services (Homebrew)
- **Prometheus** - Time-series database
- **Grafana** - Metrics visualization
- **Go** - Required for building MCP servers

## After Computer Restart

The monitoring stack doesn't start automatically. Here's how to restart everything:

### 1. Start Prometheus
```bash
cd ~/mcp-servers
prometheus --config.file=prometheus.yml &
```

### 2. Start Grafana
```bash
brew services start grafana
```

### 3. Verify Everything is Running
```bash
# Check Prometheus
curl http://localhost:9090/api/v1/targets

# Check Grafana
curl http://localhost:3000/api/health

# Check ESP32 metrics
curl http://10.27.27.201/metrics
```

## After ESP32 Restart

The ESP32 automatically:
- ✅ Starts the metrics endpoint at `/metrics`
- ✅ Starts telnet server on port 23
- ✅ Connects to WiFi (using saved credentials)

No action needed! Prometheus will automatically resume scraping.

## Viewing Your Data

### Grafana Dashboard
1. Open http://localhost:3000
2. Login: `admin` / `admin`
3. Go to Dashboards → ESP32-S3 Display Dashboard

You'll see:
- **FPS Graph** - Real-time frames per second (should show 55-65 FPS)
- **CPU Usage Gauge** - Current CPU utilization
- **Temperature Graph** - Internal temperature over time
- **Memory Graph** - Free heap memory trends

### Direct Prometheus Queries
```bash
# Current FPS
curl -s 'http://localhost:9090/api/v1/query?query=esp32_fps_actual' | jq .

# CPU usage
curl -s 'http://localhost:9090/api/v1/query?query=esp32_cpu_usage_percent' | jq .

# All ESP32 metrics
curl -s 'http://localhost:9090/api/v1/label/__name__/values' | jq . | grep esp32
```

## Using with Claude Desktop (MCP)

### Setup (One Time)
1. Find Claude Desktop config location (varies by installation)
2. Copy the MCP config:
   ```bash
   cp ~/mcp-servers/claude-mcp-config.json <claude-config-location>
   ```

### What You Can Ask Claude
With MCP servers configured, Claude can:
- "Show me the current FPS on my ESP32"
- "What's the temperature trend over the last hour?"
- "Is the ESP32 memory usage stable?"
- "Compare CPU usage before and after the last update"

## Troubleshooting

### Prometheus Not Scraping
```bash
# Check Prometheus targets
curl http://localhost:9090/targets

# Check ESP32 is reachable
ping 10.27.27.201

# Check metrics endpoint
curl http://10.27.27.201/metrics
```

### Grafana Not Showing Data
1. Check data source: Settings → Data Sources → Prometheus
2. Test connection (should show "Data source is working")
3. Check time range (top right) - set to "Last 5 minutes"

### ESP32 IP Changed
1. Find new IP:
   ```bash
   # Look for ESP32 MAC address
   arp -a | grep b4:3a:45
   ```

2. Update Prometheus config:
   ```bash
   # Edit ~/mcp-servers/prometheus.yml
   # Change the IP in targets: ['NEW_IP:80']
   
   # Restart Prometheus
   pkill prometheus
   cd ~/mcp-servers && prometheus --config.file=prometheus.yml &
   ```

## Performance Impact

The monitoring has minimal impact on ESP32:
- **Metrics endpoint**: ~1ms to generate
- **Memory overhead**: < 1KB
- **CPU impact**: < 0.1%
- **Network traffic**: ~2KB every 5 seconds

## Security Notes

- Metrics endpoint is read-only
- No authentication on metrics (local network only)
- Grafana has default admin/admin (change in production)
- Telnet is unencrypted (local network only)

## Creating Custom Dashboards

1. In Grafana, click "+" → "Create Dashboard"
2. Add panels using these metrics:
   - `esp32_fps_actual` - Current FPS
   - `esp32_cpu_usage_percent` - CPU usage
   - `esp32_temperature_celsius` - Temperature
   - `esp32_heap_free_bytes` - Free memory
   - `esp32_wifi_rssi_dbm` - WiFi signal strength
   - `esp32_render_time_milliseconds` - Render timing histogram

## Uninstalling

If you want to remove the monitoring:

```bash
# Stop services
brew services stop grafana
pkill prometheus

# Remove MCP servers
rm -rf ~/mcp-servers

# Uninstall tools (optional)
brew uninstall prometheus grafana go

# ESP32 will continue working normally
# The /metrics endpoint just won't be used
```

## Learn More

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [ESP32 Project README](README.md)