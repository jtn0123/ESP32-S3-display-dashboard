#!/bin/bash

# ESP32-S3 Dashboard - Telnet Monitor
# Connects to the device's telnet server for remote log monitoring

set -e

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
DEFAULT_PORT=23
DEVICE_MDNS="esp32-dashboard.local"
RETRY_DELAY=2
MAX_RETRIES=10

# Parse command line arguments
DEVICE_HOST=""
USE_IP=false
SAVE_LOG=false
LOG_FILE=""

print_usage() {
    echo -e "${GREEN}${BOLD}ESP32-S3 Dashboard - Telnet Monitor${NC}"
    echo -e "===================================="
    echo ""
    echo "Usage: $0 [options] [ip-address]"
    echo ""
    echo "Options:"
    echo "  -h, --help       Show this help message"
    echo "  -s, --save       Save output to log file"
    echo "  -f, --file FILE  Specify log file (default: esp32-telnet-<timestamp>.log)"
    echo "  -r, --retry      Keep retrying connection if it fails"
    echo ""
    echo "Examples:"
    echo "  $0                    # Connect using mDNS (esp32-dashboard.local)"
    echo "  $0 192.168.1.100      # Connect to specific IP"
    echo "  $0 -s                 # Connect and save to log file"
    echo "  $0 -s -f debug.log    # Save to specific file"
    echo "  $0 -r 192.168.1.100   # Retry connection on failure"
    echo ""
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            print_usage
            exit 0
            ;;
        -s|--save)
            SAVE_LOG=true
            shift
            ;;
        -f|--file)
            SAVE_LOG=true
            LOG_FILE="$2"
            shift 2
            ;;
        -r|--retry)
            MAX_RETRIES=9999
            shift
            ;;
        -*)
            echo -e "${RED}Unknown option: $1${NC}"
            print_usage
            exit 1
            ;;
        *)
            DEVICE_HOST="$1"
            USE_IP=true
            shift
            ;;
    esac
done

# Set default values
if [ -z "$DEVICE_HOST" ]; then
    DEVICE_HOST="$DEVICE_MDNS"
fi

if [ "$SAVE_LOG" = true ] && [ -z "$LOG_FILE" ]; then
    LOG_FILE="esp32-telnet-$(date +%Y%m%d-%H%M%S).log"
fi

# Function to check if telnet is available
check_telnet() {
    if ! command -v telnet &> /dev/null; then
        echo -e "${RED}Error: telnet command not found${NC}"
        echo ""
        echo "Please install telnet:"
        echo "  macOS:    brew install telnet"
        echo "  Ubuntu:   sudo apt-get install telnet"
        echo "  CentOS:   sudo yum install telnet"
        echo ""
        echo "Alternative: You can use netcat instead:"
        echo "  nc $DEVICE_HOST $DEFAULT_PORT"
        exit 1
    fi
}

# Function to test connection
test_connection() {
    local host=$1
    echo -e "${BLUE}Testing connection to $host:$DEFAULT_PORT...${NC}"
    
    # Use timeout with nc to test connection
    if command -v nc &> /dev/null; then
        if nc -z -w 2 "$host" "$DEFAULT_PORT" 2>/dev/null; then
            return 0
        fi
    else
        # Fallback to telnet with timeout
        if timeout 2 telnet "$host" "$DEFAULT_PORT" 2>&1 | grep -q "Connected"; then
            return 0
        fi
    fi
    
    return 1
}

# Function to resolve mDNS
resolve_mdns() {
    local mdns_name=$1
    
    echo -e "${BLUE}Resolving $mdns_name...${NC}"
    
    # Try different methods to resolve mDNS
    if command -v avahi-resolve &> /dev/null; then
        # Linux with avahi
        IP=$(avahi-resolve -4 -n "$mdns_name" 2>/dev/null | awk '{print $2}')
    elif command -v dscacheutil &> /dev/null; then
        # macOS
        IP=$(dscacheutil -q host -a name "$mdns_name" 2>/dev/null | grep "ip_address" | awk '{print $2}')
    elif command -v dig &> /dev/null; then
        # Using dig with mDNS
        IP=$(dig +short "$mdns_name" @224.0.0.251 -p 5353 2>/dev/null | head -1)
    fi
    
    if [ -n "$IP" ]; then
        echo -e "${GREEN}Resolved to: $IP${NC}"
        echo "$IP"
        return 0
    else
        return 1
    fi
}

