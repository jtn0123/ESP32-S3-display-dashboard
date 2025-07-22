#\!/usr/bin/env python3

import serial
import sys
import time

def main():
    port = '/dev/cu.usbmodem101'
    baudrate = 115200
    
    try:
        ser = serial.Serial(port, baudrate, timeout=1)
        print(f"Connected to {port} at {baudrate} baud")
        print("Press Ctrl+C to exit\n")
        
        # Open log file
        with open('lcd_enhanced_debug.log', 'w') as logfile:
            while True:
                if ser.in_waiting > 0:
                    data = ser.readline()
                    try:
                        line = data.decode('utf-8', errors='ignore').strip()
                        if line:
                            print(line)
                            logfile.write(line + '\n')
                            logfile.flush()
                    except:
                        pass
                        
    except serial.SerialException as e:
        print(f"Error: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nExiting...")
        ser.close()

if __name__ == "__main__":
    main()
