#!/usr/bin/env python3
"""
Comprehensive stress testing script for ESP32-S3 Dashboard.
Systematically tests device limits to find breaking points.
"""

import time
import json
import requests
import argparse
import threading
from datetime import datetime
from typing import Dict, List, Optional
import concurrent.futures


class StressTester:
    """Stress test the ESP32 device."""
    
    def __init__(self, device_ip: str):
        self.device_ip = device_ip
        self.base_url = f"http://{device_ip}"
        self.results = []
        
    def is_device_alive(self, timeout: float = 2.0) -> bool:
        """Check if device is responding."""
        try:
            response = requests.get(f"{self.base_url}/health", timeout=timeout)
            return response.status_code == 200
        except:
            return False
            
    def wait_for_device(self, max_wait: float = 30.0) -> bool:
        """Wait for device to come back online."""
        print("Waiting for device to recover...")
        start = time.time()
        while time.time() - start < max_wait:
            if self.is_device_alive():
                print("✅ Device is back online")
                return True
            time.sleep(1)
        print("❌ Device did not recover")
        return False
        
    def test_endpoint_endurance(self, endpoint: str, duration: int = 60,
                               requests_per_second: float = 1.0) -> Dict:
        """Test how long an endpoint can handle sustained load."""
        print(f"\n🔨 Endurance test: {endpoint} @ {requests_per_second} req/s for {duration}s")
        
        result = {
            'test': 'endpoint_endurance',
            'endpoint': endpoint,
            'requests_per_second': requests_per_second,
            'duration': duration,
            'total_requests': 0,
            'successful_requests': 0,
            'failed_requests': 0,
            'device_crashed': False,
            'crash_after_seconds': None,
            'crash_after_requests': None,
            'avg_response_time': 0
        }
        
        interval = 1.0 / requests_per_second
        response_times = []
        start_time = time.time()
        
        while time.time() - start_time < duration:
            req_start = time.time()
            
            try:
                response = requests.get(f"{self.base_url}{endpoint}", 
                                      timeout=5.0,
                                      headers={'Connection': 'close'})
                response_time = time.time() - req_start
                
                result['total_requests'] += 1
                if response.status_code == 200:
                    result['successful_requests'] += 1
                    response_times.append(response_time)
                else:
                    result['failed_requests'] += 1
                    
            except Exception as e:
                result['total_requests'] += 1
                result['failed_requests'] += 1
                
                # Check if device crashed
                if not self.is_device_alive():
                    result['device_crashed'] = True
                    result['crash_after_seconds'] = time.time() - start_time
                    result['crash_after_requests'] = result['total_requests']
                    print(f"💥 Device crashed after {result['crash_after_seconds']:.1f}s "
                          f"and {result['crash_after_requests']} requests")
                    break
                    
            # Rate limiting
            elapsed = time.time() - req_start
            if elapsed < interval:
                time.sleep(interval - elapsed)
                
        if response_times:
            result['avg_response_time'] = sum(response_times) / len(response_times)
            
        print(f"✅ Completed: {result['successful_requests']}/{result['total_requests']} successful")
        return result
        
    def test_concurrent_burst(self, endpoint: str, concurrent: int = 5,
                            burst_count: int = 10) -> Dict:
        """Test device response to concurrent request bursts."""
        print(f"\n🔨 Burst test: {concurrent} concurrent requests × {burst_count} bursts to {endpoint}")
        
        result = {
            'test': 'concurrent_burst',
            'endpoint': endpoint,
            'concurrent_requests': concurrent,
            'burst_count': burst_count,
            'successful_bursts': 0,
            'failed_bursts': 0,
            'device_crashed': False,
            'crash_after_burst': None
        }
        
        def make_request():
            try:
                response = requests.get(f"{self.base_url}{endpoint}",
                                      timeout=5.0,
                                      headers={'Connection': 'close'})
                return response.status_code == 200
            except:
                return False
                
        for burst_num in range(burst_count):
            print(f"  Burst {burst_num + 1}/{burst_count}...", end='', flush=True)
            
            # Send concurrent requests
            with concurrent.futures.ThreadPoolExecutor(max_workers=concurrent) as executor:
                futures = [executor.submit(make_request) for _ in range(concurrent)]
                results = [f.result() for f in concurrent.futures.as_completed(futures)]
                
            success_count = sum(results)
            if success_count == concurrent:
                result['successful_bursts'] += 1
                print(" ✅")
            else:
                result['failed_bursts'] += 1
                print(f" ⚠️  ({success_count}/{concurrent} succeeded)")
                
            # Check if device is still alive
            time.sleep(1)
            if not self.is_device_alive():
                result['device_crashed'] = True
                result['crash_after_burst'] = burst_num + 1
                print(f"💥 Device crashed after burst {burst_num + 1}")
                break
                
            # Wait between bursts
            time.sleep(2)
            
        return result
        
    def test_memory_exhaustion(self, endpoint: str = "/dashboard",
                             acceleration: float = 1.5) -> Dict:
        """Test device by gradually increasing request rate until crash."""
        print(f"\n🔨 Memory exhaustion test: {endpoint} with {acceleration}x acceleration")
        
        result = {
            'test': 'memory_exhaustion',
            'endpoint': endpoint,
            'acceleration': acceleration,
            'max_rate_achieved': 0,
            'requests_before_crash': 0,
            'duration_before_crash': 0,
            'final_heap_size': None
        }
        
        current_rate = 0.5  # Start at 0.5 req/s
        start_time = time.time()
        total_requests = 0
        
        while True:
            print(f"  Testing at {current_rate:.1f} req/s...", end='', flush=True)
            
            # Test at current rate for 10 seconds
            interval = 1.0 / current_rate
            rate_start = time.time()
            rate_requests = 0
            
            while time.time() - rate_start < 10:
                try:
                    req_start = time.time()
                    response = requests.get(f"{self.base_url}{endpoint}",
                                          timeout=5.0,
                                          headers={'Connection': 'close'})
                    
                    total_requests += 1
                    rate_requests += 1
                    
                    # Try to get heap size
                    try:
                        health = requests.get(f"{self.base_url}/health", timeout=1).json()
                        result['final_heap_size'] = health.get('free_heap')
                    except:
                        pass
                        
                except:
                    # Check if device crashed
                    if not self.is_device_alive():
                        result['max_rate_achieved'] = current_rate
                        result['requests_before_crash'] = total_requests
                        result['duration_before_crash'] = time.time() - start_time
                        print(f"\n💥 Device crashed at {current_rate:.1f} req/s")
                        print(f"   Total requests: {total_requests}")
                        print(f"   Duration: {result['duration_before_crash']:.1f}s")
                        return result
                        
                # Rate limiting
                elapsed = time.time() - req_start
                if elapsed < interval:
                    time.sleep(interval - elapsed)
                    
            print(f" ✅ ({rate_requests} requests)")
            
            # Increase rate
            current_rate *= acceleration
            if current_rate > 20:  # Safety limit
                print("⚠️  Reached safety limit of 20 req/s")
                result['max_rate_achieved'] = current_rate
                result['requests_before_crash'] = total_requests
                result['duration_before_crash'] = time.time() - start_time
                break
                
        return result
        
    def test_payload_bomb(self, endpoint: str = "/api/config") -> Dict:
        """Test device with increasingly large payloads."""
        print(f"\n🔨 Payload bomb test: {endpoint}")
        
        result = {
            'test': 'payload_bomb',
            'endpoint': endpoint,
            'max_payload_handled': 0,
            'device_crashed': False,
            'crash_at_size': None
        }
        
        sizes = [100, 500, 1000, 2000, 5000, 10000, 20000, 50000]
        
        for size in sizes:
            print(f"  Testing {size} byte payload...", end='', flush=True)
            
            payload = {
                'test_data': 'x' * size,
                'size': size
            }
            
            try:
                response = requests.post(f"{self.base_url}{endpoint}",
                                       json=payload,
                                       timeout=10.0,
                                       headers={'Connection': 'close'})
                
                if response.status_code in [200, 201, 400, 413]:
                    result['max_payload_handled'] = size
                    print(" ✅")
                else:
                    print(f" ⚠️  Status {response.status_code}")
                    
            except Exception as e:
                print(f" ❌ {type(e).__name__}")
                
                # Check if device crashed
                if not self.is_device_alive():
                    result['device_crashed'] = True
                    result['crash_at_size'] = size
                    print(f"💥 Device crashed at payload size {size}")
                    break
                    
            # Wait between tests
            time.sleep(2)
            
        return result
        
    def run_all_tests(self) -> List[Dict]:
        """Run all stress tests."""
        tests = [
            # Endurance tests
            ('endpoint_endurance', lambda: self.test_endpoint_endurance("/health", 30, 2.0)),
            ('endpoint_endurance', lambda: self.test_endpoint_endurance("/api/metrics", 30, 1.0)),
            ('endpoint_endurance', lambda: self.test_endpoint_endurance("/dashboard", 20, 0.5)),
            
            # Burst tests
            ('concurrent_burst', lambda: self.test_concurrent_burst("/health", 3, 5)),
            ('concurrent_burst', lambda: self.test_concurrent_burst("/api/metrics", 2, 5)),
            
            # Exhaustion tests
            ('memory_exhaustion', lambda: self.test_memory_exhaustion("/api/metrics", 1.5)),
            
            # Payload tests
            ('payload_bomb', lambda: self.test_payload_bomb("/api/config")),
        ]
        
        for test_name, test_func in tests:
            print(f"\n{'='*60}")
            print(f"Running {test_name}")
            print(f"{'='*60}")
            
            result = test_func()
            result['timestamp'] = datetime.now().isoformat()
            self.results.append(result)
            
            # If device crashed, wait for recovery
            if result.get('device_crashed'):
                if not self.wait_for_device():
                    print("⚠️  Device did not recover, stopping tests")
                    break
                else:
                    # Extra recovery time
                    print("Waiting 10s for full recovery...")
                    time.sleep(10)
            else:
                # Normal wait between tests
                time.sleep(5)
                
        return self.results
        
    def save_results(self, filename: str = "stress_test_results.json"):
        """Save test results to file."""
        with open(filename, 'w') as f:
            json.dump({
                'device_ip': self.device_ip,
                'test_time': datetime.now().isoformat(),
                'results': self.results
            }, f, indent=2)
            
        print(f"\n📊 Results saved to {filename}")
        
    def print_summary(self):
        """Print test summary."""
        print("\n" + "="*60)
        print("STRESS TEST SUMMARY")
        print("="*60)
        
        crashes = [r for r in self.results if r.get('device_crashed')]
        
        print(f"\nTotal tests run: {len(self.results)}")
        print(f"Tests causing crashes: {len(crashes)}")
        
        if crashes:
            print("\nCrash details:")
            for crash in crashes:
                if crash['test'] == 'endpoint_endurance':
                    print(f"  - {crash['endpoint']} crashed after {crash['crash_after_seconds']:.1f}s "
                          f"at {crash['requests_per_second']} req/s")
                elif crash['test'] == 'concurrent_burst':
                    print(f"  - {crash['endpoint']} crashed after burst {crash['crash_after_burst']} "
                          f"with {crash['concurrent_requests']} concurrent requests")
                elif crash['test'] == 'memory_exhaustion':
                    print(f"  - {crash['endpoint']} crashed at {crash['max_rate_achieved']:.1f} req/s "
                          f"after {crash['requests_before_crash']} requests")
                elif crash['test'] == 'payload_bomb':
                    print(f"  - {crash['endpoint']} crashed with {crash['crash_at_size']} byte payload")
                    
        # Find limits
        print("\nDevice limits discovered:")
        
        # Max sustained rate
        endurance_tests = [r for r in self.results 
                          if r['test'] == 'endpoint_endurance' and not r['device_crashed']]
        if endurance_tests:
            max_rate = max(r['requests_per_second'] for r in endurance_tests)
            print(f"  - Max sustained rate: {max_rate} req/s")
            
        # Max concurrent
        burst_tests = [r for r in self.results 
                      if r['test'] == 'concurrent_burst' and not r['device_crashed']]
        if burst_tests:
            max_concurrent = max(r['concurrent_requests'] for r in burst_tests)
            print(f"  - Max concurrent requests: {max_concurrent}")
            
        # Max payload
        payload_tests = [r for r in self.results if r['test'] == 'payload_bomb']
        if payload_tests:
            max_payload = max(r['max_payload_handled'] for r in payload_tests)
            print(f"  - Max payload size: {max_payload} bytes")


def main():
    parser = argparse.ArgumentParser(description='Stress test ESP32 device')
    parser.add_argument('--ip', default='10.27.27.201', help='Device IP address')
    parser.add_argument('--test', help='Run specific test only')
    
    args = parser.parse_args()
    
    tester = StressTester(args.ip)
    
    print(f"🚀 Starting stress tests for {args.ip}")
    print("⚠️  WARNING: These tests may crash the device!")
    
    # Check device is alive
    if not tester.is_device_alive():
        print("❌ Device is not responding")
        return
        
    if args.test:
        # Run specific test
        if args.test == 'endurance':
            result = tester.test_endpoint_endurance("/api/metrics", 60, 1.0)
        elif args.test == 'burst':
            result = tester.test_concurrent_burst("/api/metrics", 3, 10)
        elif args.test == 'exhaustion':
            result = tester.test_memory_exhaustion("/api/metrics")
        elif args.test == 'payload':
            result = tester.test_payload_bomb()
        else:
            print(f"Unknown test: {args.test}")
            return
            
        tester.results = [result]
    else:
        # Run all tests
        tester.run_all_tests()
        
    # Save and summarize
    tester.save_results()
    tester.print_summary()


if __name__ == "__main__":
    main()