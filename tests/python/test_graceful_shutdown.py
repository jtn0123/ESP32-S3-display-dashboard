#!/usr/bin/env python3
"""Test graceful shutdown functionality"""

import sys
import time
import telnetlib
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed

def monitor_telnet_during_shutdown(device_ip, duration=30):
    """Monitor telnet output during shutdown sequence"""
    print(f"\nMonitoring telnet during shutdown on {device_ip}...")
    
    try:
        tn = telnetlib.Telnet(device_ip, 23, timeout=5)
        print("Connected to telnet server")
        
        start_time = time.time()
        shutdown_detected = False
        
        while time.time() - start_time < duration:
            try:
                # Read available data
                data = tn.read_very_eager().decode('utf-8', errors='ignore')
                if data:
                    for line in data.splitlines():
                        print(f"  TELNET: {line}")
                        
                        # Look for shutdown indicators
                        if any(x in line.lower() for x in ['shutdown', 'shutting down', 'graceful']):
                            shutdown_detected = True
                            print(f"\n✅ SHUTDOWN DETECTED: {line}")
                
                time.sleep(0.1)
                
            except Exception as e:
                print(f"\n❌ Telnet connection lost: {e}")
                break
        
        return shutdown_detected
        
    except Exception as e:
        print(f"❌ Failed to connect to telnet: {e}")
        return False

def test_service_availability(device_ip):
    """Test if services are still responding"""
    services = {
        'Web': f'http://{device_ip}/',
        'Health': f'http://{device_ip}/health',
        'Metrics': f'http://{device_ip}/metrics',
        'API': f'http://{device_ip}/api/system'
    }
    
    results = {}
    for name, url in services.items():
        try:
            resp = requests.get(url, timeout=2)
            results[name] = resp.status_code == 200
        except:
            results[name] = False
    
    return results

def test_graceful_shutdown(device_ip):
    """Test graceful shutdown behavior"""
    print(f"Testing graceful shutdown on {device_ip}")
    print("="*60)
    
    # Test 1: Check initial service availability
    print("\n--- Test 1: Initial Service Status ---")
    initial_services = test_service_availability(device_ip)
    for service, available in initial_services.items():
        status = "✅" if available else "❌"
        print(f"{status} {service}: {'Available' if available else 'Not available'}")
    
    if not any(initial_services.values()):
        print("\n❌ No services available - device may be offline")
        return
    
    # Test 2: Monitor shutdown sequence
    print("\n--- Test 2: Shutdown Sequence ---")
    print("NOTE: Trigger shutdown manually on device (hold both buttons)")
    print("Monitoring for 30 seconds...")
    
    # Start concurrent monitoring
    with ThreadPoolExecutor(max_workers=2) as executor:
        # Monitor telnet
        telnet_future = executor.submit(monitor_telnet_during_shutdown, device_ip, 30)
        
        # Monitor service availability
        service_checks = []
        start_time = time.time()
        
        while time.time() - start_time < 25:
            services = test_service_availability(device_ip)
            timestamp = time.time() - start_time
            service_checks.append((timestamp, services))
            
            # Show status
            active = sum(1 for v in services.values() if v)
            print(f"\r  [{timestamp:5.1f}s] Active services: {active}/4", end='', flush=True)
            
            if active == 0:
                print("\n  All services stopped")
                break
                
            time.sleep(1)
        
        # Wait for telnet monitoring to complete
        shutdown_detected = telnet_future.result()
    
    # Test 3: Analyze shutdown pattern
    print("\n\n--- Test 3: Shutdown Analysis ---")
    
    if shutdown_detected:
        print("✅ Graceful shutdown was detected")
    else:
        print("⚠️  No explicit shutdown message detected")
    
    # Analyze service shutdown order
    print("\nService shutdown timeline:")
    last_status = {k: True for k in initial_services.keys()}
    
    for timestamp, services in service_checks:
        for service, available in services.items():
            if last_status.get(service, True) and not available:
                print(f"  [{timestamp:5.1f}s] {service} stopped")
                last_status[service] = False
    
    # Test 4: Final state check
    print("\n--- Test 4: Final State ---")
    time.sleep(2)
    final_services = test_service_availability(device_ip)
    
    all_stopped = not any(final_services.values())
    if all_stopped:
        print("✅ All services properly stopped")
    else:
        print("⚠️  Some services still running:")
        for service, available in final_services.items():
            if available:
                print(f"   - {service}")
    
    # Test 5: Recovery test
    print("\n--- Test 5: Recovery Test ---")
    print("Waiting 10 seconds for device to restart...")
    time.sleep(10)
    
    recovery_services = test_service_availability(device_ip)
    recovered = any(recovery_services.values())
    
    if recovered:
        print("✅ Device recovered after shutdown")
        for service, available in recovery_services.items():
            status = "✅" if available else "❌"
            print(f"{status} {service}: {'Available' if available else 'Not available'}")
    else:
        print("❌ Device did not recover automatically")
    
    print("\n" + "="*60)
    print("Graceful shutdown test complete!")

if __name__ == "__main__":
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    test_graceful_shutdown(device_ip)