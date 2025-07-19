#!/bin/bash
# GCC wrapper script for ESP32-S3 builds
# This handles the ARM64 host to Xtensa target cross-compilation

# Find the xtensa-esp32s3-elf-gcc in the PATH
XTENSA_GCC=$(which xtensa-esp32s3-elf-gcc 2>/dev/null)

if [ -z "$XTENSA_GCC" ]; then
    # Try common ESP-IDF installation paths
    if [ -f "$HOME/.espressif/tools/xtensa-esp-elf/esp-13.2.0_20240530/xtensa-esp-elf/bin/xtensa-esp32s3-elf-gcc" ]; then
        XTENSA_GCC="$HOME/.espressif/tools/xtensa-esp-elf/esp-13.2.0_20240530/xtensa-esp-elf/bin/xtensa-esp32s3-elf-gcc"
    elif [ -f "$HOME/.espressif/tools/xtensa-esp32s3-elf/esp-2022r1-11.2.0/xtensa-esp32s3-elf/bin/xtensa-esp32s3-elf-gcc" ]; then
        XTENSA_GCC="$HOME/.espressif/tools/xtensa-esp32s3-elf/esp-2022r1-11.2.0/xtensa-esp32s3-elf/bin/xtensa-esp32s3-elf-gcc"
    else
        echo "Error: xtensa-esp32s3-elf-gcc not found in PATH or common locations" >&2
        exit 1
    fi
fi

# Pass all arguments to the actual compiler
exec "$XTENSA_GCC" "$@"