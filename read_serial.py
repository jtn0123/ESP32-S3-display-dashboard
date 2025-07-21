#!/usr/bin/env python3
import sys
import time
import os
import termios
import tty

port = '/dev/cu.usbmodem101'
print(f"Reading from {port} at 115200 baud...")
print("=" * 60)

# Open in non-blocking mode
fd = os.open(port, os.O_RDONLY | os.O_NONBLOCK)

# Save original settings
old_settings = termios.tcgetattr(fd)

try:
    # Configure serial port
    tty.setraw(fd)
    new_settings = termios.tcgetattr(fd)
    new_settings[4] = termios.B115200  # input speed
    new_settings[5] = termios.B115200  # output speed
    termios.tcsetattr(fd, termios.TCSANOW, new_settings)
    
    start_time = time.time()
    data_buffer = b''
    
    while time.time() - start_time < 10:  # Read for 10 seconds
        try:
            chunk = os.read(fd, 1024)
            if chunk:
                data_buffer += chunk
                # Print as we receive
                try:
                    text = chunk.decode('utf-8', errors='replace')
                    print(text, end='', flush=True)
                except:
                    pass
        except BlockingIOError:
            time.sleep(0.01)  # No data available, wait a bit
            
except Exception as e:
    print(f"\nError: {e}")
finally:
    # Restore settings
    termios.tcsetattr(fd, termios.TCSANOW, old_settings)
    os.close(fd)
    
print("\n" + "=" * 60)
print("Read complete")