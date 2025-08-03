#!/usr/bin/env python3
"""Debug script to identify what triggers device freeze"""

import requests
import time
import sys
import json
from datetime import datetime


def test_single_endpoint_repeated(device_ip, endpoint, count=50):
    """Test a single endpoint repeatedly to see if it causes freeze"""
    print(f"\nTesting {endpoint} {count} times...")
    base_url = f"http://{device_ip}"
    
    successes = 0
    last_success_time = None
    
    for i in range(count):
        try:
            start = time.time()
            response = requests.get(base_url + endpoint, timeout=3)
            elapsed = time.time() - start
            
            if response.status_code == 200:
                successes += 1
                last_success_time = time.time()
                print(f"  [{i+1}] ✓ {elapsed:.2f}s")
            else:
                print(f"  [{i+1}] ✗ Status {response.status_code}")
        except Exception as e:
            print(f"  [{i+1}] ✗ {type(e).__name__}")
            if last_success_time and time.time() - last_success_time > 10:
                print(f"\n❌ FREEZE DETECTED after {successes} successful requests")
                return False
        
        time.sleep(0.5)  # Half second between requests
    
    print(f"✅ Completed {successes}/{count} successfully")
    return True


def test_memory_exhaustion(device_ip):
    """Test if memory exhaustion causes freeze"""
    print("\nTesting memory exhaustion scenarios...")
    base_url = f"http://{device_ip}"
    
    # Monitor heap over time
    heap_samples = []
    
    for i in range(20):
        try:
            response = requests.get(f"{base_url}/api/system", timeout=3)
            if response.status_code == 200:
                data = response.json()
                heap = data.get('free_heap', 0)
                heap_samples.append(heap)
                print(f"  [{i+1}] Heap: {heap:,} bytes")
                
                # Test large response
                requests.get(f"{base_url}/api/metrics", timeout=3)
                requests.get(f"{base_url}/", timeout=3)
                
        except Exception as e:
            print(f"  [{i+1}] Error: {e}")
            if len(heap_samples) > 2 and heap_samples[-1] < heap_samples[0] * 0.5:
                print("❌ Possible memory exhaustion before freeze")
            return False
            
        time.sleep(1)
    
    return True


def test_concurrent_connections(device_ip):
    """Test if concurrent connections cause freeze"""
    print("\nTesting concurrent connections...")
    base_url = f"http://{device_ip}"
    
    import threading
    results = []
    
    def make_request(idx):
        try:
            response = requests.get(f"{base_url}/health", timeout=5)
            results.append((idx, response.status_code))
        except Exception as e:
            results.append((idx, str(e)))
    
    # Start 10 concurrent connections
    threads = []
    for i in range(10):
        t = threading.Thread(target=make_request, args=(i,))
        threads.append(t)
        t.start()
    
    # Wait for completion
    for t in threads:
        t.join()
    
    # Check results
    successes = sum(1 for _, result in results if isinstance(result, int) and result == 200)
    print(f"  Concurrent requests: {successes}/10 successful")
    
    # Check if device is still responsive
    try:
        response = requests.get(f"{base_url}/health", timeout=5)
        print("  ✅ Device still responsive after concurrent test")
        return True
    except:
        print("  ❌ Device frozen after concurrent connections")
        return False


def test_specific_api_sequences(device_ip):
    """Test specific API call sequences that might trigger freeze"""
    print("\nTesting specific API sequences...")
    base_url = f"http://{device_ip}"
    
    sequences = [
        # Sequence 1: Config updates
        [
            ("GET", "/api/config", None),
            ("POST", "/api/config", {"brightness": 50}),
            ("GET", "/api/config", None),
        ],
        # Sequence 2: Rapid metrics
        [
            ("GET", "/api/metrics", None),
            ("GET", "/api/metrics", None),
            ("GET", "/api/metrics", None),
        ],
        # Sequence 3: Mixed endpoints
        [
            ("GET", "/health", None),
            ("GET", "/api/system", None),
            ("GET", "/", None),
            ("GET", "/api/metrics", None),
        ]
    ]
    
    for seq_idx, sequence in enumerate(sequences):
        print(f"\n  Testing sequence {seq_idx + 1}...")
        
        for method, endpoint, data in sequence:
            try:
                if method == "GET":
                    response = requests.get(base_url + endpoint, timeout=3)
                else:
                    response = requests.post(base_url + endpoint, json=data, timeout=3)
                
                print(f"    {method} {endpoint}: {response.status_code}")
                
            except Exception as e:
                print(f"    {method} {endpoint}: ERROR - {e}")
                return False
        
        time.sleep(1)
    
    return True


