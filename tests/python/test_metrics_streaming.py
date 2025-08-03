#!/usr/bin/env python3
"""Test streaming metrics endpoint"""

import requests
import time
import sys

# Find device
device_url = "http://192.168.1.199"

try:
    print(f"Testing streaming metrics endpoint at {device_url}/metrics")
    
    # Test 1: Single request
    print("\n--- Test 1: Single metrics request ---")
    start = time.time()
    response = requests.get(f"{device_url}/metrics", timeout=5)
    duration = time.time() - start
    
    print(f"Response status: {response.status_code}")
    print(f"Response time: {duration:.3f}s")
    print(f"Response size: {len(response.content)} bytes")
    print(f"Headers: {dict(response.headers)}")
    
    # Check Prometheus format
    if response.status_code == 200:
        lines = response.text.split('\n')
        print(f"Number of lines: {len(lines)}")
        
        # Show first few lines
        print("\nFirst 10 lines:")
        for line in lines[:10]:
            print(f"  {line}")
        
        # Check for expected metrics
        expected_metrics = [
            "esp32_device_info",
            "esp32_uptime_seconds",
            "esp32_heap_free_bytes",
            "esp32_internal_dram_free_kb",
            "esp32_psram_free_kb",
            "esp32_temperature_celsius",
            "esp32_cpu_usage_percent"
        ]
        
        found_metrics = []
        for metric in expected_metrics:
            if any(metric in line for line in lines):
                found_metrics.append(metric)
        
        print(f"\nFound metrics: {', '.join(found_metrics)}")
        print(f"Missing metrics: {', '.join(set(expected_metrics) - set(found_metrics))}")
    
    # Test 2: Sequential requests
    print("\n--- Test 2: Sequential requests ---")
    success_count = 0
    for i in range(10):
        try:
            start = time.time()
            response = requests.get(f"{device_url}/metrics", timeout=5)
            duration = time.time() - start
            
            if response.status_code == 200:
                success_count += 1
                print(f"Request {i+1}: OK ({duration:.3f}s)")
            else:
                print(f"Request {i+1}: Failed - {response.status_code}")
            
            time.sleep(0.1)  # Small delay between requests
            
        except Exception as e:
            print(f"Request {i+1}: Error - {e}")
    
    print(f"\nSuccess rate: {success_count}/10")
    
    # Test 3: Compare memory usage
    print("\n--- Test 3: Memory usage comparison ---")
    try:
        # Get system info to check memory
        sys_response = requests.get(f"{device_url}/api/system", timeout=5)
        if sys_response.status_code == 200:
            sys_info = sys_response.json()
            print(f"Free heap: {sys_info.get('free_heap', 0)} bytes")
    except Exception as e:
        print(f"Could not get system info: {e}")
    
except Exception as e:
    print(f"Error: {e}")
    sys.exit(1)