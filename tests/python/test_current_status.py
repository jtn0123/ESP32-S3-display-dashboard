#!/usr/bin/env python3
"""Test current device status and capabilities"""

import requests
import concurrent.futures
import time
import sys

def test_endpoint(base_url, endpoint, method="GET", data=None, expected_status=None):
    """Test a single endpoint"""
    try:
        start = time.time()
        if method == "GET":
            resp = requests.get(f"{base_url}{endpoint}", timeout=5)
        else:
            resp = requests.post(f"{base_url}{endpoint}", data=data, timeout=5)
        duration = time.time() - start
        
        status_icon = "✅" if resp.status_code == 200 else "⚠️" if resp.status_code < 500 else "❌"
        
        if expected_status and resp.status_code != expected_status:
            status_icon = "❌"
            
        print(f"{status_icon} {endpoint:30} - {resp.status_code} ({duration:.3f}s)")
        
        return resp.status_code == 200 or (expected_status and resp.status_code == expected_status)
    except Exception as e:
        print(f"❌ {endpoint:30} - Error: {str(e)[:50]}")
        return False

def main(device_ip):
    base_url = f"http://{device_ip}"
    
    print(f"\nESP32-S3 Dashboard Status Check")
    print(f"Device: {device_ip}")
    print("=" * 60)
    
    # Check version
    try:
        resp = requests.get(f"{base_url}/", timeout=5)
        if resp.status_code == 200:
            import re
            match = re.search(r'v5\.\d+', resp.text)
            version = match.group(0) if match else "unknown"
            print(f"Version: {version}")
    except:
        print("Could not determine version")
    
    print("\n--- Core Endpoints ---")
    core_pass = 0
    core_total = 0
    
    endpoints = [
        ("/", "Home page"),
        ("/health", "Health check"),
        ("/api/system", "System info"),
        ("/api/metrics", "Metrics JSON"),
        ("/metrics", "Prometheus metrics"),
        ("/api/config", "Configuration"),
    ]
    
    for endpoint, desc in endpoints:
        core_total += 1
        if test_endpoint(base_url, endpoint):
            core_pass += 1
    
    print("\n--- OTA Endpoints ---")
    ota_pass = 0
    ota_total = 0
    
    ota_endpoints = [
        ("/ota", "OTA web interface"),
        ("/api/ota/status", "OTA status"),
    ]
    
    for endpoint, desc in ota_endpoints:
        ota_total += 1
        if test_endpoint(base_url, endpoint):
            ota_pass += 1
    
    # Test POST endpoints with expected failures
    print("\n--- POST Endpoints (Expected Failures) ---")
    test_endpoint(base_url, "/api/config", "POST", {"test": "data"}, expected_status=500)
    test_endpoint(base_url, "/ota/update", "POST", {}, expected_status=401)  # No password
    
    print("\n--- Performance Tests ---")
    
    # Sequential requests
    print("\nSequential requests (10x):")
    success = 0
    total_time = 0
    for i in range(10):
        try:
            start = time.time()
            resp = requests.get(f"{base_url}/health", timeout=2)
            duration = time.time() - start
            total_time += duration
            if resp.status_code == 200:
                success += 1
                print(f"  [{i+1}] ✅ {duration:.3f}s")
            else:
                print(f"  [{i+1}] ❌ Status {resp.status_code}")
        except Exception as e:
            print(f"  [{i+1}] ❌ {str(e)[:30]}")
    
    print(f"\nSequential: {success}/10 successful, avg {total_time/10:.3f}s")
    
    # Concurrent requests
    print("\nConcurrent requests (5x):")
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
        futures = []
        for i in range(5):
            future = executor.submit(requests.get, f"{base_url}/health", timeout=2)
            futures.append(future)
        
        concurrent_success = 0
        for i, future in enumerate(futures):
            try:
                resp = future.result()
                if resp.status_code == 200:
                    print(f"  [{i+1}] ✅")
                    concurrent_success += 1
                else:
                    print(f"  [{i+1}] ❌ Status {resp.status_code}")
            except Exception as e:
                print(f"  [{i+1}] ❌ {str(e)[:30]}")
    
    print(f"\nConcurrent: {concurrent_success}/5 successful")
    
    # Memory check
    print("\n--- Memory Status ---")
    try:
        resp = requests.get(f"{base_url}/api/system", timeout=5)
        if resp.status_code == 200:
            data = resp.json()
            heap = data.get('free_heap', 0)
            uptime = data.get('uptime_ms', 0) / 1000
            print(f"Free heap: {heap:,} bytes ({heap/1024/1024:.1f} MB)")
            print(f"Uptime: {uptime:.0f} seconds")
            
        # Check internal DRAM from home page
        resp = requests.get(f"{base_url}/", timeout=5)
        if "Internal DRAM" in resp.text:
            print("✅ Streaming home page working (shows Internal DRAM)")
    except Exception as e:
        print(f"❌ Could not get memory status: {e}")
    
    # Summary
    print("\n" + "=" * 60)
    print("📊 SUMMARY")
    print("=" * 60)
    print(f"Core endpoints: {core_pass}/{core_total} working")
    print(f"OTA endpoints: {ota_pass}/{ota_total} working")
    print(f"Sequential performance: {success}/10 successful")
    print(f"Concurrent performance: {concurrent_success}/5 successful")
    
    overall = (core_pass + ota_pass + success/10 + concurrent_success/5) / (core_total + ota_total + 2)
    print(f"\nOverall health: {overall*100:.0f}%")
    
    if overall > 0.9:
        print("✅ Device is healthy and stable")
    elif overall > 0.7:
        print("⚠️  Device has minor issues")
    else:
        print("❌ Device has significant issues")

if __name__ == "__main__":
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    main(device_ip)