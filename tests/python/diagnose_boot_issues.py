#!/usr/bin/env python3
"""Diagnose boot and network issues"""

import serial
import serial.tools.list_ports
import time
import sys
import subprocess

def find_esp32_serial_port():
    """Find ESP32 serial port"""
    ports = list(serial.tools.list_ports.comports())
    for port in ports:
        if "USB" in port.description or "Serial" in port.description:
            if "cu.usbmodem" in port.device or "ttyUSB" in port.device:
                return port.device
    return None

def monitor_serial(port, duration=30):
    """Monitor serial output for errors"""
    print(f"Monitoring serial port {port} for {duration} seconds...")
    print("Looking for panic, errors, and boot issues...\n")
    
    error_keywords = [
        "panic",
        "PANIC",
        "abort",
        "assert",
        "failed",
        "error",
        "ERROR",
        "StoreProhibited",
        "LoadProhibited",
        "IllegalInstruction",
        "InstrFetchProhibited",
        "heap_caps_malloc",
        "stack overflow",
        "watchdog",
        "Brownout",
        "WiFi: Failed",
        "E (",  # ESP-IDF error logs
        "crash",
        "CRASH",
        "reboot",
        "restart"
    ]
    
    boot_stages = {
        "boot:": "Boot stage",
        "WiFi: ": "WiFi initialization",
        "network_manager": "Network manager",
        "web_server": "Web server",
        "OTA": "OTA system",
        "display": "Display driver",
        "ESP32-S3 Dashboard": "Main app start"
    }
    
    try:
        with serial.Serial(port, 115200, timeout=1) as ser:
            start_time = time.time()
            errors_found = []
            boot_progress = []
            
            while time.time() - start_time < duration:
                if ser.in_waiting:
                    try:
                        line = ser.readline().decode('utf-8', errors='ignore').strip()
                        if line:
                            # Check for boot stages
                            for stage, desc in boot_stages.items():
                                if stage in line:
                                    boot_progress.append(f"{desc}: {line}")
                                    print(f"✓ {desc}")
                                    break
                            
                            # Check for errors
                            for keyword in error_keywords:
                                if keyword in line:
                                    errors_found.append(line)
                                    print(f"❌ ERROR: {line}")
                                    break
                            else:
                                # Print other potentially interesting lines
                                if any(x in line.lower() for x in ['version', 'heap', 'memory', 'init']):
                                    print(f"  {line}")
                    except:
                        pass
            
            print(f"\n{'='*60}")
            print("SUMMARY:")
            print(f"{'='*60}")
            
            if errors_found:
                print(f"\n❌ Found {len(errors_found)} errors:")
                for error in errors_found[:10]:  # Show first 10 errors
                    print(f"  - {error}")
            else:
                print("\n✅ No critical errors detected")
            
            if boot_progress:
                print(f"\n📊 Boot progress:")
                for stage in boot_progress[-5:]:  # Show last 5 stages
                    print(f"  - {stage}")
            
            return len(errors_found) == 0
            
    except serial.SerialException as e:
        print(f"❌ Could not open serial port: {e}")
        return False

def check_network_connectivity(timeout=60):
    """Check if device appears on network"""
    print(f"\n{'='*60}")
    print("Checking network connectivity...")
    print(f"{'='*60}")
    
    start_time = time.time()
    while time.time() - start_time < timeout:
        # Try OTA scan
        result = subprocess.run(
            ["./scripts/ota.sh", "find"],
            capture_output=True,
            text=True,
            timeout=10
        )
        
        if "Found ESP32" in result.stdout or "Found device" in result.stdout:
            print("✅ Device found on network!")
            # Extract IP if possible
            import re
            ip_match = re.search(r'(\d+\.\d+\.\d+\.\d+)', result.stdout)
            if ip_match:
                print(f"   IP: {ip_match.group(1)}")
            return True
        
        # Also try common IPs
        for ip in ["10.27.27.201", "192.168.1.199", "192.168.4.1"]:
            try:
                import requests
                resp = requests.get(f"http://{ip}/health", timeout=1)
                if resp.status_code == 200:
                    print(f"✅ Device found at {ip}")
                    return True
            except:
                pass
        
        remaining = int(timeout - (time.time() - start_time))
        print(f"   Still searching... ({remaining}s remaining)", end='\r')
        time.sleep(2)
    
    print("\n❌ Device not found on network after 60 seconds")
    return False

def main():
    print("ESP32 Boot Diagnostics Tool")
    print("="*60)
    
    # Find serial port
    port = find_esp32_serial_port()
    if not port:
        print("❌ No ESP32 serial port found")
        print("   Please connect the device via USB")
        return 1
    
    print(f"Found ESP32 on port: {port}")
    
    # Monitor serial
    serial_ok = monitor_serial(port, duration=30)
    
    # Check network
    network_ok = check_network_connectivity(timeout=30)
    
    # Final diagnosis
    print(f"\n{'='*60}")
    print("DIAGNOSIS:")
    print(f"{'='*60}")
    
    if serial_ok and network_ok:
        print("✅ Device is working properly")
    elif serial_ok and not network_ok:
        print("⚠️  Device boots but doesn't connect to network")
        print("   Possible causes:")
        print("   - Wrong WiFi credentials")
        print("   - WiFi initialization failure")
        print("   - Web server crash after boot")
    elif not serial_ok and network_ok:
        print("⚠️  Unexpected: errors in serial but device on network")
    else:
        print("❌ Device has critical issues")
        print("   - Check serial output for panic messages")
        print("   - Device may be in boot loop")
        print("   - Memory corruption possible")
    
    return 0 if (serial_ok and network_ok) else 1

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\nInterrupted by user")
        sys.exit(1)