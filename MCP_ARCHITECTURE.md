# MCP Server Architecture for ESP32 Monitoring

## Overview

MCP servers run on your Mac and connect to your ESP32 over the network. They don't run on the ESP32 itself.

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Your Mac (Development Machine)                │
│                                                                     │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────┐       │
│  │   Claude    │───▶│ Grafana MCP  │───▶│   Grafana      │       │
│  │   Desktop   │    │   Server     │    │  (port 3000)   │       │
│  └─────────────┘    └──────────────┘    └───────┬────────┘       │
│         │                                        │                 │
│         │           ┌──────────────┐    ┌───────▼────────┐       │
│         ├──────────▶│MCP Telemetry │    │   Prometheus   │       │
│         │           │   Server     │    │  (port 9090)   │       │
│         │           └──────────────┘    └───────┬────────┘       │
│         │                  │                     │                 │
│         │           ┌──────▼───────┐            │ HTTP scrape     │
│         └──────────▶│System Monitor│            │ every 15s       │
│                     │  MCP Server  │            │                 │
│                     └──────┬───────┘            │                 │
└─────────────────────────────┼────────────────────┼─────────────────┘
                             │ Telnet              │ HTTP
                             │ Port 23             │ Port 80
                             ▼                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          ESP32-S3 (Device)                          │
│                                                                     │
│  ┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐  │
│  │  Telnet Server  │    │  Web Server  │    │ Display/Sensors │  │
│  │   (Port 23)     │    │  (Port 80)   │    │   (55-65 FPS)   │  │
│  └─────────────────┘    └──────────────┘    └─────────────────┘  │
│         │                       │                      │           │
│         │  Logs/Traces         │  /metrics endpoint   │           │
│         └───────────────────────┴──────────────────────┘           │
│                                                                     │
│                    Running firmware v5.37-mcp                       │
└─────────────────────────────────────────────────────────────────────┘
```

## Directory Structure

```
Your Mac:
~/
├── Documents/Github/ESP32-S3-Display-Dashboard/  # Your project
│   ├── src/                                      # ESP32 source code
│   ├── MCP_INTEGRATION_GUIDE.md                  # Setup guide
│   └── setup-mcp-servers.sh                      # Setup script
│
└── mcp-servers/                                  # MCP servers (separate)
    ├── mcp-telemetry/                           # Cloned repo
    ├── mcp-monitor/                             # Cloned repo
    ├── prometheus.yml                           # Prometheus config
    └── claude-mcp-config.json                   # Claude config
```

## How Data Flows

1. **ESP32 generates metrics** (FPS, temperature, CPU usage)
2. **Metrics exposed** at `http://<ESP32-IP>/metrics`
3. **Prometheus scrapes** these metrics every 15 seconds
4. **Grafana visualizes** the Prometheus data
5. **MCP servers** allow Claude to:
   - Query Grafana dashboards
   - Analyze telemetry traces
   - Monitor system resources

## Key Points

- **MCP servers are NOT part of your ESP32 code**
- They run on your Mac as separate processes
- They connect to your ESP32 over WiFi
- You can close your ESP32 project and MCP servers keep running
- Claude uses MCP servers to interact with your ESP32 remotely

## Quick Start

1. Run the setup script:
   ```bash
   ./setup-mcp-servers.sh
   ```

2. Install required tools:
   ```bash
   brew install prometheus grafana
   ```

3. Start the monitoring stack:
   ```bash
   cd ~/mcp-servers
   ./start-prometheus.sh    # Terminal 1
   ./start-grafana.sh       # Terminal 2
   ```

4. Configure Claude Desktop with the generated config

5. Test in Claude:
   ```
   You: "What's the current FPS on my ESP32?"
   Claude: [Uses Grafana MCP to query metrics]
   ```