# Function to find device IP using ARP scan
find_device_ip() {
    echo -e "${YELLOW}Searching for ESP32 devices on network...${NC}"
    
    # Get list of ESP32 MAC prefixes
    local esp_macs="b4:3a:45"  # Your device's MAC prefix
    
    # Try arp-scan if available
    if command -v arp-scan &> /dev/null && [ "$EUID" -eq 0 ]; then
        echo "Using arp-scan..."
        IP=$(sudo arp-scan --local 2>/dev/null | grep -i "$esp_macs" | awk '{print $1}' | head -1)
        if [ -n "$IP" ]; then
            echo -e "${GREEN}Found ESP32 at: $IP${NC}"
            return 0
        fi
    fi
    
    # Try arp command
    if command -v arp &> /dev/null; then
        echo "Checking ARP cache..."
        IP=$(arp -a | grep -i "$esp_macs" | sed -n 's/.*(\([0-9.]*\)).*/\1/p' | head -1)
        if [ -n "$IP" ]; then
            echo -e "${GREEN}Found ESP32 in ARP cache: $IP${NC}"
            return 0
        fi
    fi
    
    return 1
}

# Function to connect with telnet
connect_telnet() {
    local host=$1
    
    echo -e "${GREEN}${BOLD}Connecting to $host:$DEFAULT_PORT${NC}"
    echo -e "${CYAN}Press Ctrl+] then 'quit' to exit${NC}"
    echo ""
    
    if [ "$SAVE_LOG" = true ]; then
        echo -e "${YELLOW}Saving output to: $LOG_FILE${NC}"
        echo ""
        
        # Use script command to capture output
        if command -v script &> /dev/null; then
            # macOS and Linux have different script syntax
            if [[ "$OSTYPE" == "darwin"* ]]; then
                script -q "$LOG_FILE" telnet "$host" "$DEFAULT_PORT"
            else
                script -q -c "telnet $host $DEFAULT_PORT" "$LOG_FILE"
            fi
        else
            # Fallback: use tee
            telnet "$host" "$DEFAULT_PORT" | tee "$LOG_FILE"
        fi
    else
        telnet "$host" "$DEFAULT_PORT"
    fi
}

# Main script
echo -e "${GREEN}${BOLD}ESP32-S3 Dashboard - Telnet Monitor${NC}"
echo -e "===================================="
echo ""

# Check for telnet
check_telnet

# Connection loop
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    # Determine the host to connect to
    CONNECT_HOST="$DEVICE_HOST"
    
    # If using mDNS, try to resolve it
    if [ "$USE_IP" = false ]; then
        if RESOLVED_IP=$(resolve_mdns "$DEVICE_HOST"); then
            CONNECT_HOST="$RESOLVED_IP"
        else
            echo -e "${YELLOW}Warning: Could not resolve $DEVICE_HOST${NC}"
            
            # Try to find device by MAC
            if find_device_ip; then
                CONNECT_HOST="$IP"
            else
                echo -e "${YELLOW}Could not find device on network${NC}"
            fi
        fi
    fi
    
    # Test connection
    if test_connection "$CONNECT_HOST"; then
        echo -e "${GREEN}Connection test successful!${NC}"
        echo ""
        
        # Connect
        connect_telnet "$CONNECT_HOST"
        
        # If we get here, telnet exited
        echo ""
        echo -e "${YELLOW}Connection closed${NC}"
        
        if [ $MAX_RETRIES -gt 10 ]; then
            echo -e "${BLUE}Retrying in $RETRY_DELAY seconds...${NC}"
            sleep $RETRY_DELAY
            RETRY_COUNT=$((RETRY_COUNT + 1))
            continue
        else
            break
        fi
    else
        echo -e "${RED}Failed to connect to $CONNECT_HOST:$DEFAULT_PORT${NC}"
        
        if [ $RETRY_COUNT -lt $((MAX_RETRIES - 1)) ]; then
            echo -e "${BLUE}Retrying in $RETRY_DELAY seconds... (attempt $((RETRY_COUNT + 2))/$MAX_RETRIES)${NC}"
            sleep $RETRY_DELAY
            RETRY_COUNT=$((RETRY_COUNT + 1))
        else
            echo -e "${RED}Maximum retries reached${NC}"
            echo ""
            echo "Troubleshooting:"
            echo "  1. Check if device is powered on"
            echo "  2. Verify WiFi connection on Network screen"
            echo "  3. Check IP address on device display"
            echo "  4. Ensure you're on the same network"
            echo "  5. Try: ping $DEVICE_HOST"
            exit 1
        fi
    fi
done

if [ "$SAVE_LOG" = true ]; then
    echo ""
    echo -e "${GREEN}Log saved to: $LOG_FILE${NC}"
fi

echo -e "${GREEN}Done!${NC}"