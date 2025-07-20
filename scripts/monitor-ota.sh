#!/bin/bash

# Wrapper script for telnet monitoring
# This provides a consistent naming with the OTA tools

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Forward all arguments to the telnet monitor
exec "$SCRIPT_DIR/monitor-telnet.sh" "$@"