#!/bin/bash
# Log filtering wrapper for monitor-telnet.py
# Makes it easy to filter for specific log patterns

set -e

# Default filter pattern (empty = show all)
FILTER=""
EXCLUDE=""
COLOR=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--filter)
            FILTER="$2"
            shift 2
            ;;
        -e|--exclude)
            EXCLUDE="$2"
            shift 2
            ;;
        --no-color)
            COLOR=false
            shift
            ;;
        --help)
            echo "ESP32-S3 Dashboard - Log Filter"
            echo "=============================="
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  -f, --filter PATTERN   Show only lines matching PATTERN"
            echo "  -e, --exclude PATTERN  Hide lines matching PATTERN"
            echo "  --no-color            Disable colored output"
            echo "  --help               Show this help message"
            echo ""
            echo "Common filter patterns:"
            echo "  -f 'WIFI|NETWORK'    Show WiFi/network logs"
            echo "  -f 'ERROR|WARN'      Show errors and warnings"
            echo "  -f 'PERF|FPS'        Show performance metrics"
            echo "  -f 'SENSOR|TEMP'     Show sensor data"
            echo "  -f 'OTA|UPDATE'      Show OTA update logs"
            echo "  -f 'BUTTON|UI'       Show UI interactions"
            echo ""
            echo "Examples:"
            echo "  $0 -f 'ERROR'                     # Show only errors"
            echo "  $0 -f 'WIFI' -e 'RSSI'          # Show WiFi logs but not RSSI"
            echo "  $0 -f 'FPS|frame'               # Show FPS and frame timing"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build the filter command
if [ -n "$FILTER" ] && [ -n "$EXCLUDE" ]; then
    # Both filter and exclude
    ./scripts/monitor-telnet.py | grep -E "$FILTER" | grep -vE "$EXCLUDE"
elif [ -n "$FILTER" ]; then
    # Only filter
    ./scripts/monitor-telnet.py | grep -E "$FILTER"
elif [ -n "$EXCLUDE" ]; then
    # Only exclude
    ./scripts/monitor-telnet.py | grep -vE "$EXCLUDE"
else
    # No filters
    ./scripts/monitor-telnet.py
fi