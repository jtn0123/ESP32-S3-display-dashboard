#!/usr/bin/env python3
"""Surgical tests to isolate exact freeze conditions"""

import requests
import time
import sys
import threading
import json
from datetime import datetime


class FreezeIsolator:
    def __init__(self, device_ip):
        self.device_ip = device_ip
        self.base_url = f"http://{device_ip}"
        self.freeze_detected = False
        self.last_success_time = time.time()
    
    def check_health(self):
        """Quick health check"""
        try:
            response = requests.get(f"{self.base_url}/health", timeout=2)
            if response.status_code == 200:
                self.last_success_time = time.time()
                return True
        except:
            pass
        return False
    
    def test_1_single_sequential(self):
        """Test 1: Single endpoint, sequential requests"""
        print("\n🧪 TEST 1: Single Sequential Requests")
        print("Testing: 50 sequential requests to /health with 1s delay")
        
        for i in range(50):
            try:
                start = time.time()
                response = requests.get(f"{self.base_url}/health", timeout=3)
                elapsed = time.time() - start
                
                if response.status_code == 200:
                    print(f"  [{i+1:2d}] ✓ {elapsed:.2f}s", end="\r")
                else:
                    print(f"\n  [{i+1:2d}] ✗ Status {response.status_code}")
            except Exception as e:
                print(f"\n  [{i+1:2d}] ✗ {type(e).__name__}")
                if time.time() - self.last_success_time > 10:
                    print("\n❌ FREEZE DETECTED in sequential test")
                    return False
            
            time.sleep(1)
        
        print(f"\n✅ Completed 50 sequential requests successfully")
        return True
    
    def test_2_rapid_sequential(self):
        """Test 2: Rapid sequential requests (no delay)"""
        print("\n🧪 TEST 2: Rapid Sequential Requests")
        print("Testing: 100 rapid requests to /health (no delay)")
        
        successes = 0
        start_time = time.time()
        
        for i in range(100):
            try:
                response = requests.get(f"{self.base_url}/health", timeout=1)
                if response.status_code == 200:
                    successes += 1
                    self.last_success_time = time.time()
                    print(f"  Progress: {i+1}/100 (✓{successes})", end="\r")
            except:
                print(f"  Progress: {i+1}/100 (✓{successes}) - ERROR", end="\r")
                if time.time() - self.last_success_time > 5:
                    elapsed = time.time() - start_time
                    print(f"\n❌ FREEZE after {successes} requests in {elapsed:.1f}s")
                    return False
        
        elapsed = time.time() - start_time
        print(f"\n✅ Completed {successes}/100 in {elapsed:.1f}s ({successes/elapsed:.1f} req/s)")
        return True
    
    def test_3_concurrent_limited(self):
        """Test 3: Limited concurrent requests"""
        print("\n🧪 TEST 3: Limited Concurrent Requests")
        print("Testing: 2 concurrent connections for 20 seconds")
        
        results = {'success': 0, 'failed': 0}
        stop_event = threading.Event()
        
        def worker(worker_id):
            while not stop_event.is_set():
                try:
                    response = requests.get(f"{self.base_url}/health", timeout=2)
                    if response.status_code == 200:
                        results['success'] += 1
                        self.last_success_time = time.time()
                except:
                    results['failed'] += 1
                time.sleep(0.5)
        
        # Start 2 workers
        threads = []
        for i in range(2):
            t = threading.Thread(target=worker, args=(i,))
            t.daemon = True
            t.start()
            threads.append(t)
        
        # Monitor for 20 seconds
        start_time = time.time()
        while time.time() - start_time < 20:
            time.sleep(1)
            total = results['success'] + results['failed']
            print(f"  Progress: {total} requests (✓{results['success']} ✗{results['failed']})", end="\r")
            
            if time.time() - self.last_success_time > 10:
                print(f"\n❌ FREEZE detected with 2 concurrent connections")
                stop_event.set()
                return False
        
        stop_event.set()
        print(f"\n✅ Handled 2 concurrent connections successfully")
        return True
    
    def test_4_concurrent_increasing(self):
        """Test 4: Gradually increase concurrent connections"""
        print("\n🧪 TEST 4: Increasing Concurrent Connections")
        print("Testing: Gradually increase from 1 to 10 concurrent connections")
        
        for num_concurrent in range(1, 11):
            print(f"\n  Testing {num_concurrent} concurrent connections...")
            
            results = []
            threads = []
            test_duration = 5  # 5 seconds per level
            
            def worker(worker_id):
                try:
                    response = requests.get(f"{self.base_url}/health", timeout=3)
                    results.append(response.status_code == 200)
                except:
                    results.append(False)
            
            # Launch concurrent requests
            start_time = time.time()
            request_count = 0
            
            while time.time() - start_time < test_duration:
                # Launch batch of concurrent requests
                batch_threads = []
                for i in range(num_concurrent):
                    t = threading.Thread(target=worker, args=(i,))
                    t.start()
                    batch_threads.append(t)
                
                # Wait for batch to complete
                for t in batch_threads:
                    t.join(timeout=4)
                
                request_count += num_concurrent
                
                # Check results
                success_rate = sum(results[-num_concurrent:]) / num_concurrent if results else 0
                print(f"    Batch complete: {success_rate:.0%} success rate", end="\r")
                
                if success_rate < 0.5:
                    print(f"\n❌ FREEZE at {num_concurrent} concurrent connections")
                    print(f"    Total requests before freeze: {request_count}")
                    return False
                
                time.sleep(0.5)  # Brief pause between batches
            
            print(f"    ✓ {num_concurrent} concurrent: {len(results)} requests, {sum(results)/len(results):.0%} success")
        
        print("\n✅ Handled up to 10 concurrent connections")
        return True
    
    def test_5_different_endpoints(self):
        """Test 5: Test different endpoints for freeze susceptibility"""
        print("\n🧪 TEST 5: Different Endpoints")
        
        endpoints = [
            ("/health", "minimal"),
            ("/api/system", "system info"),
            ("/api/metrics", "metrics data"),
            ("/api/config", "configuration"),
            ("/", "full HTML page"),
        ]
        
        for endpoint, description in endpoints:
            print(f"\n  Testing {endpoint} ({description})...")
            
            # Try 20 rapid requests
            successes = 0
            start_time = time.time()
            
            for i in range(20):
                try:
                    response = requests.get(f"{self.base_url}{endpoint}", timeout=3)
                    if response.status_code == 200:
                        successes += 1
                        self.last_success_time = time.time()
                        
                        # Check response size
                        if i == 0:
                            size = len(response.content)
                            print(f"    Response size: {size:,} bytes")
                except Exception as e:
                    print(f"    Request {i+1} failed: {type(e).__name__}")
                    if time.time() - self.last_success_time > 5:
                        print(f"❌ FREEZE on endpoint {endpoint}")
                        return False
            
            elapsed = time.time() - start_time
            print(f"    ✓ {successes}/20 in {elapsed:.1f}s")
            
            time.sleep(2)  # Pause between endpoint tests
        
        return True
    
    def test_6_memory_monitoring(self):
        """Test 6: Monitor memory during requests"""
        print("\n🧪 TEST 6: Memory Monitoring")
        print("Testing: Monitor heap memory during 30 requests")
        
        memory_samples = []
        
        for i in range(30):
            # Get memory before request
            try:
                response = requests.get(f"{self.base_url}/api/system", timeout=3)
                if response.status_code == 200:
                    data = response.json()
                    heap = data.get('free_heap', 0)
                    memory_samples.append(heap)
                    
                    if i == 0:
                        print(f"  Initial heap: {heap:,} bytes")
                    else:
                        change = heap - memory_samples[0]
                        print(f"  [{i+1:2d}] Heap: {heap:,} ({change:+,} bytes)", end="\r")
                    
                    # Make another request to stress memory
                    requests.get(f"{self.base_url}/api/metrics", timeout=3)
                    
            except Exception as e:
                print(f"\n  Request {i+1} failed: {e}")
                if memory_samples and memory_samples[-1] < memory_samples[0] * 0.5:
                    print("❌ Possible memory exhaustion before freeze")
                return False
            
            time.sleep(0.5)
        
        if memory_samples:
            initial = memory_samples[0]
            final = memory_samples[-1]
            min_heap = min(memory_samples)
            
            print(f"\n  Final heap: {final:,} bytes")
            print(f"  Minimum heap: {min_heap:,} bytes")
            print(f"  Total change: {final - initial:+,} bytes")
            
            if final < initial * 0.8:
                print("⚠️  Significant memory loss detected")
        
        return True
    
    def test_7_connection_limits(self):
        """Test 7: Test connection limits"""
        print("\n🧪 TEST 7: Connection Limits")
        print("Testing: Open multiple connections without closing")
        
        sessions = []
        
        try:
            for i in range(20):
                # Create new session (new connection)
                session = requests.Session()
                
                # Make request but keep connection alive
                response = session.get(f"{self.base_url}/health", timeout=3)
                
                if response.status_code == 200:
                    sessions.append(session)
                    print(f"  Opened connection {i+1}", end="\r")
                else:
                    print(f"\n  Connection {i+1} failed: {response.status_code}")
                    break
                
                time.sleep(0.1)
            
            print(f"\n  Successfully opened {len(sessions)} connections")
            
            # Try one more request with all connections open
            print("  Testing with all connections open...")
            try:
                response = requests.get(f"{self.base_url}/health", timeout=5)
                if response.status_code == 200:
                    print("  ✓ Device still responsive")
                else:
                    print(f"  ✗ Device returned {response.status_code}")
            except Exception as e:
                print(f"  ✗ Device not responding: {type(e).__name__}")
                print("❌ Device frozen with multiple open connections")
                return False
                
        finally:
            # Clean up sessions
            for session in sessions:
                session.close()
        
        return True


