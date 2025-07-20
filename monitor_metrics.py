#!/usr/bin/env python3
import serial
import time
import sys

# Configure serial port
port = '/dev/cu.usbmodem101'
baudrate = 115200

# Performance metrics patterns to look for
patterns = [
    "[DISPLAY PERF]",
    "[DISPLAY OPS]", 
    "[DISPLAY TIME]",
    "[DISPLAY EFF]",
    "[PERF]",
    "[CORES]"
]

print(f"Monitoring {port} at {baudrate} baud...")
print("Looking for performance metrics...")
print("-" * 60)

try:
    with serial.Serial(port, baudrate, timeout=1) as ser:
        ser.reset_input_buffer()
        
        start_time = time.time()
        line_count = 0
        
        while time.time() - start_time < 60:  # Monitor for 60 seconds
            if ser.in_waiting:
                try:
                    line = ser.readline().decode('utf-8', errors='ignore').strip()
                    if line:
                        # Check if line contains performance metrics
                        for pattern in patterns:
                            if pattern in line:
                                print(f"[{time.strftime('%H:%M:%S')}] {line}")
                                break
                        
                        line_count += 1
                        
                        # Also print version and important startup messages
                        if "ESP32-S3 Dashboard" in line or "Free heap" in line:
                            print(f"[{time.strftime('%H:%M:%S')}] {line}")
                            
                except Exception as e:
                    print(f"Error reading line: {e}")
                    
        print("-" * 60)
        print(f"Monitored for {int(time.time() - start_time)} seconds")
        print(f"Total lines processed: {line_count}")
        
except serial.SerialException as e:
    print(f"Serial error: {e}")
except KeyboardInterrupt:
    print("\nMonitoring stopped by user")
except Exception as e:
    print(f"Unexpected error: {e}")