#!/usr/bin/env python3
"""
ESP32-S3 Dashboard Telnet Monitor
Remote log monitoring with color support and filtering
"""

import sys
import socket
import time
import argparse
import datetime
import re
import threading
import queue
from typing import Optional

# ANSI color codes
class Colors:
    RED = '\033[0;31m'
    GREEN = '\033[0;32m'
    YELLOW = '\033[0;33m'
    BLUE = '\033[0;34m'
    PURPLE = '\033[0;35m'
    CYAN = '\033[0;36m'
    WHITE = '\033[0;37m'
    GRAY = '\033[0;90m'
    BOLD = '\033[1m'
    RESET = '\033[0m'

class TelnetMonitor:
    def __init__(self, host: str, port: int = 23):
        self.host = host
        self.port = port
        self.socket = None
        self.connected = False
        self.log_file = None
        self.filters = []
        self.highlight_patterns = []
        self.line_queue = queue.Queue()
        self.stats = {
            'lines': 0,
            'errors': 0,
            'warnings': 0,
            'fps_current': 0.0,
            'fps_avg': 0.0,
            'cpu0': 0,
            'cpu1': 0
        }
        
    def add_filter(self, pattern: str):
        """Add a regex filter pattern"""
        try:
            self.filters.append(re.compile(pattern))
            print(f"{Colors.GREEN}Added filter: {pattern}{Colors.RESET}")
        except re.error as e:
            print(f"{Colors.RED}Invalid regex pattern: {e}{Colors.RESET}")
            
    def add_highlight(self, pattern: str, color: str):
        """Add a pattern to highlight in specific color"""
        try:
            self.highlight_patterns.append((re.compile(pattern), color))
        except re.error as e:
            print(f"{Colors.RED}Invalid regex pattern: {e}{Colors.RESET}")
    
    def connect(self, timeout: int = 5) -> bool:
        """Connect to telnet server"""
        try:
            print(f"{Colors.BLUE}Connecting to {self.host}:{self.port}...{Colors.RESET}")
            
            self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.socket.settimeout(timeout)
            self.socket.connect((self.host, self.port))
            self.socket.settimeout(0.1)  # Non-blocking for reads
            
            self.connected = True
            print(f"{Colors.GREEN}Connected successfully!{Colors.RESET}")
            return True
            
        except socket.timeout:
            print(f"{Colors.RED}Connection timeout{Colors.RESET}")
            return False
        except socket.error as e:
            print(f"{Colors.RED}Connection failed: {e}{Colors.RESET}")
            return False
            
    def disconnect(self):
        """Disconnect from server"""
        if self.socket:
            self.socket.close()
        self.connected = False
        
    def parse_line(self, line: str):
        """Parse log line for statistics"""
        # Count log levels
        if 'ERROR' in line or 'error' in line:
            self.stats['errors'] += 1
        elif 'WARN' in line or 'warning' in line:
            self.stats['warnings'] += 1
            
        # Extract FPS info
        fps_match = re.search(r'FPS: ([\d.]+) \(avg: ([\d.]+)\)', line)
        if fps_match:
            self.stats['fps_current'] = float(fps_match.group(1))
            self.stats['fps_avg'] = float(fps_match.group(2))
            
        # Extract CPU usage
        cpu_match = re.search(r'CPU0: (\d+)%.*CPU1: (\d+)%', line)
        if cpu_match:
            self.stats['cpu0'] = int(cpu_match.group(1))
            self.stats['cpu1'] = int(cpu_match.group(2))
            
    def format_line(self, line: str) -> str:
        """Format line with colors and highlighting"""
        # Default colors for log levels
        if 'ERROR' in line:
            line = f"{Colors.RED}{line}{Colors.RESET}"
        elif 'WARN' in line:
            line = f"{Colors.YELLOW}{line}{Colors.RESET}"
        elif 'INFO' in line:
            line = f"{Colors.WHITE}{line}{Colors.RESET}"
        elif '[PERF]' in line:
            line = f"{Colors.CYAN}{line}{Colors.RESET}"
        elif '[CORES]' in line:
            line = f"{Colors.PURPLE}{line}{Colors.RESET}"
            
        # Apply custom highlights
        for pattern, color in self.highlight_patterns:
            if pattern.search(line):
                color_code = getattr(Colors, color.upper(), Colors.YELLOW)
                line = f"{color_code}{line}{Colors.RESET}"
                break
                
        return line
        
    def should_display(self, line: str) -> bool:
        """Check if line should be displayed based on filters"""
        if not self.filters:
            return True
            
        for filter_pattern in self.filters:
            if filter_pattern.search(line):
                return True
        return False
        
    def reader_thread(self):
        """Thread to read from socket"""
        buffer = ""
        
        while self.connected:
            try:
                data = self.socket.recv(1024)
                if not data:
                    print(f"\n{Colors.YELLOW}Connection closed by server{Colors.RESET}")
                    self.connected = False
                    break
                    
                # Decode and add to buffer
                buffer += data.decode('utf-8', errors='replace')
                
                # Process complete lines
                while '\n' in buffer:
                    line, buffer = buffer.split('\n', 1)
                    line = line.strip('\r')
                    if line:
                        self.line_queue.put(line)
                        
            except socket.timeout:
                continue
            except Exception as e:
                print(f"\n{Colors.RED}Read error: {e}{Colors.RESET}")
                self.connected = False
                break
                
    def run(self, log_file: Optional[str] = None):
        """Main monitoring loop"""
        self.log_file = log_file
        
        if log_file:
            print(f"{Colors.YELLOW}Logging to: {log_file}{Colors.RESET}")
            
        # Start reader thread
        reader = threading.Thread(target=self.reader_thread)
        reader.daemon = True
        reader.start()
        
        # Print header
        print(f"\n{Colors.BOLD}ESP32-S3 Dashboard - Telnet Monitor{Colors.RESET}")
        print("=" * 50)
        print(f"Time: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"Press Ctrl+C to exit\n")
        
        # Main display loop
        try:
            with open(log_file, 'a') if log_file else None as f:
                while self.connected or not self.line_queue.empty():
                    try:
                        line = self.line_queue.get(timeout=0.1)
                        self.stats['lines'] += 1
                        
                        # Parse for stats
                        self.parse_line(line)
                        
                        # Check filters
                        if self.should_display(line):
                            # Format and display
                            formatted = self.format_line(line)
                            print(formatted)
                            
                            # Log to file
                            if f:
                                timestamp = datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]
                                f.write(f"[{timestamp}] {line}\n")
                                f.flush()
                                
                    except queue.Empty:
                        continue
                        
        except KeyboardInterrupt:
            print(f"\n{Colors.YELLOW}Interrupted by user{Colors.RESET}")
            
        finally:
            self.disconnect()
            self.print_stats()
            
    def print_stats(self):
        """Print session statistics"""
        print(f"\n{Colors.BOLD}Session Statistics{Colors.RESET}")
        print("=" * 30)
        print(f"Total lines: {self.stats['lines']}")
        print(f"Errors: {Colors.RED}{self.stats['errors']}{Colors.RESET}")
        print(f"Warnings: {Colors.YELLOW}{self.stats['warnings']}{Colors.RESET}")
        print(f"Last FPS: {Colors.CYAN}{self.stats['fps_current']:.1f}{Colors.RESET} (avg: {self.stats['fps_avg']:.1f})")
        print(f"CPU Usage: Core0={self.stats['cpu0']}% Core1={self.stats['cpu1']}%")

