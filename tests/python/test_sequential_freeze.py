#!/usr/bin/env python3
"""Test sequential requests to identify memory leak or resource exhaustion"""

import requests
import time
import json
from datetime import datetime

ESP32_IP = "10.27.27.201"
BASE_URL = f"http://{ESP32_IP}"

def get_system_info():
    """Get system info including memory"""
    try:
        response = requests.get(f"{BASE_URL}/api/system", timeout=2)
        if response.status_code == 200:
            return response.json()
    except:
        pass
    return None

def test_with_memory_tracking():
    """Test sequential requests while tracking memory"""
    print("Testing sequential requests with memory tracking")
    print("=" * 60)
    
    # Get initial state
    initial_info = get_system_info()
    if initial_info:
        print(f"Initial free heap: {initial_info.get('free_heap', 0):,} bytes")
        print(f"Initial largest block: {initial_info.get('largest_free_block', 0):,} bytes")
    else:
        print("Warning: Could not get initial system info")
    
    print("\nMaking sequential requests...")
    
    request_count = 0
    last_heap = initial_info.get('free_heap', 0) if initial_info else 0
    
    while True:
        request_count += 1
        
        try:
            # Alternate between different endpoints
            if request_count % 5 == 0:
                # Every 5th request, check system info
                info = get_system_info()
                if info:
                    heap = info.get('free_heap', 0)
                    heap_change = heap - last_heap if last_heap else 0
                    total_change = heap - initial_info.get('free_heap', 0) if initial_info else 0
                    
                    print(f"\n[{request_count:3d}] System check:")
                    print(f"      Free heap: {heap:,} bytes ({heap_change:+,} from last, {total_change:+,} total)")
                    print(f"      Largest block: {info.get('largest_free_block', 0):,} bytes")
                    
                    last_heap = heap
                    
                    # Check for low memory
                    if heap < 50000:  # Less than 50KB
                        print("\n⚠️  WARNING: Very low memory!")
                    
            else:
                # Regular health check
                endpoint = "/health" if request_count % 3 else "/api/metrics"
                response = requests.get(f"{BASE_URL}{endpoint}", timeout=3)
                print(f"\r[{request_count:3d}] {endpoint}: {response.status_code}", end="", flush=True)
                
        except requests.exceptions.Timeout:
            print(f"\n\n❌ TIMEOUT at request {request_count}")
            break
        except requests.exceptions.ConnectionError:
            print(f"\n\n❌ CONNECTION ERROR at request {request_count}")
            break
        except Exception as e:
            print(f"\n\n❌ ERROR at request {request_count}: {type(e).__name__}")
            break
        
        # Small delay to not overwhelm
        time.sleep(0.5)
    
    print(f"\nTotal successful requests before freeze: {request_count - 1}")
    
    # Try to get final state after recovery
    print("\nWaiting 30s for recovery...")
    time.sleep(30)
    
    final_info = get_system_info()
    if final_info:
        print("\nPost-recovery state:")
        print(f"  Free heap: {final_info.get('free_heap', 0):,} bytes")
        print(f"  Largest block: {final_info.get('largest_free_block', 0):,} bytes")
        
        if initial_info:
            heap_diff = final_info.get('free_heap', 0) - initial_info.get('free_heap', 0)
            print(f"  Total heap change: {heap_diff:+,} bytes")

def test_endpoint_specific():
    """Test if specific endpoints cause issues"""
    print("\nTesting specific endpoints for memory leaks")
    print("=" * 60)
    
    endpoints = [
        ("/health", 50),           # Light endpoint
        ("/api/metrics", 20),      # Medium data
        ("/api/system", 20),       # System info
        ("/", 10),                 # Full HTML
    ]
    
    for endpoint, count in endpoints:
        print(f"\nTesting {endpoint} ({count} requests)...")
        
        success = 0
        for i in range(count):
            try:
                response = requests.get(f"{BASE_URL}{endpoint}", timeout=3)
                if response.status_code == 200:
                    success += 1
                    size = len(response.content)
                    print(f"  [{i+1:2d}] OK - {size:,} bytes", end="\r")
                else:
                    print(f"\n  [{i+1:2d}] Status {response.status_code}")
            except Exception as e:
                print(f"\n  [{i+1:2d}] ERROR: {type(e).__name__}")
                print(f"  Endpoint {endpoint} caused freeze after {success} requests")
                return
            
            time.sleep(0.5)
        
        print(f"\n  ✅ Completed {success}/{count} requests")
        time.sleep(2)  # Pause between endpoint tests

if __name__ == "__main__":
    print(f"Sequential Freeze Test - {datetime.now().strftime('%H:%M:%S')}")
    print(f"Target: {ESP32_IP}")
    print()
    
    # First test: track memory during requests
    test_with_memory_tracking()
    
    # Wait for full recovery
    print("\n" + "="*60)
    print("Waiting 60s for full recovery before next test...")
    time.sleep(60)
    
    # Second test: endpoint-specific
    test_endpoint_specific()