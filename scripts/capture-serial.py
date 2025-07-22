#!/usr/bin/env python3
import serial
import sys
import time

port = '/dev/cu.usbmodem101'
baudrate = 115200

print(f"Opening serial port {port} at {baudrate} baud...")
ser = serial.Serial(port, baudrate, timeout=1)

# Reset the device by toggling RTS
ser.rts = False
time.sleep(0.1)
ser.rts = True
time.sleep(0.1)

print("Capturing output for 20 seconds...")
print("=" * 50)

start_time = time.time()
lcd_messages = []

while time.time() - start_time < 20:
    if ser.in_waiting:
        try:
            line = ser.readline().decode('utf-8', errors='ignore').strip()
            if line:
                # Highlight LCD-related messages
                if any(keyword in line for keyword in ['ESP_LCD', 'LCD', 'display', 'power', 'backlight', 'init', 'Power pins', 'gap', 'ST7789', 'panel', 'I80']):
                    print(f"\033[1;32m{line}\033[0m")
                    lcd_messages.append(line)
                elif 'error' in line.lower() or 'fail' in line.lower():
                    print(f"\033[1;31m{line}\033[0m")
                    lcd_messages.append(line)
                else:
                    print(line)
        except Exception as e:
            pass

ser.close()

print("\n" + "=" * 50)
print("LCD-related messages captured:")
for msg in lcd_messages[-30:]:  # Last 30 LCD messages
    print(msg)