def resolve_mdns(hostname: str) -> Optional[str]:
    """Try to resolve mDNS hostname"""
    try:
        # Remove .local if present
        if hostname.endswith('.local'):
            hostname = hostname[:-6]
            
        # Try socket resolution
        full_hostname = f"{hostname}.local"
        ip = socket.gethostbyname(full_hostname)
        return ip
    except:
        return None

def find_esp32_devices():
    """Scan network for ESP32 devices"""
    print(f"{Colors.BLUE}Scanning for ESP32 devices...{Colors.RESET}")
    
    # This is a simplified version - you could enhance with actual network scanning
    # Try common ESP32 hostnames
    common_names = ['esp32-dashboard', 'esp32', 'esp32-s3']
    
    for name in common_names:
        ip = resolve_mdns(name)
        if ip:
            print(f"{Colors.GREEN}Found: {name}.local -> {ip}{Colors.RESET}")
            return ip
            
    return None

def main():
    parser = argparse.ArgumentParser(
        description='ESP32-S3 Dashboard Telnet Monitor',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  monitor-telnet.py                              # Connect using mDNS
  monitor-telnet.py 192.168.1.100                # Connect to IP
  monitor-telnet.py -f ERROR                     # Show only errors
  monitor-telnet.py -f "PERF|CORES"              # Show performance logs
  monitor-telnet.py -H Temperature RED           # Highlight temperature in red
  monitor-telnet.py -l debug.log                 # Save to log file
  monitor-telnet.py --stats-only                 # Show only statistics
  
Filter Examples:
  -f ERROR                  # Show lines containing ERROR
  -f "^\\[PERF\\]"          # Show lines starting with [PERF]
  -f "FPS|CPU|Memory"       # Show lines with FPS, CPU, or Memory
  
Highlight Examples:
  -H "FPS: [0-9.]+" GREEN   # Highlight FPS values in green
  -H "ERROR.*" RED          # Highlight error lines in red
  -H "Core[01]: \\d+%" CYAN # Highlight CPU usage in cyan
        """
    )
    
    parser.add_argument('host', nargs='?', default='esp32-dashboard.local',
                        help='Device hostname or IP (default: esp32-dashboard.local)')
    parser.add_argument('-p', '--port', type=int, default=23,
                        help='Telnet port (default: 23)')
    parser.add_argument('-l', '--log', metavar='FILE',
                        help='Save output to log file')
    parser.add_argument('-f', '--filter', action='append', metavar='REGEX',
                        help='Filter pattern (can be used multiple times)')
    parser.add_argument('-H', '--highlight', nargs=2, action='append',
                        metavar=('PATTERN', 'COLOR'),
                        help='Highlight pattern with color')
    parser.add_argument('-r', '--retry', action='store_true',
                        help='Keep retrying connection')
    parser.add_argument('-s', '--scan', action='store_true',
                        help='Scan for ESP32 devices')
    parser.add_argument('--stats-only', action='store_true',
                        help='Show only statistics, no log lines')
    
    args = parser.parse_args()
    
    # Scan for devices if requested
    if args.scan:
        ip = find_esp32_devices()
        if ip and input(f"\nConnect to {ip}? (y/n): ").lower() == 'y':
            args.host = ip
        else:
            sys.exit(0)
    
    # Resolve mDNS if needed
    if '.local' in args.host:
        print(f"{Colors.BLUE}Resolving {args.host}...{Colors.RESET}")
        ip = resolve_mdns(args.host)
        if ip:
            print(f"{Colors.GREEN}Resolved to: {ip}{Colors.RESET}")
            host = ip
        else:
            print(f"{Colors.YELLOW}Could not resolve {args.host}, trying anyway...{Colors.RESET}")
            host = args.host
    else:
        host = args.host
    
    # Create monitor
    monitor = TelnetMonitor(host, args.port)
    
    # Add filters
    if args.filter:
        for f in args.filter:
            monitor.add_filter(f)
    
    # Add highlights
    if args.highlight:
        for pattern, color in args.highlight:
            monitor.add_highlight(pattern, color)
    
    # Stats only mode
    if args.stats_only:
        monitor.add_filter(r'^$')  # Match nothing
        monitor.add_highlight(r'^\[PERF\]', 'CYAN')
        monitor.add_highlight(r'^\[CORES\]', 'PURPLE')
    
    # Connection loop
    retry_count = 0
    max_retries = 9999 if args.retry else 1
    
    while retry_count < max_retries:
        if monitor.connect():
            monitor.run(args.log)
            
            if not args.retry:
                break
                
            print(f"\n{Colors.BLUE}Retrying in 2 seconds...{Colors.RESET}")
            time.sleep(2)
        else:
            retry_count += 1
            if retry_count < max_retries:
                print(f"{Colors.BLUE}Retry {retry_count}/{max_retries} in 2 seconds...{Colors.RESET}")
                time.sleep(2)
            else:
                print(f"\n{Colors.RED}Failed to connect after {max_retries} attempts{Colors.RESET}")
                sys.exit(1)

if __name__ == '__main__':
    main()