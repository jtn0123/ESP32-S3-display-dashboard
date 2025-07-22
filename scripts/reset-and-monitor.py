#!/usr/bin/env python3
import subprocess
import time
import os

port = '/dev/cu.usbmodem101'

print("Resetting ESP32-S3 and monitoring output...")

# First, reset using esptool
print("Resetting device...")
subprocess.run([
    '.embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py',
    '--chip', 'esp32s3',
    '--port', port,
    'run'
], capture_output=True)

# Give it a moment to reset
time.sleep(0.5)

# Now monitor with our trace script
print("Starting monitor...")
os.system('./scripts/monitor-trace.sh')