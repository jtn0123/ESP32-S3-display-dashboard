#!/usr/bin/env python3
"""Minimal test to reproduce freeze with logging"""

import requests
import time
import sys
import socket
import select
import threading


def monitor_telnet(device_ip, log_queue):
    """Monitor telnet in background"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect((device_ip, 23))
        sock.setblocking(False)
        
        while True:
            ready = select.select([sock], [], [], 0.1)
            if ready[0]:
                data = sock.recv(4096)
                if data:
                    lines = data.decode('utf-8', errors='ignore').split('\n')
                    for line in lines:
                        if line.strip():
                            log_queue.append(f"{time.strftime('%H:%M:%S')} {line.strip()}")
    except Exception as e:
        log_queue.append(f"Telnet error: {e}")


def simple_freeze_test(device_ip):
    """Simplest possible test to reproduce freeze"""
    base_url = f"http://{device_ip}"
    
    print(f"Minimal Freeze Test - {device_ip}")
    print("="*50)
    
    # Start telnet monitor
    log_queue = []
    telnet_thread = threading.Thread(target=monitor_telnet, args=(device_ip, log_queue))
    telnet_thread.daemon = True
    telnet_thread.start()
    
    print("Waiting 2s for telnet connection...")
    time.sleep(2)
    
    # Initial state
    try:
        response = requests.get(f"{base_url}/api/system", timeout=3)
        if response.status_code == 200:
            data = response.json()
            print(f"Initial state:")
            print(f"  Heap: {data.get('free_heap', 0):,} bytes")
            print(f"  Uptime: {data.get('uptime_seconds', 0)}s")
            print(f"  Temperature: {data.get('temperature', 0)}°C")
    except Exception as e:
        print(f"Failed to get initial state: {e}")
    
    print("\nStarting requests with 2s delay...")
    print("-"*50)
    
    request_count = 0
    last_heap = None
    
    for i in range(20):
        request_count += 1
        print(f"\nRequest {request_count}:")
        
        try:
            # Health check
            start = time.time()
            response = requests.get(f"{base_url}/health", timeout=5)
            elapsed = time.time() - start
            
            if response.status_code == 200:
                print(f"  /health: ✓ {elapsed:.2f}s")
                
                # Get system info
                sys_response = requests.get(f"{base_url}/api/system", timeout=5)
                if sys_response.status_code == 200:
                    data = sys_response.json()
                    heap = data.get('free_heap', 0)
                    print(f"  Heap: {heap:,} bytes", end="")
                    
                    if last_heap:
                        change = heap - last_heap
                        print(f" (change: {change:+,})")
                    else:
                        print()
                    
                    last_heap = heap
            else:
                print(f"  /health: Status {response.status_code}")
                
        except requests.exceptions.Timeout:
            print(f"  /health: TIMEOUT")
            print("\n❌ DEVICE FROZEN")
            
            # Print recent logs
            print("\nLast 20 telnet logs:")
            print("-"*50)
            for log in log_queue[-20:]:
                print(log)
            
            return False
            
        except Exception as e:
            print(f"  /health: ERROR - {type(e).__name__}: {e}")
            
        # Print any interesting logs
        critical_logs = [log for log in log_queue[-10:] 
                        if any(word in log.lower() for word in 
                              ['error', 'panic', 'abort', 'fail', 'watchdog', 'task'])]
        if critical_logs:
            print("  ⚠️  Critical logs:")
            for log in critical_logs:
                print(f"    {log}")
        
        # Clear processed logs
        if len(log_queue) > 100:
            log_queue[:50] = []
        
        print(f"  Waiting 2s...")
        time.sleep(2)
    
    print("\n✅ Test completed without freeze")
    return True


def test_specific_sequence(device_ip):
    """Test specific sequence that might trigger freeze"""
    base_url = f"http://{device_ip}"
    
    print("\nTesting specific sequence...")
    sequences = [
        # Sequence that seemed to fail before
        ["/health", "/api/system", "/health", "/api/metrics"],
    ]
    
    for seq_num, sequence in enumerate(sequences):
        print(f"\nSequence {seq_num + 1}: {' -> '.join(sequence)}")
        
        for endpoint in sequence:
            try:
                response = requests.get(base_url + endpoint, timeout=5)
                print(f"  {endpoint}: {response.status_code}")
                
                # Small delay between requests in sequence
                time.sleep(0.5)
                
            except Exception as e:
                print(f"  {endpoint}: ERROR - {type(e).__name__}")
                return False
        
        # Longer delay between sequences
        time.sleep(2)
    
    return True


def main():
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    
    # Run minimal test
    if not simple_freeze_test(device_ip):
        print("\nDevice froze during minimal test")
        
        # Wait for recovery
        print("\nWaiting 30s for recovery...")
        time.sleep(30)
        
        try:
            response = requests.get(f"http://{device_ip}/health", timeout=5)
            if response.status_code == 200:
                print("✅ Device recovered")
                
                # Try specific sequence
                print("\nTrying specific sequence test...")
                test_specific_sequence(device_ip)
            else:
                print("❌ Device still not responding")
        except:
            print("❌ Device still frozen")


if __name__ == "__main__":
    main()