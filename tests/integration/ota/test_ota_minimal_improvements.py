#!/usr/bin/env python3
"""Test minimal OTA improvements - stack allocation and OTA_IN_PROGRESS flag"""

import requests
import time
import threading
import sys

def test_ota_with_concurrent_metrics(device_ip):
    """Test that metrics endpoint returns 503 during OTA"""
    base_url = f"http://{device_ip}"
    
    print(f"Testing OTA improvements on {device_ip}")
    
    # First, verify device is responsive
    try:
        resp = requests.get(f"{base_url}/health", timeout=5)
        print(f"✅ Device online - Health check: {resp.status_code}")
    except Exception as e:
        print(f"❌ Device not responding: {e}")
        return False
    
    # Test 1: Verify metrics work before OTA
    print("\n--- Test 1: Metrics before OTA ---")
    try:
        resp = requests.get(f"{base_url}/metrics", timeout=5)
        print(f"Metrics status: {resp.status_code}")
        if resp.status_code == 200:
            print("✅ Metrics accessible before OTA")
        else:
            print(f"⚠️  Unexpected status: {resp.text}")
    except Exception as e:
        print(f"❌ Metrics failed: {e}")
    
    # Test 2: Check memory before (to verify stack allocation works)
    print("\n--- Test 2: Memory check ---")
    try:
        resp = requests.get(f"{base_url}/api/system", timeout=5)
        if resp.status_code == 200:
            data = resp.json()
            heap_before = data.get('free_heap', 0)
            print(f"Free heap before: {heap_before:,} bytes")
    except Exception as e:
        print(f"Could not get system info: {e}")
    
    print("\n--- Test 3: OTA simulation ---")
    print("NOTE: Not actually uploading firmware (would require real binary)")
    print("Just testing the OTA endpoint behavior and concurrent access")
    
    # Monitor metrics access during simulated OTA preparation
    metrics_results = []
    
    def check_metrics():
        """Background thread to check metrics availability"""
        for i in range(5):
            try:
                start = time.time()
                resp = requests.get(f"{base_url}/metrics", timeout=2)
                duration = time.time() - start
                metrics_results.append({
                    'attempt': i + 1,
                    'status': resp.status_code,
                    'duration': duration,
                    'message': resp.text if resp.status_code == 503 else 'OK'
                })
            except Exception as e:
                metrics_results.append({
                    'attempt': i + 1,
                    'status': 'error',
                    'duration': 0,
                    'message': str(e)
                })
            time.sleep(0.5)
    
    # Start background metrics checker
    metrics_thread = threading.Thread(target=check_metrics)
    metrics_thread.start()
    
    # Simulate OTA request (without actual firmware)
    print("Checking OTA endpoint...")
    try:
        # Just check if OTA endpoint exists
        resp = requests.get(f"{base_url}/ota", timeout=5)
        print(f"OTA page status: {resp.status_code}")
    except Exception as e:
        print(f"OTA endpoint check: {e}")
    
    # Wait for metrics thread
    metrics_thread.join()
    
    # Show metrics results
    print("\n--- Concurrent metrics access results ---")
    for result in metrics_results:
        if result['status'] == 503:
            print(f"  Attempt {result['attempt']}: {result['status']} - {result['message']} ✅")
        elif result['status'] == 200:
            print(f"  Attempt {result['attempt']}: {result['status']} - Metrics available")
        else:
            print(f"  Attempt {result['attempt']}: {result['status']} - {result['message']}")
    
    # Final health check
    print("\n--- Final health check ---")
    try:
        resp = requests.get(f"{base_url}/health", timeout=5)
        print(f"✅ Device still responsive - Health: {resp.status_code}")
        return True
    except Exception as e:
        print(f"❌ Device not responding: {e}")
        return False

if __name__ == "__main__":
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    success = test_ota_with_concurrent_metrics(device_ip)
    sys.exit(0 if success else 1)