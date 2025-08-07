#!/usr/bin/env python3
"""Enhanced telnet monitoring with filtering and analysis"""

import socket
import select
import time
import sys
import re
from collections import defaultdict
from datetime import datetime

class TelnetMonitor:
    def __init__(self, host, port=23):
        self.host = host
        self.port = port
        self.sock = None
        self.filters = []
        self.stats = defaultdict(int)
        self.error_patterns = [
            (r'panic|PANIC', 'PANIC'),
            (r'assert|Assert', 'ASSERT'),
            (r'abort|Abort', 'ABORT'),
            (r'E \(', 'ERROR'),
            (r'failed|Failed|FAILED', 'FAILED'),
            (r'crash|Crash|CRASH', 'CRASH'),
            (r'heap_caps_malloc.*failed', 'HEAP_ALLOC_FAIL'),
            (r'stack overflow', 'STACK_OVERFLOW'),
            (r'StoreProhibited|LoadProhibited', 'MEMORY_ACCESS'),
            (r'Guru Meditation', 'GURU_MEDITATION'),
            (r'Watchdog', 'WATCHDOG'),
            (r'Connection refused', 'CONN_REFUSED'),
            (r'Out of memory', 'OOM'),
        ]
        self.important_patterns = [
            (r'📊 Memory', 'MEMORY_LOG'),
            (r'Free heap:', 'HEAP_INFO'),
            (r'Internal DRAM:', 'DRAM_INFO'),
            (r'PSRAM:', 'PSRAM_INFO'),
            (r'OTA', 'OTA_EVENT'),
            (r'WiFi:', 'WIFI_EVENT'),
            (r'HTTP server', 'HTTP_EVENT'),
            (r'Restart', 'RESTART'),
            (r'Version', 'VERSION'),
        ]
        
    def connect(self):
        """Connect to telnet server"""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.settimeout(5)
            self.sock.connect((self.host, self.port))
            self.sock.setblocking(False)
            print(f"✅ Connected to {self.host}:{self.port}")
            return True
        except Exception as e:
            print(f"❌ Failed to connect: {e}")
            return False
    
    def add_filter(self, pattern, name=None):
        """Add a filter pattern"""
        try:
            regex = re.compile(pattern, re.IGNORECASE)
            self.filters.append((regex, name or pattern))
            print(f"Added filter: {name or pattern}")
        except re.error as e:
            print(f"Invalid regex pattern: {e}")
    
    def process_line(self, line):
        """Process a single log line"""
        # Update stats
        self.stats['total_lines'] += 1
        
        # Check for errors
        for pattern, error_type in self.error_patterns:
            if re.search(pattern, line, re.IGNORECASE):
                self.stats[f'error_{error_type}'] += 1
                return f"🚨 {error_type}", line, 'red'
        
        # Check for important events
        for pattern, event_type in self.important_patterns:
            if re.search(pattern, line, re.IGNORECASE):
                self.stats[f'event_{event_type}'] += 1
                return f"📌 {event_type}", line, 'yellow'
        
        # Check user filters
        if self.filters:
            for regex, name in self.filters:
                if regex.search(line):
                    self.stats[f'filter_{name}'] += 1
                    return f"🔍 {name}", line, 'green'
            return None  # Filtered out
        
        return "", line, None
    
    def monitor(self, duration=None, show_all=False):
        """Monitor telnet output"""
        if not self.sock:
            if not self.connect():
                return
        
        print(f"\nMonitoring telnet logs...")
        if self.filters and not show_all:
            print(f"Filters active: {[name for _, name in self.filters]}")
        print("Press Ctrl+C to stop\n")
        
        start_time = time.time()
        last_stats_time = start_time
        
        try:
            while True:
                # Check duration
                if duration and time.time() - start_time > duration:
                    break
                
                # Check for data
                ready = select.select([self.sock], [], [], 0.1)
                if ready[0]:
                    try:
                        data = self.sock.recv(4096)
                        if not data:
                            print("Connection closed")
                            break
                        
                        # Process lines
                        lines = data.decode('utf-8', errors='ignore').split('\n')
                        for line in lines:
                            line = line.strip()
                            if not line:
                                continue
                            
                            result = self.process_line(line)
                            if result is None and not show_all:
                                continue  # Filtered out
                            
                            if result:
                                prefix, content, color = result
                                timestamp = datetime.now().strftime('%H:%M:%S')
                                
                                # Color codes
                                colors = {
                                    'red': '\033[91m',
                                    'yellow': '\033[93m',
                                    'green': '\033[92m',
                                    'reset': '\033[0m'
                                }
                                
                                if color and color in colors:
                                    print(f"{timestamp} {colors[color]}{prefix:15} {content}{colors['reset']}")
                                else:
                                    print(f"{timestamp} {prefix:15} {content}")
                    
                    except socket.error:
                        pass
                
                # Print stats periodically
                if time.time() - last_stats_time > 10:
                    self.print_stats()
                    last_stats_time = time.time()
                    
        except KeyboardInterrupt:
            print("\n\nMonitoring stopped")
        finally:
            self.disconnect()
            self.print_final_stats()
    
    def print_stats(self):
        """Print current statistics"""
        print(f"\n--- Stats (Total: {self.stats['total_lines']} lines) ---")
        
        # Errors
        errors = [(k.replace('error_', ''), v) for k, v in self.stats.items() if k.startswith('error_')]
        if errors:
            print("Errors:")
            for name, count in sorted(errors, key=lambda x: x[1], reverse=True):
                print(f"  {name}: {count}")
        
        # Events
        events = [(k.replace('event_', ''), v) for k, v in self.stats.items() if k.startswith('event_')]
        if events:
            print("Events:")
            for name, count in sorted(events, key=lambda x: x[1], reverse=True):
                print(f"  {name}: {count}")
        
        print()
    
    def print_final_stats(self):
        """Print final statistics"""
        print("\n" + "="*60)
        print("FINAL STATISTICS")
        print("="*60)
        
        self.print_stats()
        
        # Summary
        total_errors = sum(v for k, v in self.stats.items() if k.startswith('error_'))
        if total_errors > 0:
            print(f"⚠️  Total errors detected: {total_errors}")
        else:
            print("✅ No errors detected")
    
    def disconnect(self):
        """Disconnect from telnet server"""
        if self.sock:
            self.sock.close()
            self.sock = None

def main():
    if len(sys.argv) < 2:
        print("Usage: telnet_monitor.py <device_ip> [options]")
        print("\nOptions:")
        print("  -f <pattern>   Add filter (can be used multiple times)")
        print("  -a             Show all lines (ignore filters)")
        print("  -t <seconds>   Monitor for specific duration")
        print("\nExamples:")
        print("  telnet_monitor.py 10.27.27.201")
        print("  telnet_monitor.py 10.27.27.201 -f 'OTA|HTTP' -f 'Memory'")
        print("  telnet_monitor.py 10.27.27.201 -f 'error|fail' -t 60")
        return
    
    host = sys.argv[1]
    monitor = TelnetMonitor(host)
    
    # Parse arguments
    show_all = '-a' in sys.argv
    duration = None
    
    i = 2
    while i < len(sys.argv):
        if sys.argv[i] == '-f' and i + 1 < len(sys.argv):
            monitor.add_filter(sys.argv[i + 1])
            i += 2
        elif sys.argv[i] == '-t' and i + 1 < len(sys.argv):
            duration = int(sys.argv[i + 1])
            i += 2
        else:
            i += 1
    
    # Run monitor
    monitor.monitor(duration=duration, show_all=show_all)

if __name__ == "__main__":
    main()