#!/bin/bash

echo "🛑 Stopping ESP32 Monitoring Stack..."
echo "===================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Stop Prometheus
echo -e "\n${YELLOW}Stopping Prometheus...${NC}"
if pgrep -x "prometheus" > /dev/null; then
    pkill prometheus
    echo -e "${GREEN}✅ Prometheus stopped${NC}"
else
    echo "⚠️  Prometheus was not running"
fi

# Stop Grafana
echo -e "\n${YELLOW}Stopping Grafana...${NC}"
if brew services list | grep grafana | grep -q started; then
    brew services stop grafana
    echo -e "${GREEN}✅ Grafana stopped${NC}"
else
    echo "⚠️  Grafana was not running"
fi

echo -e "\n${GREEN}✅ Monitoring stack stopped${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Note: ESP32 will continue running normally"
echo "To restart monitoring: ./start-monitoring.sh"