def main():
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    
    print("ESP32 Freeze Isolation Tool")
    print("="*50)
    print(f"Device: {device_ip}")
    
    # Initial health check
    isolator = FreezeIsolator(device_ip)
    if not isolator.check_health():
        print("❌ Device not responding initially")
        return
    
    print("✅ Device responding - starting tests")
    
    # Run tests in order
    tests = [
        isolator.test_1_single_sequential,
        isolator.test_2_rapid_sequential,
        isolator.test_3_concurrent_limited,
        isolator.test_4_concurrent_increasing,
        isolator.test_5_different_endpoints,
        isolator.test_6_memory_monitoring,
        isolator.test_7_connection_limits,
    ]
    
    for test_func in tests:
        if not test_func():
            print(f"\n🔴 Freeze detected in: {test_func.__name__}")
            
            # Wait a bit and check if device recovered
            print("\nWaiting 30s for device recovery...")
            time.sleep(30)
            
            if isolator.check_health():
                print("✅ Device recovered")
            else:
                print("❌ Device still frozen")
            break
        
        # Brief pause between tests
        time.sleep(2)
        
        # Health check between tests
        if not isolator.check_health():
            print(f"\n❌ Device became unresponsive after {test_func.__name__}")
            break
    else:
        print("\n✅ All tests completed successfully!")
        
    # Final summary
    print("\n" + "="*50)
    print("SUMMARY")
    print("="*50)
    
    if isolator.freeze_detected:
        print("❌ Device freeze was detected")
    else:
        print("✅ No freezes detected")


if __name__ == "__main__":
    main()