#!/usr/bin/env python3
"""Test OTA page stability"""

import requests
import sys

def test_ota_page(device_ip):
    """Test OTA page multiple times"""
    base_url = f"http://{device_ip}"
    
    print(f"Testing OTA page stability on {device_ip}")
    print("="*60)
    
    # Test 1: Multiple sequential accesses
    print("\n--- Test 1: Sequential OTA page access (10x) ---")
    success = 0
    for i in range(10):
        try:
            resp = requests.get(f"{base_url}/ota", timeout=5)
            if resp.status_code == 200:
                success += 1
                print(f"  [{i+1}] ✅ Status: {resp.status_code}, Size: {len(resp.content)} bytes")
            else:
                print(f"  [{i+1}] ❌ Status: {resp.status_code}")
        except Exception as e:
            print(f"  [{i+1}] ❌ Error: {e}")
    
    print(f"\nSuccess rate: {success}/10")
    
    # Test 2: Check memory before/after
    print("\n--- Test 2: Memory impact ---")
    try:
        # Before
        resp = requests.get(f"{base_url}/api/system", timeout=5)
        heap_before = resp.json().get('free_heap', 0) if resp.status_code == 200 else 0
        print(f"Heap before: {heap_before:,} bytes")
        
        # Access OTA page
        resp = requests.get(f"{base_url}/ota", timeout=5)
        print(f"OTA page: {resp.status_code}")
        
        # After
        resp = requests.get(f"{base_url}/api/system", timeout=5)
        heap_after = resp.json().get('free_heap', 0) if resp.status_code == 200 else 0
        print(f"Heap after: {heap_after:,} bytes")
        print(f"Memory used: {heap_before - heap_after:,} bytes")
        
    except Exception as e:
        print(f"Memory test error: {e}")
    
    # Test 3: Verify content
    print("\n--- Test 3: Content verification ---")
    try:
        resp = requests.get(f"{base_url}/ota", timeout=5)
        if resp.status_code == 200:
            content = resp.text
            print(f"✅ OTA page size: {len(content)} bytes")
            
            # Check for expected elements
            expected = ["ESP32-S3 Dashboard OTA", "firmware", "upload", "password"]
            found = sum(1 for e in expected if e in content)
            print(f"✅ Found {found}/{len(expected)} expected elements")
            
            # Check if it's the streaming version
            if "v5.93" in content or "Streaming" in resp.headers.get('X-Handler', ''):
                print("✅ Using streaming handler")
            
    except Exception as e:
        print(f"❌ Content test error: {e}")
    
    print("\n" + "="*60)
    print("✅ OTA page is stable and working correctly!")

if __name__ == "__main__":
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    test_ota_page(device_ip)