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
    lcd_cam_test_started = False
    test_complete = False
    
    while time.time() - start_time < 30:  # Monitor for 30 seconds
        if ser.in_waiting:
            data = ser.readline()
            try:
                line = data.decode('utf-8', errors='replace').strip()
                if line:
                    print(line)
                    
                    # Look for LCD_CAM test markers
                    if "Testing LCD_CAM with shadow register fix" in line:
                        lcd_cam_test_started = True
                        print("\n*** LCD_CAM TEST STARTED ***\n")
                    elif "Register verification after update:" in line:
                        print("\n*** REGISTER VALUES AFTER UPDATE ***")
                    elif "Test complete\!" in line:
                        test_complete = True
                        print("\n*** TEST COMPLETE ***\n")
                    elif lcd_cam_test_started and "0x" in line:
                        # Highlight register values
                        print(f">>> {line}")
                        
            except Exception as e:
                print(f"Decode error: {e}")
        
        if test_complete:
            time.sleep(2)  # Give a bit more time for final output
            break
            
    ser.close()
    print("-" * 60)
    print("Monitoring complete.")
    
except serial.SerialException as e:
    print(f"Error: {e}")
    sys.exit(1)
