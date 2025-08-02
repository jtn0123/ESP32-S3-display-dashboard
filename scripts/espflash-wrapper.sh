#!/bin/bash
# Wrapper script for espflash that automatically includes --flash-size 16mb
# This prevents the common mistake of forgetting the flash size parameter

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if espflash is available
if ! command -v espflash &> /dev/null; then
    echo -e "${RED}Error: espflash not found!${NC}"
    echo "Please install espflash v3.3.0:"
    echo "  cargo install espflash@3.3.0 --force"
    exit 1
fi

# Check if first argument is "flash"
if [ "$1" = "flash" ]; then
    # Check if --flash-size is already specified
    if [[ " $@ " =~ " --flash-size " ]]; then
        # User already specified flash size, use their command as-is
        echo -e "${YELLOW}Using specified flash size${NC}"
        espflash "$@"
    else
        # Insert --flash-size 16mb after "flash" command
        echo -e "${GREEN}Auto-adding --flash-size 16mb${NC}"
        shift # Remove "flash" from arguments
        espflash flash --flash-size 16mb "$@"
    fi
else
    # Not a flash command, pass through as-is
    espflash "$@"
fi