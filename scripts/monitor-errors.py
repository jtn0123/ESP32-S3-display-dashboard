#!/usr/bin/env python3
"""
ESP32 Error and Panic Monitor
Specialized telnet monitor for detecting and analyzing critical errors
"""

import sys
import os
import re
import datetime
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from monitor_telnet import TelnetMonitor, Colors
import argparse

class ErrorMonitor(TelnetMonitor):
    def __init__(self, host: str, port: int = 23):
        super().__init__(host, port)
        
        # Define error patterns with severity and descriptions
        self.error_patterns = [
            # Panic/Crash patterns
            {
                'pattern': r'Guru Meditation Error|Core.*panicked|PANIC|panic at',
                'severity': 'CRITICAL',
                'category': 'PANIC',
                'color': Colors.RED,
                'description': 'System panic detected'
            },
            {
                'pattern': r'StoreProhibited|LoadProhibited|IllegalInstruction|InstrFetchProhibited',
                'severity': 'CRITICAL',
                'category': 'CPU_EXCEPTION',
                'color': Colors.RED,
                'description': 'CPU exception occurred'
            },
            {
                'pattern': r'abort\(\) was called|assert failed|assertion.*failed',
                'severity': 'CRITICAL',
                'category': 'ASSERTION',
                'color': Colors.RED,
                'description': 'Assertion failure'
            },
            
            # Memory errors
            {
                'pattern': r'heap_caps_malloc.*failed|MALLOC_CAP.*failed|allocation failed',
                'severity': 'HIGH',
                'category': 'MEMORY_ALLOC',
                'color': Colors.YELLOW,
                'description': 'Memory allocation failed'
            },
            {
                'pattern': r'stack overflow|Stack canary.*corrupted|stack smashing',
                'severity': 'CRITICAL',
                'category': 'STACK_OVERFLOW',
                'color': Colors.RED,
                'description': 'Stack overflow detected'
            },
            {
                'pattern': r'CORRUPT HEAP|heap corruption|bad heap',
                'severity': 'CRITICAL',
                'category': 'HEAP_CORRUPTION',
                'color': Colors.RED,
                'description': 'Heap corruption detected'
            },
            
            # Task/Watchdog errors
            {
                'pattern': r'Task watchdog.*triggered|watchdog timeout|WDT timeout',
                'severity': 'HIGH',
                'category': 'WATCHDOG',
                'color': Colors.YELLOW,
                'description': 'Watchdog timeout'
            },
            {
                'pattern': r'Failed to create task|vTaskCreate.*failed',
                'severity': 'HIGH',
                'category': 'TASK_CREATE',
                'color': Colors.YELLOW,
                'description': 'Task creation failed'
            },
            
            # Network errors
            {
                'pattern': r'WiFi: Failed|wifi.*disconnect|WIFI_REASON_|Connection lost',
                'severity': 'MEDIUM',
                'category': 'WIFI',
                'color': Colors.YELLOW,
                'description': 'WiFi connection issue'
            },
            {
                'pattern': r'httpd.*failed|HTTP server error|Failed to start.*server',
                'severity': 'HIGH',
                'category': 'HTTP_SERVER',
                'color': Colors.YELLOW,
                'description': 'HTTP server error'
            },
            {
                'pattern': r'socket.*failed|bind.*failed|listen.*failed',
                'severity': 'MEDIUM',
                'category': 'SOCKET',
                'color': Colors.YELLOW,
                'description': 'Socket operation failed'
            },
            
            # System errors
            {
                'pattern': r'Brownout detector|voltage.*low|power.*fail',
                'severity': 'HIGH',
                'category': 'POWER',
                'color': Colors.YELLOW,
                'description': 'Power issue detected'
            },
            {
                'pattern': r'NVS.*error|nvs.*failed|partition.*error',
                'severity': 'MEDIUM',
                'category': 'STORAGE',
                'color': Colors.YELLOW,
                'description': 'Storage/NVS error'
            },
            {
                'pattern': r'E \(\d+\).*:',
                'severity': 'MEDIUM',
                'category': 'ESP_LOG',
                'color': Colors.YELLOW,
                'description': 'ESP-IDF error log'
            },
            
            # Application errors
            {
                'pattern': r'ERROR|Error:|error:',
                'severity': 'LOW',
                'category': 'APP_ERROR',
                'color': Colors.YELLOW,
                'description': 'Application error'
            },
            {
                'pattern': r'FAIL|Failed|failed',
                'severity': 'LOW',
                'category': 'FAILURE',
                'color': Colors.YELLOW,
                'description': 'Operation failed'
            }
        ]
        
        # Compile patterns
        for error in self.error_patterns:
            error['regex'] = re.compile(error['pattern'], re.IGNORECASE)
        
        # Error tracking
        self.error_history = []
        self.error_counts = {}
        self.last_panic = None
        self.context_lines = []
        self.context_size = 10  # Lines before/after error
        
    def analyze_line(self, line: str):
        """Analyze line for errors and track context"""
        # Keep context buffer
        self.context_lines.append((datetime.datetime.now(), line))
        if len(self.context_lines) > self.context_size * 2:
            self.context_lines.pop(0)
        
        # Check for errors
        for error_def in self.error_patterns:
            if error_def['regex'].search(line):
                error_info = {
                    'timestamp': datetime.datetime.now(),
                    'line': line,
                    'category': error_def['category'],
                    'severity': error_def['severity'],
                    'description': error_def['description']
                }
                
                self.error_history.append(error_info)
                
                # Count by category
                category = error_def['category']
                self.error_counts[category] = self.error_counts.get(category, 0) + 1
                
                # Handle critical errors
                if error_def['severity'] == 'CRITICAL':
                    self.handle_critical_error(error_info, error_def)
                else:
                    self.display_error(error_info, error_def)
                
                break
    
    def handle_critical_error(self, error_info: dict, error_def: dict):
        """Handle critical errors with context"""
        print(f"\n{Colors.RED}{Colors.BOLD}{'='*60}")
        print(f"🚨 CRITICAL ERROR DETECTED: {error_def['category']}")
        print(f"{'='*60}{Colors.RESET}")
        
        print(f"{Colors.RED}Description: {error_def['description']}")
        print(f"Time: {error_info['timestamp'].strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]}")
        print(f"Line: {error_info['line']}{Colors.RESET}")
        
        # Show context
        print(f"\n{Colors.YELLOW}Context (last {self.context_size} lines):{Colors.RESET}")
        
        # Find current line in context
        current_index = -1
        for i, (ts, ctx_line) in enumerate(self.context_lines):
            if ctx_line == error_info['line']:
                current_index = i
                break
        
        # Show before context
        start_idx = max(0, current_index - self.context_size)
        end_idx = min(len(self.context_lines), current_index + self.context_size + 1)
        
        for i in range(start_idx, end_idx):
            if i < len(self.context_lines):
                ts, ctx_line = self.context_lines[i]
                if i == current_index:
                    print(f"{Colors.RED}>>> {ctx_line}{Colors.RESET}")
                else:
                    print(f"    {Colors.GRAY}{ctx_line}{Colors.RESET}")
        
        print(f"{Colors.RED}{'='*60}{Colors.RESET}\n")
        
        # Track panic
        if error_def['category'] == 'PANIC':
            self.last_panic = error_info
    
    def display_error(self, error_info: dict, error_def: dict):
        """Display non-critical error"""
        color = error_def['color']
        timestamp = error_info['timestamp'].strftime('%H:%M:%S.%f')[:-3]
        
        print(f"{color}[{timestamp}] [{error_def['category']}] {error_info['line']}{Colors.RESET}")
    
    def format_line(self, line: str) -> str:
        """Override to analyze all lines"""
        self.analyze_line(line)
        return super().format_line(line)
    
    def print_stats(self):
        """Print error statistics"""
        super().print_stats()
        
        if self.error_counts:
            print(f"\n{Colors.BOLD}Error Summary{Colors.RESET}")
            print("=" * 30)
            
            # Sort by count
            sorted_errors = sorted(self.error_counts.items(), key=lambda x: x[1], reverse=True)
            
            for category, count in sorted_errors:
                # Find severity
                severity = 'UNKNOWN'
                for error_def in self.error_patterns:
                    if error_def['category'] == category:
                        severity = error_def['severity']
                        break
                
                # Color by severity
                if severity == 'CRITICAL':
                    color = Colors.RED
                elif severity == 'HIGH':
                    color = Colors.YELLOW
                else:
                    color = Colors.WHITE
                
                print(f"{color}{category}: {count}{Colors.RESET}")
        
        if self.last_panic:
            print(f"\n{Colors.RED}{Colors.BOLD}Last Panic:{Colors.RESET}")
            print(f"Time: {self.last_panic['timestamp'].strftime('%Y-%m-%d %H:%M:%S')}")
            print(f"Type: {self.last_panic['category']}")
            print(f"Line: {self.last_panic['line']}")

