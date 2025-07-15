#!/bin/bash
# Wrapper script to filter out ldproxy flags that cause issues on ARM64 macOS

# Filter out problematic flags but preserve the working directory
args=()
skip_next=false
cwd=""

for arg in "$@"; do
    if [[ "$arg" == "--ldproxy-linker" ]]; then
        skip_next=true
        continue
    fi
    if [[ "$arg" == "--ldproxy-cwd" ]]; then
        # Next arg is the working directory, capture it
        cwd_next=true
        continue
    fi
    if [[ "$skip_next" == true ]]; then
        skip_next=false
        continue
    fi
    if [[ "$cwd_next" == true ]]; then
        cwd="$arg"
        cwd_next=false
        continue
    fi
    args+=("$arg")
done

# Change to the build directory if specified
if [[ -n "$cwd" ]]; then
    cd "$cwd"
fi

# Call the real compiler with filtered arguments
exec xtensa-esp32s3-elf-gcc "${args[@]}"