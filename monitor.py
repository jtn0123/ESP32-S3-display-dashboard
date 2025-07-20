#\!/usr/bin/env python3
import serial
import sys
import time

port = "/dev/cu.usbmodem101"
baudrate = 115200

print(f"Opening serial port {port} at {baudrate} baud...")
try:
    ser = serial.Serial(port, baudrate, timeout=1)
    print("Serial port opened. Monitoring output...")
    print("-" * 60)
    
    start_time = time.time()
    metrics_found = False
    
    while time.time() - start_time < 60:  # Monitor for 60 seconds
        if ser.in_waiting:
            data = ser.readline()
            try:
                line = data.decode('utf-8', errors='replace').strip()
                if line:
                    # Always print version and startup info
                    if "ESP32-S3 Dashboard" in line or "Free heap" in line:
                        print(line)
                    
                    # Look for performance metrics
                    if "[DISPLAY PERF]" in line:
                        metrics_found = True
                        print(f"\n>>> PERFORMANCE: {line}")
                    elif "[DISPLAY OPS]" in line:
                        print(f">>> OPERATIONS: {line}")
                    elif "[DISPLAY TIME]" in line:
                        print(f">>> TIMING: {line}")
                    elif "[DISPLAY EFF]" in line:
                        print(f">>> EFFICIENCY: {line}")
                    elif "[PERF]" in line:
                        print(f">>> SYSTEM: {line}")
                    elif "[CORES]" in line:
                        print(f">>> CORES: {line}")
                    elif metrics_found and "Free heap" in line:
                        print(f">>> MEMORY: {line}")
                        
            except Exception as e:
                print(f"Decode error: {e}")
            
    ser.close()
    print("-" * 60)
    print("Monitoring complete.")
    
except serial.SerialException as e:
    print(f"Error: {e}")
    sys.exit(1)