def monitor_telnet_during_freeze(device_ip):
    """Monitor telnet logs while triggering freeze"""
    print("\nMonitoring telnet during stress...")
    
    import socket
    import select
    import threading
    
    logs = []
    stop_monitoring = False
    
    def telnet_monitor():
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            sock.connect((device_ip, 23))
            sock.setblocking(False)
            
            while not stop_monitoring:
                ready = select.select([sock], [], [], 0.1)
                if ready[0]:
                    data = sock.recv(4096)
                    if data:
                        lines = data.decode('utf-8', errors='ignore').split('\n')
                        for line in lines:
                            if line.strip():
                                logs.append(f"{datetime.now().strftime('%H:%M:%S')} {line.strip()}")
                                if "panic" in line.lower() or "abort" in line.lower():
                                    print(f"\n🔴 CRITICAL: {line}")
        except Exception as e:
            print(f"Telnet error: {e}")
    
    # Start monitoring
    monitor_thread = threading.Thread(target=telnet_monitor)
    monitor_thread.daemon = True
    monitor_thread.start()
    
    # Trigger some requests
    base_url = f"http://{device_ip}"
    freeze_detected = False
    
    for i in range(20):
        try:
            response = requests.get(f"{base_url}/api/metrics", timeout=2)
            print(f"  Request {i+1}: OK")
        except:
            print(f"  Request {i+1}: FAILED")
            if i > 5:  # Give it a few tries
                freeze_detected = True
                break
        time.sleep(0.5)
    
    stop_monitoring = True
    time.sleep(1)
    
    # Print last logs before freeze
    if freeze_detected:
        print("\n📋 Last 10 log entries before freeze:")
        for log in logs[-10:]:
            print(f"  {log}")
    
    return not freeze_detected


def main():
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    
    print(f"ESP32 Freeze Debugging Tool")
    print(f"Device: {device_ip}")
    print("="*50)
    
    # Check initial health
    try:
        response = requests.get(f"http://{device_ip}/health", timeout=5)
        if response.status_code == 200:
            data = response.json()
            print(f"✅ Device online - Uptime: {data.get('uptime_seconds', 0)}s")
        else:
            print(f"❌ Device returned status {response.status_code}")
            return
    except Exception as e:
        print(f"❌ Device not responding: {e}")
        return
    
    # Run tests
    tests = [
        ("Single endpoint - /health", lambda: test_single_endpoint_repeated(device_ip, "/health", 30)),
        ("Single endpoint - /api/metrics", lambda: test_single_endpoint_repeated(device_ip, "/api/metrics", 30)),
        ("Memory exhaustion", lambda: test_memory_exhaustion(device_ip)),
        ("Concurrent connections", lambda: test_concurrent_connections(device_ip)),
        ("API sequences", lambda: test_specific_api_sequences(device_ip)),
        ("Telnet monitoring", lambda: monitor_telnet_during_freeze(device_ip)),
    ]
    
    for test_name, test_func in tests:
        print(f"\n{'='*50}")
        print(f"Running: {test_name}")
        
        if not test_func():
            print(f"\n❌ FREEZE DETECTED during: {test_name}")
            break
        
        # Brief pause between tests
        time.sleep(2)
    
    # Final check
    print("\n" + "="*50)
    print("Final device check...")
    try:
        response = requests.get(f"http://{device_ip}/health", timeout=5)
        if response.status_code == 200:
            print("✅ Device still responsive")
        else:
            print(f"❌ Device returned status {response.status_code}")
    except Exception as e:
        print(f"❌ Device not responding: {e}")


if __name__ == "__main__":
    main()