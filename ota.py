#!/usr/bin/env python3
"""
ESP32-S3 Dashboard OTA Tool

A unified tool for OTA updates with auto-discovery support.

Usage:
    # Update a specific device
    python ota.py 192.168.1.100
    
    # Auto-discover and update all devices
    python ota.py --auto
    
    # Scan network to find devices
    python ota.py --scan
    
    # Update all devices in parallel
    python ota.py --auto --parallel
    
    # Dry run (show what would be done)
    python ota.py --auto --dry-run
"""

import sys
import os
import time
import socket
import requests
import argparse
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

try:
    from zeroconf import ServiceBrowser, Zeroconf, ServiceInfo
    MDNS_AVAILABLE = True
except ImportError:
    MDNS_AVAILABLE = False

DEFAULT_FIRMWARE = "target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard"

class ESP32Device:
    """Represents an ESP32 device"""
    def __init__(self, ip, port=8080, name=None, version="unknown"):
        self.ip = ip
        self.port = port
        self.name = name or f"esp32-{ip.split('.')[-1]}"
        self.version = version
        
    def __str__(self):
        return f"{self.name} ({self.ip}:{self.port}) v{self.version}"

class DeviceDiscovery:
    """Discovers ESP32 devices on the network"""
    
    def __init__(self):
        self.devices = []
        
    def discover_mdns(self, timeout=5):
        """Discover devices using mDNS"""
        if not MDNS_AVAILABLE:
            return []
            
        print("🔍 Searching for devices using mDNS...")
        discovered = []
        zeroconf = Zeroconf()
        
        class Listener:
            def __init__(self, discovered_list):
                self.discovered = discovered_list
                
            def add_service(self, zeroconf, service_type, name):
                info = zeroconf.get_service_info(service_type, name)
                if info:
                    ip = socket.inet_ntoa(info.addresses[0])
                    port = info.port
                    properties = info.properties
                    
                    device = ESP32Device(
                        ip=ip,
                        port=port,
                        name=name.replace('._esp32-ota._tcp.local.', ''),
                        version=properties.get(b'version', b'unknown').decode('utf-8')
                    )
                    self.discovered.append(device)
                    print(f"  ✓ Found: {device}")
                    
            def remove_service(self, zeroconf, service_type, name):
                pass
                
            def update_service(self, zeroconf, service_type, name):
                pass
        
        listener = Listener(discovered)
        browser = ServiceBrowser(zeroconf, "_esp32-ota._tcp.local.", listener)
        time.sleep(timeout)
        zeroconf.close()
        
        return discovered
        
    def scan_network(self, subnet=None):
        """Scan network for devices"""
        if not subnet:
            # Get local subnet
            hostname = socket.gethostname()
            local_ip = socket.gethostbyname(hostname)
            subnet = '.'.join(local_ip.split('.')[:-1])
            
        print(f"🔍 Scanning network {subnet}.0/24...")
        discovered = []
        
        def check_device(ip):
            try:
                response = requests.get(f"http://{ip}:8080/ota", timeout=0.5)
                if response.status_code == 200 and "ESP32-S3" in response.text:
                    # Extract version
                    version = "unknown"
                    if "Current Version:" in response.text:
                        for line in response.text.split('\n'):
                            if "Current Version:" in line:
                                version = line.split('</strong>')[1].split('</p>')[0].strip()
                                break
                    
                    return ESP32Device(ip=ip, version=version)
            except:
                pass
            return None
            
        # Parallel scan
        with ThreadPoolExecutor(max_workers=50) as executor:
            futures = []
            for i in range(1, 255):
                ip = f"{subnet}.{i}"
                futures.append(executor.submit(check_device, ip))
                
            for future in as_completed(futures):
                device = future.result()
                if device:
                    discovered.append(device)
                    print(f"  ✓ Found: {device}")
                    
        return discovered
        
    def discover_all(self):
        """Try all discovery methods"""
        devices = []
        
        # Try mDNS first
        if MDNS_AVAILABLE:
            devices.extend(self.discover_mdns())
            
        # Also scan network
        scan_devices = self.scan_network()
        
        # Merge results (avoid duplicates)
        existing_ips = {d.ip for d in devices}
        for device in scan_devices:
            if device.ip not in existing_ips:
                devices.append(device)
                
        return devices

