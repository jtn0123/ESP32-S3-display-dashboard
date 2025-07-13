#!/usr/bin/env python3
"""
ESP32-S3 OTA Upload Script
Works around ArduinoOTA issues with ESP32-S3 native USB
"""

import sys
import os
import time
import socket
import hashlib
import random

def upload_ota(host, port, filename):
    """Upload firmware via OTA"""
    # Read the binary file
    with open(filename, 'rb') as f:
        content = f.read()
    
    file_size = len(content)
    file_md5 = hashlib.md5(content).hexdigest()
    
    print(f"Uploading {filename} ({file_size} bytes) to {host}:{port}")
    
    # Create socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(10)
    
    try:
        # Connect to ESP32
        print(f"Connecting to {host}:{port}...")
        sock.connect((host, port))
        print("Connected!")
        
        # Send invitation with file size and MD5
        invitation = f"0 {port} {file_size} {file_md5}\n"
        sock.send(invitation.encode())
        
        # Wait for acceptance
        response = sock.recv(128).decode().strip()
        if "OK" not in response:
            print(f"Device rejected update: {response}")
            return False
        
        print("Device accepted, uploading...")
        
        # Send file in chunks
        chunk_size = 1024
        sent = 0
        
        while sent < file_size:
            chunk = content[sent:sent + chunk_size]
            sock.send(chunk)
            sent += len(chunk)
            
            # Progress
            progress = int(sent * 100 / file_size)
            print(f"\rProgress: {progress}%", end='', flush=True)
        
        print("\nUpload complete!")
        return True
        
    except socket.timeout:
        print("Connection timed out")
        return False
    except Exception as e:
        print(f"Error: {e}")
        return False
    finally:
        sock.close()

if __name__ == "__main__":
    # Default values
    host = "10.27.27.201"
    port = 3232
    
    # Check for binary file
    binary = "dashboard/build/esp32.esp32.lilygo_t_display_s3/dashboard.ino.bin"
    
    if not os.path.exists(binary):
        print(f"Binary not found: {binary}")
        print("Run: arduino-cli compile --fqbn esp32:esp32:lilygo_t_display_s3 --export-binaries dashboard")
        sys.exit(1)
    
    # Upload
    if upload_ota(host, port, binary):
        print("OTA upload successful!")
    else:
        print("OTA upload failed")
        print("\nTroubleshooting:")
        print("1. Check device is connected to WiFi")
        print("2. Verify IP address on WiFi Status screen")
        print("3. Try power cycling the device")