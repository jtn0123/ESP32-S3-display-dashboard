#!/usr/bin/env python3
"""
ESP32-S3 Dashboard Telnet Control Script
Provides commands for device control via telnet while streaming logs
"""

import telnetlib
import sys
import time
import argparse
import requests
import socket
import re
from datetime import datetime

def find_esp32_devices():
    """Use mDNS to find ESP32 devices on the network"""
    try:
        import subprocess
        # Use dns-sd on macOS or avahi-browse on Linux
        if sys.platform == 'darwin':
            cmd = ['dns-sd', '-B', '_esp32-ota._tcp', 'local.']
            proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            time.sleep(2)
            proc.terminate()
            output = proc.stdout.read().decode()
            
            # Extract hostnames
            devices = []
            for line in output.split('\n'):
                if 'esp32' in line.lower():
                    parts = line.split()
                    if len(parts) >= 6:
                        hostname = parts[-3]
                        if hostname.endswith('.local.'):
                            hostname = hostname[:-1]  # Remove trailing dot
                        devices.append(hostname)
            return list(set(devices))
    except Exception as e:
        print(f"mDNS discovery failed: {e}")
    
    # Fallback to checking common hostname
    try:
        socket.gethostbyname('esp32.local')
        return ['esp32.local']
    except:
        return []

def get_device_info(host):
    """Get device information via HTTP"""
    try:
        # Try health endpoint
        response = requests.get(f'http://{host}/health', timeout=2)
        if response.status_code == 200:
            data = response.json()
            return {
                'version': data.get('version', 'Unknown'),
                'uptime': data.get('uptime_seconds', 0),
                'heap': data.get('free_heap', 0)
            }
    except:
        pass
    
    return None

def format_uptime(seconds):
    """Format uptime seconds to human readable"""
    days = seconds // 86400
    hours = (seconds % 86400) // 3600
    minutes = (seconds % 3600) // 60
    secs = seconds % 60
    
    if days > 0:
        return f"{days}d {hours}h {minutes}m"
    elif hours > 0:
        return f"{hours}h {minutes}m {secs}s"
    else:
        return f"{minutes}m {secs}s"

class TelnetControl:
    def __init__(self, host, port=23):
        self.host = host
        self.port = port
        self.tn = None
        self.running = True
        
    def connect(self):
        """Connect to telnet server"""
        try:
            print(f"Connecting to {self.host}:{self.port}...")
            self.tn = telnetlib.Telnet(self.host, self.port, timeout=5)
            print("Connected! Type 'help' for commands\n")
            return True
        except Exception as e:
            print(f"Connection failed: {e}")
            return False
    
    def send_http_command(self, command):
        """Send commands via HTTP that telnet doesn't support"""
        if command == 'restart':
            print("Sending restart command via HTTP...")
            try:
                # Most ESP32 web servers have a restart endpoint
                response = requests.post(f'http://{self.host}/restart', timeout=5)
                if response.status_code < 400:
                    print("Restart command sent successfully!")
                    print("Device will restart in 3 seconds...")
                    return True
            except:
                pass
            
            print("Restart endpoint not available. Please restart manually.")
            return False
            
        elif command == 'stats':
            info = get_device_info(self.host)
            if info:
                print("\nDevice Statistics:")
                print(f"  Version: {info['version']}")
                print(f"  Uptime:  {format_uptime(info['uptime'])}")
                print(f"  Heap:    {info['heap'] // 1024} KB\n")
                return True
            else:
                print("Could not retrieve device stats via HTTP")
                return False
                
        return False
    
    def process_command(self, cmd):
        """Process local commands"""
        cmd = cmd.strip().lower()
        
        if cmd in ['help', '?']:
            print("\nAvailable Commands:")
            print("  help      - Show this help")
            print("  stats     - Show device statistics (via HTTP)")
            print("  restart   - Restart the device (via HTTP)")
            print("  clear     - Clear the screen")
            print("  quit      - Exit the program")
            print("  filter X  - Filter logs containing X")
            print("  nofilter  - Remove log filter\n")
            return True
            
        elif cmd == 'clear':
            print('\033[2J\033[H')  # ANSI clear screen
            return True
            
        elif cmd in ['quit', 'exit']:
            self.running = False
            return True
            
        elif cmd == 'stats':
            return self.send_http_command('stats')
            
        elif cmd == 'restart':
            return self.send_http_command('restart')
            
        elif cmd.startswith('filter '):
            self.filter_pattern = cmd[7:]
            print(f"Filtering logs for: {self.filter_pattern}")
            return True
            
        elif cmd == 'nofilter':
            self.filter_pattern = None
            print("Filter removed")
            return True
            
        else:
            print(f"Unknown command: {cmd}")
            return False
    
    def run(self):
        """Main loop"""
        import threading
        import queue
        
        # Queue for user input
        input_queue = queue.Queue()
        self.filter_pattern = None
        
        def input_thread():
            while self.running:
                try:
                    line = input()
                    input_queue.put(line)
                except EOFError:
                    break
        
        # Start input thread
        threading.Thread(target=input_thread, daemon=True).start()
        
        # Main loop
        while self.running:
            try:
                # Check for user input
                while not input_queue.empty():
                    cmd = input_queue.get()
                    if cmd.strip():
                        self.process_command(cmd)
                
                # Read telnet data
                data = self.tn.read_some()
                if data:
                    text = data.decode('utf-8', errors='ignore')
                    
                    # Apply filter if set
                    if self.filter_pattern:
                        lines = text.split('\n')
                        filtered = [line for line in lines if self.filter_pattern in line]
                        if filtered:
                            print('\n'.join(filtered), end='', flush=True)
                    else:
                        print(text, end='', flush=True)
                        
            except KeyboardInterrupt:
                print("\n\nInterrupted by user")
                break
            except Exception as e:
                print(f"\nConnection lost: {e}")
                break
        
        print("\nDisconnecting...")
        if self.tn:
            self.tn.close()

def main():
    parser = argparse.ArgumentParser(description='ESP32-S3 Dashboard Telnet Control')
    parser.add_argument('host', nargs='?', help='Device hostname or IP')
    parser.add_argument('-p', '--port', type=int, default=23, help='Telnet port (default: 23)')
    parser.add_argument('-s', '--scan', action='store_true', help='Scan for devices')
    
    args = parser.parse_args()
    
    if args.scan:
        print("Scanning for ESP32 devices...")
        devices = find_esp32_devices()
        if devices:
            print(f"\nFound {len(devices)} device(s):")
            for device in devices:
                info = get_device_info(device)
                if info:
                    print(f"  - {device} (v{info['version']}, up {format_uptime(info['uptime'])})")
                else:
                    print(f"  - {device}")
        else:
            print("No devices found")
        return
    
    # Get host
    host = args.host
    if not host:
        # Try to find device
        devices = find_esp32_devices()
        if devices:
            host = devices[0]
            print(f"Found device: {host}")
        else:
            print("No device specified and none found via mDNS")
            print("Usage: telnet-control.py <hostname|ip>")
            return
    
    # Connect and run
    control = TelnetControl(host, args.port)
    if control.connect():
        print("ESP32-S3 Dashboard Telnet Control")
        print("================================")
        print("This tool adds command functionality to the telnet log stream")
        print("Type 'help' for available commands\n")
        
        # Show device info if available
        info = get_device_info(host)
        if info:
            print(f"Device: {host}")
            print(f"Version: {info['version']}")
            print(f"Uptime: {format_uptime(info['uptime'])}")
            print(f"Free heap: {info['heap'] // 1024} KB\n")
        
        control.run()

if __name__ == '__main__':
    main()