#!/usr/bin/env python3
"""Wait for device to recover and monitor when it comes back online"""

import requests
import time
import sys
from datetime import datetime

ESP32_IP = "10.27.27.201"
BASE_URL = f"http://{ESP32_IP}"

def check_device():
    """Check if device is responding"""
    try:
        response = requests.get(f"{BASE_URL}/health", timeout=2)
        return response.status_code == 200
    except:
        return False

def main():
    print(f"Waiting for ESP32 at {ESP32_IP} to recover...")
    print(f"Started at: {datetime.now().strftime('%H:%M:%S')}")
    print("Press Ctrl+C to stop\n")
    
    down_since = datetime.now()
    check_count = 0
    
    while True:
        check_count += 1
        if check_device():
            up_time = datetime.now()
            down_duration = (up_time - down_since).total_seconds()
            
            print(f"\n✅ Device is UP!")
            print(f"Time: {up_time.strftime('%H:%M:%S')}")
            print(f"Was down for: {down_duration:.1f} seconds")
            print(f"Checked {check_count} times")
            
            # Do a few more checks to ensure it's stable
            print("\nVerifying stability...")
            stable = True
            for i in range(5):
                time.sleep(1)
                if not check_device():
                    print(f"  Check {i+1}: ❌ Failed")
                    stable = False
                    break
                else:
                    print(f"  Check {i+1}: ✅ OK")
            
            if stable:
                print("\n✅ Device appears stable!")
                sys.exit(0)
            else:
                print("\n⚠️  Device is unstable, continuing to monitor...")
                down_since = datetime.now()
        else:
            # Show progress
            elapsed = (datetime.now() - down_since).total_seconds()
            print(f"\r⏳ Down for {elapsed:.0f}s (checked {check_count} times)", end="", flush=True)
        
        time.sleep(2)

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nMonitoring stopped by user")
        sys.exit(0)