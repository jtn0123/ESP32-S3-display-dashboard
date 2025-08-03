#!/usr/bin/env python3
"""Quick stress test to try to trigger device freeze"""

import requests
import time
import threading
import sys
from datetime import datetime


def stress_test(device_ip, duration=60):
    """Run stress test on device"""
    print(f"Starting stress test on {device_ip} for {duration} seconds")
    print("Press Ctrl+C to stop")
    
    base_url = f"http://{device_ip}"
    start_time = time.time()
    stats = {
        'requests': 0,
        'successes': 0,
        'failures': 0,
        'last_success': time.time(),
        'freeze_detected': False
    }
    
    def status_monitor():
        """Monitor and report status"""
        while time.time() - start_time < duration:
            time.sleep(5)
            elapsed = time.time() - start_time
            since_last = time.time() - stats['last_success']
            
            print(f"[{elapsed:.0f}s] Requests: {stats['requests']}, "
                  f"Success: {stats['successes']}, Failed: {stats['failures']}, "
                  f"Last success: {since_last:.1f}s ago")
            
            if since_last > 10 and not stats['freeze_detected']:
                print("\n⚠️  POSSIBLE FREEZE DETECTED - No successful requests for 10+ seconds\n")
                stats['freeze_detected'] = True
    
    def make_concurrent_requests():
        """Make multiple concurrent requests"""
        endpoints = [
            "/health",
            "/api/metrics", 
            "/api/system",
            "/api/config",
            "/",
            "/api/network/status"
        ]
        
        def single_request(endpoint):
            try:
                response = requests.get(base_url + endpoint, timeout=2)
                stats['requests'] += 1
                if response.status_code == 200:
                    stats['successes'] += 1
                    stats['last_success'] = time.time()
                else:
                    stats['failures'] += 1
            except Exception as e:
                stats['requests'] += 1
                stats['failures'] += 1
                print(f"Error on {endpoint}: {e}")
        
        threads = []
        for endpoint in endpoints:
            t = threading.Thread(target=single_request, args=(endpoint,))
            t.start()
            threads.append(t)
        
        for t in threads:
            t.join(timeout=3)
    
    # Start monitor thread
    monitor = threading.Thread(target=status_monitor)
    monitor.daemon = True
    monitor.start()
    
    # Run stress test
    try:
        while time.time() - start_time < duration:
            make_concurrent_requests()
            time.sleep(0.5)  # Brief pause between bursts
            
    except KeyboardInterrupt:
        print("\nStopping stress test...")
    
    # Final report
    print("\n" + "="*60)
    print("STRESS TEST COMPLETE")
    print("="*60)
    print(f"Duration: {time.time() - start_time:.1f} seconds")
    print(f"Total requests: {stats['requests']}")
    print(f"Successful: {stats['successes']} ({stats['successes']/stats['requests']*100:.1f}%)")
    print(f"Failed: {stats['failures']}")
    
    if stats['freeze_detected']:
        print("\n⚠️  DEVICE FREEZE WAS DETECTED DURING TEST")
    else:
        print("\n✅ No freezes detected")
    
    # Try final health check
    print("\nFinal health check...")
    try:
        response = requests.get(f"{base_url}/health", timeout=5)
        if response.status_code == 200:
            data = response.json()
            print(f"✅ Device responsive - Uptime: {data.get('uptime_seconds', 0)}s")
        else:
            print(f"❌ Device returned status {response.status_code}")
    except Exception as e:
        print(f"❌ Device not responding: {e}")


if __name__ == "__main__":
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    duration = int(sys.argv[2]) if len(sys.argv) > 2 else 60
    
    stress_test(device_ip, duration)