def upload_firmware(device, firmware_path, show_progress=True):
    """Upload firmware to a device"""
    
    if not os.path.exists(firmware_path):
        print(f"❌ Firmware not found: {firmware_path}")
        return False
        
    file_size = os.path.getsize(firmware_path)
    
    print(f"\n📤 Updating {device}")
    print(f"   Firmware: {file_size:,} bytes ({file_size/1024/1024:.2f} MB)")
    
    try:
        # Check device is reachable
        response = requests.get(f"http://{device.ip}:{device.port}/ota", timeout=2)
        if response.status_code != 200:
            print("❌ Device not reachable")
            return False
            
        # Upload firmware
        with open(firmware_path, 'rb') as f:
            firmware_data = f.read()
            
        url = f"http://{device.ip}:{device.port}/ota/update"
        headers = {'Content-Length': str(file_size)}
        
        if show_progress:
            print("   Uploading...", end='', flush=True)
            
        response = requests.post(url, data=firmware_data, headers=headers, timeout=60)
        
        if response.status_code == 200:
            print("\r   ✅ Upload successful! Device will restart.")
            return True
        else:
            print(f"\r   ❌ Upload failed: HTTP {response.status_code}")
            return False
            
    except requests.exceptions.Timeout:
        print("\r   ❌ Upload timed out")
        return False
    except Exception as e:
        print(f"\r   ❌ Error: {e}")
        return False

def main():
    parser = argparse.ArgumentParser(description="ESP32-S3 Dashboard OTA Tool")
    parser.add_argument('ip', nargs='?', help='Device IP address')
    parser.add_argument('-f', '--firmware', default=DEFAULT_FIRMWARE,
                       help='Firmware file to upload')
    parser.add_argument('--auto', action='store_true',
                       help='Auto-discover and update all devices')
    parser.add_argument('--scan', action='store_true',
                       help='Only scan for devices (no update)')
    parser.add_argument('--parallel', action='store_true',
                       help='Update devices in parallel')
    parser.add_argument('--dry-run', action='store_true',
                       help='Show what would be done')
    parser.add_argument('--no-confirm', action='store_true',
                       help='Skip confirmation prompt')
    
    args = parser.parse_args()
    
    # Check if zeroconf is available
    if not MDNS_AVAILABLE and (args.auto or args.scan):
        print("💡 Tip: Install zeroconf for better device discovery:")
        print("   pip install zeroconf")
        print()
    
    # Scan only mode
    if args.scan:
        discovery = DeviceDiscovery()
        devices = discovery.discover_all()
        
        if not devices:
            print("\n❌ No devices found")
        else:
            print(f"\n📱 Found {len(devices)} device(s):")
            for i, device in enumerate(devices, 1):
                print(f"  {i}. {device}")
                print(f"     OTA URL: http://{device.ip}:8080/ota")
                print(f"     Config URL: http://{device.ip}/")
        return
        
    # Auto-discovery mode
    if args.auto:
        discovery = DeviceDiscovery()
        devices = discovery.discover_all()
        
        if not devices:
            print("\n❌ No devices found on network")
            print("\nTroubleshooting:")
            print("  1. Ensure devices are powered on and connected to WiFi")
            print("  2. Check that your computer is on the same network")
            print("  3. Try updating a specific device: python ota.py <device-ip>")
            return
            
        print(f"\n📋 Found {len(devices)} device(s) to update:")
        for i, device in enumerate(devices, 1):
            print(f"  {i}. {device}")
            
        if not args.no_confirm and not args.dry_run:
            response = input("\nUpdate all devices? (y/N): ")
            if response.lower() != 'y':
                print("Update cancelled")
                return
                
        # Update all devices
        if args.dry_run:
            print("\n[DRY RUN] Would update all devices")
            return
            
        success = 0
        if args.parallel:
            # Parallel updates
            with ThreadPoolExecutor(max_workers=5) as executor:
                futures = {executor.submit(upload_firmware, device, args.firmware): device 
                          for device in devices}
                for future in as_completed(futures):
                    if future.result():
                        success += 1
        else:
            # Sequential updates
            for device in devices:
                if upload_firmware(device, args.firmware):
                    success += 1
                    
        print(f"\n✨ Update complete: {success}/{len(devices)} successful")
        
    # Single device mode
    elif args.ip:
        device = ESP32Device(args.ip)
        
        # Check firmware exists
        if not os.path.exists(args.firmware):
            print(f"❌ Firmware not found: {args.firmware}")
            print("   Run ./compile.sh --release to build firmware")
            return
            
        if args.dry_run:
            print(f"[DRY RUN] Would update {device}")
            return
            
        success = upload_firmware(device, args.firmware)
        if success:
            print("\n✨ OTA update completed successfully!")
        else:
            print("\n❌ OTA update failed!")
            sys.exit(1)
            
    else:
        # No arguments - show help
        parser.print_help()
        print("\nExamples:")
        print("  python ota.py 192.168.1.100           # Update specific device")
        print("  python ota.py --auto                  # Update all devices")
        print("  python ota.py --scan                  # Find devices")

if __name__ == "__main__":
    main()