def main():
    parser = argparse.ArgumentParser(
        description='ESP32 Error and Panic Monitor',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
This tool monitors ESP32 telnet output for errors, panics, and other issues.
It provides enhanced error detection and context for debugging.

Error Categories:
  CRITICAL: Panic, CPU exceptions, heap corruption, stack overflow
  HIGH: Memory allocation failures, watchdog timeouts, server failures
  MEDIUM: WiFi issues, socket errors, storage errors
  LOW: General application errors

Examples:
  monitor-errors.py                    # Monitor default device
  monitor-errors.py 192.168.1.100      # Monitor specific IP
  monitor-errors.py -c 20              # Show 20 lines of context
  monitor-errors.py -o errors.log      # Save errors to file
  monitor-errors.py --watch-panic      # Exit on first panic
        """
    )
    
    parser.add_argument('host', nargs='?', default='esp32-dashboard.local',
                        help='Device hostname or IP')
    parser.add_argument('-p', '--port', type=int, default=23,
                        help='Telnet port (default: 23)')
    parser.add_argument('-c', '--context', type=int, default=10,
                        help='Context lines to show (default: 10)')
    parser.add_argument('-o', '--output', metavar='FILE',
                        help='Save errors to file')
    parser.add_argument('--watch-panic', action='store_true',
                        help='Exit on first panic')
    parser.add_argument('-r', '--retry', action='store_true',
                        help='Auto-retry connection')
    
    args = parser.parse_args()
    
    # Create monitor
    monitor = ErrorMonitor(args.host, args.port)
    monitor.context_size = args.context
    
    # Connect and run
    if monitor.connect():
        try:
            monitor.run(args.output)
        except KeyboardInterrupt:
            print(f"\n{Colors.YELLOW}Monitoring stopped{Colors.RESET}")
    else:
        print(f"{Colors.RED}Failed to connect to {args.host}:{args.port}{Colors.RESET}")
        sys.exit(1)

if __name__ == '__main__':
    main()