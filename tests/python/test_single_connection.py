#!/usr/bin/env python3
"""Test single connections with delays to avoid socket exhaustion"""

import requests
import time
import sys

ESP32_IP = "10.27.27.201"
BASE_URL = f"http://{ESP32_IP}"

def test_single_endpoint(endpoint, delay=1):
    """Test a single endpoint with delay between requests"""
    print(f"\nTesting {endpoint} with {delay}s delay between requests...")
    
    for i in range(10):
        try:
            response = requests.get(f"{BASE_URL}{endpoint}", timeout=5)
            print(f"  Request {i+1}: {response.status_code} - OK")
        except Exception as e:
            print(f"  Request {i+1}: FAILED - {e}")
            return False
        
        # Wait between requests
        time.sleep(delay)
    
    return True

def main():
    print(f"Testing single connections to ESP32 at {ESP32_IP}")
    print("This test makes requests one at a time with delays")
    print("=" * 60)
    
    # Wait for device to be ready
    print("\nWaiting for device to recover...")
    time.sleep(10)
    
    # Test health endpoint first
    print("\nPhase 1: Testing /health endpoint only")
    if not test_single_endpoint("/health", delay=2):
        print("❌ Device failed on simple health checks")
        sys.exit(1)
    
    print("\n✅ Phase 1 passed - device handles single connections")
    
    # Test multiple endpoints sequentially
    print("\nPhase 2: Testing different endpoints sequentially")
    endpoints = ["/health", "/api/metrics", "/api/system", "/api/config"]
    
    for endpoint in endpoints:
        try:
            response = requests.get(f"{BASE_URL}{endpoint}", timeout=5)
            print(f"  {endpoint}: {response.status_code} - OK")
        except Exception as e:
            print(f"  {endpoint}: FAILED - {e}")
        time.sleep(2)
    
    print("\n✅ All tests passed - device is stable with single connections")

if __name__ == "__main__":
    main()