#!/usr/bin/env python3
import serial
import sys
import time

port = "/dev/cu.usbmodem101"
baudrate = 115200

try:
    ser = serial.Serial(port, baudrate, timeout=1)
    print(f"Connected to {port} at {baudrate} baud")
    print("Press Ctrl+C to exit\n")
    
    start_time = time.time()
    while True:
        if ser.in_waiting:
            data = ser.readline()
            try:
                line = data.decode('utf-8', errors='replace').rstrip()
                if line:
                    elapsed = time.time() - start_time
                    print(f"[{elapsed:6.1f}s] {line}")
            except:
                pass
                
except serial.SerialException as e:
    print(f"Error: {e}")
except KeyboardInterrupt:
    print("\nExiting...")
finally:
    if 'ser' in locals() and ser.is_open:
        ser.close()