# MCP Server Integration Guide for ESP32-S3 Dashboard

This guide explains how to set up and use MCP (Model Context Protocol) servers with the ESP32-S3 Dashboard for advanced monitoring and debugging.

## Overview

The ESP32 dashboard now supports three MCP servers for comprehensive monitoring:

1. **MCP Telemetry** - OpenTelemetry tracing for performance debugging
2. **Grafana MCP** - Prometheus metrics visualization  
3. **System Monitor MCP** - Real-time system resource monitoring

## Implementation Details

### 1. MCP Telemetry Server Setup

#### Installation
```bash
# Clone the MCP Telemetry repository
git clone https://github.com/xprilion/mcp-telemetry.git
cd mcp-telemetry

# Get Weights & Biases API key from wandb.com
export WANDB_API_KEY="your-api-key-here"
```

#### Configure Claude Desktop
Add to your Claude desktop config file:
```json
{
  "mcpServers": {
    "esp32-telemetry": {
      "command": "uv",
      "args": [
        "run",
        "--with",
        "mcp[cli]",
        "--with",
        "weave",
        "mcp",
        "run",
        "~/mcp-telemetry/server.py"
      ],
      "env": {
        "WANDB_API_KEY": "your-api-key-here",
        "ESP32_TELNET_HOST": "192.168.1.100",
        "ESP32_TELNET_PORT": "23"
      }
    }
  }
}
```

#### ESP32 Integration
The ESP32 exports OpenTelemetry-compatible traces via telnet on port 23. Format:
```json
{
  "resourceSpans": [{
    "resource": {
      "attributes": [{
        "key": "service.name",
        "value": {"stringValue": "esp32-dashboard"}
      }]
    },
    "scopeSpans": [{
      "spans": [{
        "traceId": "esp32-trace-1",
        "spanId": "span-1",
        "name": "display_render",
        "startTimeUnixNano": 1735000000000000,
        "endTimeUnixNano": 1735000015000000,
        "attributes": [
          {"key": "fps", "value": {"doubleValue": 55.2}},
          {"key": "cpu_usage", "value": {"intValue": 45}}
        ]
      }]
    }]
  }]
}
```

### 2. Grafana MCP Server Setup

#### Prerequisites
- Grafana 9.0+ running locally or remotely
- Service account with appropriate permissions

#### Installation
```bash
# Install via Go
go install github.com/grafana/mcp-grafana/cmd/mcp-grafana@latest

# Or use Docker
docker pull mcp/grafana
```

#### Create Grafana Service Account
1. Go to Grafana → Configuration → Service Accounts
2. Create new service account with permissions:
   - Dashboard: View, Edit
   - Datasources: View
   - Metrics: Query
3. Generate service account token

#### Configure Claude Desktop
```json
{
  "mcpServers": {
    "grafana-esp32": {
      "command": "mcp-grafana",
      "args": [],
      "env": {
        "GRAFANA_URL": "http://localhost:3000",
        "GRAFANA_API_KEY": "your-service-account-token"
      }
    }
  }
}
```

#### Set up Prometheus + Grafana
```bash
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'esp32'
    static_configs:
      - targets: ['192.168.1.100:80']
    metrics_path: '/metrics'
```

#### ESP32 Metrics Available
The ESP32 exposes Prometheus metrics at `http://<ESP32-IP>/metrics`:
- `esp32_uptime_seconds` - Total uptime
- `esp32_heap_free_bytes` - Available memory
- `esp32_fps_actual` - Current FPS (55-65 with DMA)
- `esp32_cpu_usage_percent` - CPU utilization
- `esp32_temperature_celsius` - Internal temperature
- `esp32_wifi_rssi_dbm` - WiFi signal strength
- `esp32_render_time_milliseconds` - Frame render histogram

### 3. System Monitor MCP Setup

#### Installation
```bash
git clone https://github.com/seekrays/mcp-monitor.git
cd mcp-monitor
make build
```

#### Configure Claude Desktop
```json
{
  "mcpServers": {
    "system-monitor": {
      "command": "./mcp-monitor",
      "args": ["--include-remote", "esp32"],
      "env": {
        "REMOTE_HOSTS": "esp32:192.168.1.100:23"
      }
    }
  }
}
```

#### ESP32 System Metrics Export
The ESP32 exports system metrics via telnet in JSON format:
```json
{
  "host": "esp32-display",
  "timestamp": 1735000000,
  "cpu": {
    "cores": 2,
    "usage": [45, 12],
    "frequency_mhz": 240
  },
  "memory": {
    "heap_free": 125432,
    "heap_total": 327680,
    "psram_free": 4194304,
    "psram_total": 8388608
  },
  "network": {
    "wifi_rssi": -65,
    "ip": "192.168.1.100",
    "tx_bytes": 1234567,
    "rx_bytes": 7654321
  },
  "display": {
    "fps": 55.2,
    "brightness": 255,
    "backlight": true
  }
}
```

## Usage Examples

### With MCP Telemetry
```
You: "Trace the display performance over the next minute"
Claude: "I'll start tracing ESP32 display performance..."
[Uses MCP Telemetry to capture and analyze traces]
```

### With Grafana MCP
```
You: "Show me the FPS trends for the last hour"
Claude: "Let me query the Grafana dashboard for ESP32 metrics..."
[Uses Grafana MCP to retrieve and visualize data]
```

### With System Monitor
```
You: "What's the current resource usage on the ESP32?"
Claude: "Checking ESP32 system resources..."
[Uses System Monitor MCP to get real-time metrics]
```

## Implementation Steps

1. **Enable telnet logging** on ESP32 (already done on port 23)
2. **Add metrics endpoint** to web server at `/metrics` (implemented)
3. **Install MCP servers** on your development machine
4. **Configure Claude Desktop** with the MCP server configs
5. **Set up Prometheus** to scrape ESP32 metrics
6. **Configure Grafana** dashboards for visualization

## Monitoring Architecture

```
┌─────────────┐     Telnet:23    ┌──────────────┐
│   ESP32-S3  ├─────────────────→│ MCP Telemetry│
│  Dashboard  │                   └──────────────┘
│             │     HTTP:80/      ┌──────────────┐
│  - Telnet   ├─────metrics──────→│  Prometheus  │
│  - Web API  │                   └──────┬───────┘
│  - Metrics  │                           │
└─────────────┘                   ┌───────▼──────┐
                                  │   Grafana    │
                                  │   + MCP      │
                                  └──────────────┘
```

## Benefits

1. **Performance Debugging**: Trace render times, identify bottlenecks
2. **Historical Analysis**: Track FPS improvements over time
3. **Resource Monitoring**: Watch memory usage, CPU load
4. **Remote Debugging**: Monitor device without USB connection
5. **AI-Assisted Analysis**: Claude can directly query and analyze metrics

## Troubleshooting

### MCP Telemetry Issues
- Ensure telnet port 23 is accessible
- Check WANDB_API_KEY is valid
- Verify ESP32 IP address is correct

### Grafana MCP Issues
- Confirm Grafana version is 9.0+
- Check service account permissions
- Verify Prometheus is scraping metrics

### System Monitor Issues
- Ensure mcp-monitor binary is built
- Check network connectivity to ESP32
- Verify JSON format from telnet

## Next Steps

1. Create custom Grafana dashboards for ESP32 metrics
2. Set up alerts for performance degradation
3. Implement trace sampling for production
4. Add custom trace attributes for debugging