#!/usr/bin/env python3
"""Test connection monitoring metrics"""

import sys
import time
import requests
import telnetlib
from concurrent.futures import ThreadPoolExecutor, as_completed

def test_connection_metrics(device_ip):
    """Test connection monitoring functionality"""
    print(f"Testing connection monitoring on {device_ip}")
    print("="*60)
    
    # Test 1: Get initial metrics
    print("\n--- Test 1: Initial Metrics ---")
    try:
        resp = requests.get(f"http://{device_ip}/metrics", timeout=5)
        if resp.status_code == 200:
            lines = resp.text.split('\n')
            connection_metrics = {}
            
            for line in lines:
                if line.startswith('esp32_http_connections_active'):
                    connection_metrics['http_active'] = float(line.split()[-1])
                elif line.startswith('esp32_http_connections_total'):
                    connection_metrics['http_total'] = float(line.split()[-1])
                elif line.startswith('esp32_telnet_connections_active'):
                    connection_metrics['telnet_active'] = float(line.split()[-1])
                elif line.startswith('esp32_telnet_connections_total'):
                    connection_metrics['telnet_total'] = float(line.split()[-1])
                elif line.startswith('esp32_wifi_disconnects_total'):
                    connection_metrics['wifi_disconnects'] = float(line.split()[-1])
                elif line.startswith('esp32_wifi_reconnects_total'):
                    connection_metrics['wifi_reconnects'] = float(line.split()[-1])
                elif line.startswith('esp32_session_uptime_seconds'):
                    connection_metrics['uptime'] = float(line.split()[-1])
            
            print("Initial connection metrics:")
            for key, value in connection_metrics.items():
                print(f"  {key}: {value}")
                
            initial_http_total = connection_metrics.get('http_total', 0)
            initial_telnet_total = connection_metrics.get('telnet_total', 0)
        else:
            print(f"❌ Failed to get metrics: {resp.status_code}")
            return
    except Exception as e:
        print(f"❌ Error getting metrics: {e}")
        return
    
    # Test 2: Create multiple HTTP connections
    print("\n--- Test 2: HTTP Connection Load Test ---")
    print("Creating 10 concurrent HTTP connections...")
    
    def make_http_request(i):
        try:
            resp = requests.get(f"http://{device_ip}/", timeout=5)
            return i, resp.status_code
        except Exception as e:
            return i, str(e)
    
    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = [executor.submit(make_http_request, i) for i in range(10)]
        results = []
        for future in as_completed(futures):
            results.append(future.result())
    
    success = sum(1 for _, status in results if isinstance(status, int) and status == 200)
    print(f"✅ Successful connections: {success}/10")
    
    # Test 3: Create telnet connections
    print("\n--- Test 3: Telnet Connection Test ---")
    telnet_connections = []
    
    for i in range(3):
        try:
            tn = telnetlib.Telnet(device_ip, 23, timeout=5)
            telnet_connections.append(tn)
            print(f"✅ Telnet connection {i+1} established")
        except Exception as e:
            print(f"❌ Telnet connection {i+1} failed: {e}")
    
    # Wait a moment for metrics to update
    time.sleep(2)
    
    # Test 4: Check updated metrics
    print("\n--- Test 4: Updated Metrics ---")
    try:
        resp = requests.get(f"http://{device_ip}/metrics", timeout=5)
        if resp.status_code == 200:
            lines = resp.text.split('\n')
            updated_metrics = {}
            
            for line in lines:
                if line.startswith('esp32_http_connections_total'):
                    updated_metrics['http_total'] = float(line.split()[-1])
                elif line.startswith('esp32_telnet_connections_active'):
                    updated_metrics['telnet_active'] = float(line.split()[-1])
                elif line.startswith('esp32_telnet_connections_total'):
                    updated_metrics['telnet_total'] = float(line.split()[-1])
            
            print("Updated connection metrics:")
            print(f"  HTTP connections total: {updated_metrics.get('http_total', 0)} "
                  f"(+{updated_metrics.get('http_total', 0) - initial_http_total})")
            print(f"  Telnet connections active: {updated_metrics.get('telnet_active', 0)}")
            print(f"  Telnet connections total: {updated_metrics.get('telnet_total', 0)} "
                  f"(+{updated_metrics.get('telnet_total', 0) - initial_telnet_total})")
    except Exception as e:
        print(f"❌ Error getting updated metrics: {e}")
    
    # Test 5: Close telnet connections
    print("\n--- Test 5: Connection Cleanup ---")
    for i, tn in enumerate(telnet_connections):
        try:
            tn.close()
            print(f"✅ Closed telnet connection {i+1}")
        except:
            pass
    
    # Wait and check final metrics
    time.sleep(2)
    
    try:
        resp = requests.get(f"http://{device_ip}/metrics", timeout=5)
        if resp.status_code == 200:
            lines = resp.text.split('\n')
            for line in lines:
                if line.startswith('esp32_telnet_connections_active'):
                    active = float(line.split()[-1])
                    print(f"\nFinal telnet connections active: {active}")
                    if active == 0:
                        print("✅ All telnet connections properly closed")
                    else:
                        print(f"⚠️  {active} telnet connections still active")
    except Exception as e:
        print(f"❌ Error checking final metrics: {e}")
    
    # Test 6: Verify uptime tracking
    print("\n--- Test 6: Uptime Tracking ---")
    try:
        resp = requests.get(f"http://{device_ip}/metrics", timeout=5)
        if resp.status_code == 200:
            lines = resp.text.split('\n')
            for line in lines:
                if line.startswith('esp32_session_uptime_seconds'):
                    uptime = float(line.split()[-1])
                    print(f"Current session uptime: {uptime:.0f} seconds ({uptime/60:.1f} minutes)")
                    if uptime > 0:
                        print("✅ Uptime tracking is working")
                    else:
                        print("❌ Uptime not being tracked")
    except Exception as e:
        print(f"❌ Error checking uptime: {e}")
    
    print("\n" + "="*60)
    print("Connection monitoring test complete!")

if __name__ == "__main__":
    device_ip = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"
    test_connection_metrics(device_ip)