#!/bin/bash

echo "🚀 Starting ESP32 Monitoring Stack..."
echo "===================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if MCP servers directory exists
if [ ! -d "$HOME/mcp-servers" ]; then
    echo -e "${RED}❌ MCP servers not found at ~/mcp-servers${NC}"
    echo "Run ./setup-mcp-servers.sh first!"
    exit 1
fi

# Find ESP32 IP
echo -e "\n${YELLOW}🔍 Looking for ESP32...${NC}"
ESP32_IP=$(arp -a | grep "b4:3a:45" | grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' | head -1)

if [ -z "$ESP32_IP" ]; then
    echo -e "${RED}❌ ESP32 not found on network${NC}"
    echo "Make sure ESP32 is powered on and connected to WiFi"
    exit 1
else
    echo -e "${GREEN}✅ Found ESP32 at: $ESP32_IP${NC}"
fi

# Test ESP32 metrics
echo -e "\n${YELLOW}📊 Testing ESP32 metrics endpoint...${NC}"
if curl -s "http://$ESP32_IP/metrics" | grep -q "esp32_fps_actual"; then
    echo -e "${GREEN}✅ ESP32 metrics working${NC}"
else
    echo -e "${RED}❌ ESP32 metrics not responding${NC}"
    exit 1
fi

# Start Prometheus
echo -e "\n${YELLOW}📈 Starting Prometheus...${NC}"
if pgrep -x "prometheus" > /dev/null; then
    echo -e "${YELLOW}⚠️  Prometheus already running${NC}"
else
    cd ~/mcp-servers
    nohup prometheus --config.file=prometheus.yml > prometheus.log 2>&1 &
    sleep 2
    if pgrep -x "prometheus" > /dev/null; then
        echo -e "${GREEN}✅ Prometheus started${NC}"
    else
        echo -e "${RED}❌ Failed to start Prometheus${NC}"
        exit 1
    fi
fi

# Start Grafana
echo -e "\n${YELLOW}📊 Starting Grafana...${NC}"
if brew services list | grep grafana | grep -q started; then
    echo -e "${YELLOW}⚠️  Grafana already running${NC}"
else
    brew services start grafana
    echo -e "${GREEN}✅ Grafana started${NC}"
fi

# Wait for services to be ready
echo -e "\n${YELLOW}⏳ Waiting for services to initialize...${NC}"
sleep 5

# Final status check
echo -e "\n${GREEN}🎉 Monitoring Stack Status:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check Prometheus
if curl -s http://localhost:9090/api/v1/targets | grep -q "\"health\":\"up\""; then
    echo -e "Prometheus:     ${GREEN}✅ Running${NC} - http://localhost:9090"
else
    echo -e "Prometheus:     ${RED}❌ Not responding${NC}"
fi

# Check Grafana
if curl -s http://localhost:3000/api/health | grep -q "ok"; then
    echo -e "Grafana:        ${GREEN}✅ Running${NC} - http://localhost:3000"
    echo -e "                   Login: admin/admin"
else
    echo -e "Grafana:        ${RED}❌ Not responding${NC}"
fi

# Show dashboard link
echo -e "\nESP32 Dashboard: ${GREEN}http://localhost:3000/d/esp32-dashboard/${NC}"
echo -e "ESP32 Metrics:   ${GREEN}http://$ESP32_IP/metrics${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Show current metrics
echo -e "\n${YELLOW}📊 Current ESP32 Status:${NC}"
FPS=$(curl -s 'http://localhost:9090/api/v1/query?query=esp32_fps_actual' | grep -o '"value":\[[0-9.]*,"[0-9.]*"' | cut -d'"' -f3)
CPU=$(curl -s 'http://localhost:9090/api/v1/query?query=esp32_cpu_usage_percent' | grep -o '"value":\[[0-9.]*,"[0-9.]*"' | cut -d'"' -f3)
TEMP=$(curl -s 'http://localhost:9090/api/v1/query?query=esp32_temperature_celsius' | grep -o '"value":\[[0-9.]*,"[0-9.]*"' | cut -d'"' -f3)

echo "FPS:  ${FPS:-Loading...}"
echo "CPU:  ${CPU:-Loading...}%"
echo "Temp: ${TEMP:-Loading...}°C"

echo -e "\n${GREEN}✨ Monitoring ready!${NC}"