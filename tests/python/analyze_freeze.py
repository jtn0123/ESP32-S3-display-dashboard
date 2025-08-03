#!/usr/bin/env python3
"""Analyze freeze patterns based on findings"""

import requests
import time
import sys
import threading


def analyze_freeze_patterns(device_ip):
    """Analyze different freeze patterns"""
    base_url = f"http://{device_ip}"
    
    print("ESP32 Freeze Pattern Analysis")
    print("="*50)
    print(f"Device: {device_ip}")
    
    # Based on our findings:
    # - Web server stack size: 8192 bytes
    # - Max URI handlers: 40
    # - ESP-IDF HTTP server default limits:
    #   - Default max open sockets: 10
    #   - Default backlog connections: 5
    #   - Default max URI length: 512
    #   - Default max header length: 512
    
    print("\n📊 Known Configuration:")
    print("  - Stack size: 8192 bytes")
    print("  - Max URI handlers: 40")
    print("  - ESP-IDF defaults:")
    print("    - Max open sockets: ~10 (typical)")
    print("    - Connection backlog: ~5")
    
    # Test 1: Connection exhaustion
    print("\n🧪 Test 1: Connection Exhaustion")
    print("Theory: ESP-IDF has limited sockets (typically 10)")
    
    persistent_sessions = []
    freeze_at = None
    
    try:
        for i in range(15):
            session = requests.Session()
            # Keep connection alive
            session.headers['Connection'] = 'keep-alive'
            
            try:
                response = session.get(f"{base_url}/health", timeout=3)
                if response.status_code == 200:
                    persistent_sessions.append(session)
                    print(f"  Connection {i+1}: ✓ Established")
                else:
                    print(f"  Connection {i+1}: Status {response.status_code}")
                    break
            except Exception as e:
                print(f"  Connection {i+1}: ✗ {type(e).__name__}")
                freeze_at = i
                break
            
            time.sleep(0.2)
    finally:
        # Close all sessions
        for s in persistent_sessions:
            try:
                s.close()
            except:
                pass
    
    if freeze_at is not None:
        print(f"\n  ❌ Device froze at connection #{freeze_at + 1}")
        print("  💡 This matches typical ESP-IDF socket limit!")
        
        # Wait for recovery
        print("\n  Waiting 20s for socket cleanup...")
        time.sleep(20)
        
        try:
            response = requests.get(f"{base_url}/health", timeout=5)
            if response.status_code == 200:
                print("  ✅ Device recovered after closing connections")
            else:
                print("  ❌ Device still not responsive")
        except:
            print("  ❌ Device still frozen")
    else:
        print("\n  ✅ Handled all connections")
    
    # Test 2: Rapid connection cycling
    print("\n🧪 Test 2: Rapid Connection Cycling")
    print("Theory: Opening/closing connections rapidly exhausts resources")
    
    successes = 0
    failures = 0
    start_time = time.time()
    
    for i in range(30):
        try:
            # New connection each time (no session reuse)
            response = requests.get(f"{base_url}/health", timeout=2)
            if response.status_code == 200:
                successes += 1
            else:
                failures += 1
        except:
            failures += 1
            if failures > 5:
                elapsed = time.time() - start_time
                print(f"\n  ❌ Too many failures after {elapsed:.1f}s")
                print(f"  Stats: {successes} success, {failures} failed")
                break
        
        # No delay - stress the connection handling
    
    # Test 3: Large response handling
    print("\n🧪 Test 3: Large Response Handling")
    print("Theory: Large responses with limited heap cause issues")
    
    try:
        # Get initial heap
        response = requests.get(f"{base_url}/api/system", timeout=3)
        if response.status_code == 200:
            initial_heap = response.json().get('free_heap', 0)
            print(f"  Initial heap: {initial_heap:,} bytes")
        
        # Request large responses
        large_endpoints = [
            ("/", "HTML page"),
            ("/api/logs", "Logs"),
            ("/api/metrics", "Metrics"),
        ]
        
        for endpoint, desc in large_endpoints:
            try:
                response = requests.get(f"{base_url}{endpoint}", timeout=5)
                if response.status_code == 200:
                    size = len(response.content)
                    print(f"  {desc}: {size:,} bytes")
                    
                    # Check heap after
                    sys_resp = requests.get(f"{base_url}/api/system", timeout=3)
                    if sys_resp.status_code == 200:
                        current_heap = sys_resp.json().get('free_heap', 0)
                        print(f"    Heap after: {current_heap:,} ({current_heap - initial_heap:+,})")
            except Exception as e:
                print(f"  {desc}: Failed - {type(e).__name__}")
    except Exception as e:
        print(f"  Initial request failed: {e}")
    
    # Test 4: Task stack overflow
    print("\n🧪 Test 4: Handler Stack Pressure")
    print("Theory: Complex handlers with 8KB stack might overflow")
    
    # Try endpoints that might use more stack
    complex_endpoints = [
        ("/api/config", "POST", {"test": "x" * 1000}),  # Large JSON
        ("/api/metrics", "GET", None),  # Complex response building
        ("/api/logs", "GET", None),  # String processing
    ]
    
    for endpoint, method, data in complex_endpoints:
        try:
            if method == "GET":
                response = requests.get(f"{base_url}{endpoint}", timeout=5)
            else:
                response = requests.post(f"{base_url}{endpoint}", json=data, timeout=5)
            
            print(f"  {method} {endpoint}: {response.status_code}")
        except Exception as e:
            print(f"  {method} {endpoint}: {type(e).__name__}")
    
    # Summary
    print("\n" + "="*50)
    print("FREEZE ANALYSIS SUMMARY")
    print("="*50)
    print("\n🔍 Most likely causes:")
    print("1. Socket exhaustion (ESP-IDF limit ~10 connections)")
    print("2. Memory fragmentation from rapid allocations")
    print("3. Task stack overflow in HTTP handlers (8KB limit)")
    print("4. Heap exhaustion from concurrent requests")
    print("\n💡 Recommendations:")
    print("1. Increase max_open_sockets in HTTP server config")
    print("2. Add connection rate limiting")
    print("3. Increase handler stack size (currently 8KB)")
    print("4. Add heap monitoring and request throttling")


def main():
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    analyze_freeze_patterns(device_ip)


if __name__ == "__main__":
    main()