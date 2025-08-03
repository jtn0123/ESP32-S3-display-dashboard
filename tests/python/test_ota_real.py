#!/usr/bin/env python3
"""Test real OTA update with the new improvements"""

import requests
import time
import threading
import sys
import os

def monitor_metrics_during_ota(base_url, stop_event, results):
    """Monitor metrics availability during OTA"""
    while not stop_event.is_set():
        try:
            start = time.time()
            resp = requests.get(f"{base_url}/metrics", timeout=2)
            duration = time.time() - start
            
            result = {
                'time': time.time(),
                'status': resp.status_code,
                'duration': duration,
                'message': 'OK' if resp.status_code == 200 else resp.text[:100]
            }
            results.append(result)
            
            # Check if we got the expected 503 during OTA
            if resp.status_code == 503 and "OTA in progress" in resp.text:
                print(f"✅ Got expected 503 during OTA: {resp.text.strip()}")
            elif resp.status_code == 200:
                print(f"   Metrics available (200) - {duration:.3f}s")
            else:
                print(f"⚠️  Unexpected status {resp.status_code}: {resp.text[:50]}")
                
        except Exception as e:
            results.append({
                'time': time.time(),
                'status': 'error',
                'duration': 0,
                'message': str(e)
            })
            print(f"   Metrics error: {e}")
            
        time.sleep(0.5)

def test_ota_upload(device_ip):
    """Test OTA upload with actual firmware binary"""
    base_url = f"http://{device_ip}"
    
    print(f"\n=== Testing OTA Upload on {device_ip} ===")
    
    # 1. Check current version
    try:
        resp = requests.get(f"{base_url}/")
        current_version = "unknown"
        if resp.status_code == 200:
            # Extract version from HTML
            import re
            match = re.search(r'v5\.\d+', resp.text)
            if match:
                current_version = match.group(0)
        print(f"Current version: {current_version}")
    except Exception as e:
        print(f"❌ Could not get current version: {e}")
        return False
    
    # 2. Check if binary exists
    binary_path = "target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard.bin"
    if not os.path.exists(binary_path):
        print(f"❌ Binary not found at {binary_path}")
        print("   Run: esptool.py --chip esp32s3 elf2image target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard")
        return False
    
    binary_size = os.path.getsize(binary_path)
    print(f"Binary size: {binary_size:,} bytes")
    
    # 3. Check memory before OTA
    try:
        resp = requests.get(f"{base_url}/api/system")
        if resp.status_code == 200:
            data = resp.json()
            heap_before = data.get('free_heap', 0)
            print(f"Free heap before OTA: {heap_before:,} bytes")
    except Exception as e:
        print(f"Could not get system info: {e}")
    
    # 4. Start metrics monitoring thread
    stop_event = threading.Event()
    metrics_results = []
    monitor_thread = threading.Thread(
        target=monitor_metrics_during_ota,
        args=(base_url, stop_event, metrics_results)
    )
    
    print("\n--- Starting OTA Upload ---")
    print("Monitoring metrics endpoint for OTA_IN_PROGRESS flag...")
    
    monitor_thread.start()
    
    # 5. Upload firmware
    try:
        print(f"\nUploading {binary_size:,} bytes to /api/ota/upload...")
        
        with open(binary_path, 'rb') as f:
            files = {'firmware': ('firmware.bin', f, 'application/octet-stream')}
            headers = {
                'X-SHA256': 'test-sha256',  # Would calculate real SHA in production
            }
            
            # Short timeout for initial connection, but allow long time for upload
            resp = requests.post(
                f"{base_url}/api/ota/upload",
                files=files,
                headers=headers,
                timeout=(5, 300)  # 5s connect, 300s read
            )
            
        print(f"\nOTA Response: {resp.status_code}")
        print(f"Response text: {resp.text[:200]}")
        
    except requests.exceptions.Timeout:
        print("⚠️  OTA request timed out (device may have rebooted)")
    except Exception as e:
        print(f"❌ OTA upload failed: {e}")
    
    # 6. Stop monitoring
    time.sleep(2)  # Give a bit more time to catch any final metrics
    stop_event.set()
    monitor_thread.join()
    
    # 7. Analyze results
    print("\n--- Metrics Monitoring Results ---")
    ota_in_progress_count = sum(1 for r in metrics_results if r['status'] == 503)
    metrics_available_count = sum(1 for r in metrics_results if r['status'] == 200)
    error_count = sum(1 for r in metrics_results if r['status'] == 'error')
    
    print(f"Total checks: {len(metrics_results)}")
    print(f"  - Metrics available (200): {metrics_available_count}")
    print(f"  - OTA in progress (503): {ota_in_progress_count}")
    print(f"  - Errors: {error_count}")
    
    if ota_in_progress_count > 0:
        print("✅ OTA_IN_PROGRESS flag worked correctly!")
    else:
        print("⚠️  Did not see OTA_IN_PROGRESS flag (might have been too quick)")
    
    # 8. Wait for device to come back
    print("\n--- Waiting for device to reboot ---")
    for i in range(30):
        try:
            resp = requests.get(f"{base_url}/health", timeout=2)
            if resp.status_code == 200:
                print(f"✅ Device back online after {i+1} seconds")
                break
        except:
            pass
        time.sleep(1)
        if i % 5 == 4:
            print(f"   Still waiting... ({i+1}s)")
    else:
        print("❌ Device did not come back online after 30 seconds")
        return False
    
    # 9. Check new version
    try:
        resp = requests.get(f"{base_url}/")
        if resp.status_code == 200:
            import re
            match = re.search(r'v5\.\d+', resp.text)
            if match:
                new_version = match.group(0)
                print(f"\nVersion after OTA: {new_version}")
                if new_version != current_version:
                    print("✅ Version changed - OTA successful!")
                else:
                    print("⚠️  Version unchanged - OTA may have failed")
    except Exception as e:
        print(f"Could not check new version: {e}")
    
    return True

if __name__ == "__main__":
    # First create the binary if it doesn't exist
    if not os.path.exists("target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard.bin"):
        print("Creating binary from ELF...")
        os.system("esptool.py --chip esp32s3 elf2image target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard")
    
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    test_ota_upload(device_ip)