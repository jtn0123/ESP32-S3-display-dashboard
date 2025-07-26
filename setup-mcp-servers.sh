#!/bin/bash

echo "ESP32 MCP Server Setup Script"
echo "=============================="

# Get ESP32 IP address
echo -e "\nFirst, let's find your ESP32's IP address..."
echo "Make sure your ESP32 is connected to WiFi."
read -p "Enter your ESP32's IP address (e.g., 192.168.1.100): " ESP32_IP

# Create MCP servers directory
MCP_DIR="$HOME/mcp-servers"
echo -e "\nCreating MCP servers directory at $MCP_DIR..."
mkdir -p "$MCP_DIR"
cd "$MCP_DIR"

# 1. Install Grafana MCP
echo -e "\n1. Installing Grafana MCP Server..."
if command -v go &> /dev/null; then
    go install github.com/grafana/mcp-grafana/cmd/mcp-grafana@latest
    echo "✓ Grafana MCP installed"
else
    echo "❌ Go not installed. Install from https://golang.org/dl/"
fi

# 2. Clone MCP Telemetry
echo -e "\n2. Setting up MCP Telemetry..."
if [ ! -d "mcp-telemetry" ]; then
    git clone https://github.com/xprilion/mcp-telemetry.git
    echo "✓ MCP Telemetry cloned"
else
    echo "✓ MCP Telemetry already exists"
fi

# 3. Clone System Monitor
echo -e "\n3. Setting up System Monitor MCP..."
if [ ! -d "mcp-monitor" ]; then
    git clone https://github.com/seekrays/mcp-monitor.git
    cd mcp-monitor
    make build
    cd ..
    echo "✓ System Monitor built"
else
    echo "✓ System Monitor already exists"
fi

# 4. Create Prometheus config
echo -e "\n4. Creating Prometheus configuration..."
cat > "$MCP_DIR/prometheus.yml" << EOF
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'esp32-dashboard'
    static_configs:
      - targets: ['${ESP32_IP}:80']
    metrics_path: '/metrics'
    scrape_interval: 5s
EOF
echo "✓ Prometheus config created at $MCP_DIR/prometheus.yml"

# 5. Create Claude Desktop config
echo -e "\n5. Creating Claude Desktop configuration..."
CLAUDE_CONFIG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
cat > "$MCP_DIR/claude-mcp-config.json" << EOF
{
  "mcpServers": {
    "grafana-esp32": {
      "command": "mcp-grafana",
      "args": [],
      "env": {
        "GRAFANA_URL": "http://localhost:3000",
        "GRAFANA_API_KEY": "YOUR_GRAFANA_SERVICE_ACCOUNT_TOKEN"
      }
    },
    "system-monitor": {
      "command": "$MCP_DIR/mcp-monitor/mcp-monitor",
      "args": [],
      "env": {
        "REMOTE_HOSTS": "esp32:${ESP32_IP}:23"
      }
    },
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
        "$MCP_DIR/mcp-telemetry/server.py"
      ],
      "env": {
        "WANDB_API_KEY": "YOUR_WANDB_API_KEY",
        "ESP32_TELNET_HOST": "${ESP32_IP}",
        "ESP32_TELNET_PORT": "23"
      }
    }
  }
}
EOF
echo "✓ MCP config template created at $MCP_DIR/claude-mcp-config.json"

# 6. Create start script
echo -e "\n6. Creating startup scripts..."
cat > "$MCP_DIR/start-prometheus.sh" << 'EOF'
#!/bin/bash
echo "Starting Prometheus..."
prometheus --config.file=prometheus.yml
EOF
chmod +x "$MCP_DIR/start-prometheus.sh"

cat > "$MCP_DIR/start-grafana.sh" << 'EOF'
#!/bin/bash
echo "Starting Grafana..."
brew services start grafana
echo "Grafana running at http://localhost:3000"
echo "Default login: admin/admin"
EOF
chmod +x "$MCP_DIR/start-grafana.sh"

# 7. Create ESP32 dashboard for Grafana
cat > "$MCP_DIR/esp32-dashboard.json" << 'EOF'
{
  "dashboard": {
    "title": "ESP32-S3 Display Dashboard",
    "panels": [
      {
        "title": "FPS (Actual)",
        "targets": [{"expr": "esp32_fps_actual"}],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
      },
      {
        "title": "CPU Usage %",
        "targets": [{"expr": "esp32_cpu_usage_percent"}],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
      },
      {
        "title": "Temperature",
        "targets": [{"expr": "esp32_temperature_celsius"}],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8}
      },
      {
        "title": "Free Heap",
        "targets": [{"expr": "esp32_heap_free_bytes"}],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8}
      }
    ]
  }
}
EOF

echo -e "\n✅ Setup Complete!"
echo -e "\nNext steps:"
echo "1. Install Prometheus: brew install prometheus"
echo "2. Install Grafana: brew install grafana"
echo "3. Get API keys:"
echo "   - Weights & Biases: https://wandb.ai/authorize"
echo "   - Grafana: Create service account in Grafana UI"
echo "4. Update $MCP_DIR/claude-mcp-config.json with your API keys"
echo "5. Copy MCP config to Claude: cp $MCP_DIR/claude-mcp-config.json \"$CLAUDE_CONFIG\""
echo ""
echo "To start monitoring:"
echo "  cd $MCP_DIR"
echo "  ./start-prometheus.sh  # In one terminal"
echo "  ./start-grafana.sh     # In another terminal"
echo ""
echo "Your ESP32 metrics endpoint: http://${ESP32_IP}/metrics"
echo "Grafana will be at: http://localhost:3000"