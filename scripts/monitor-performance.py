#!/usr/bin/env python3
"""
ESP32 Performance Monitor
Real-time performance metrics visualization via telnet
"""

import sys
import os
import re
import time
import datetime
import statistics
from collections import deque
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from monitor_telnet import TelnetMonitor, Colors
import argparse

class PerformanceMonitor(TelnetMonitor):
    def __init__(self, host: str, port: int = 23):
        super().__init__(host, port)
        
        # Performance metrics tracking
        self.metrics = {
            'fps': deque(maxlen=100),
            'fps_avg': deque(maxlen=100),
            'skip_rate': deque(maxlen=100),
            'cpu0': deque(maxlen=100),
            'cpu1': deque(maxlen=100),
            'heap_free': deque(maxlen=100),
            'heap_largest': deque(maxlen=100),
            'psram_free': deque(maxlen=100),
            'psram_largest': deque(maxlen=100),
            'render_time': deque(maxlen=100),
            'flush_time': deque(maxlen=100),
            'http_requests': 0,
            'http_errors': 0,
            'wifi_disconnects': 0,
            'task_count': 0
        }
        
        # Patterns for metric extraction
        self.metric_patterns = {
            # FPS metrics
            'fps': re.compile(r'FPS: ([\d.]+) \(avg: ([\d.]+)\) skip: ([\d.]+)%'),
            'fps_simple': re.compile(r'Current FPS: ([\d.]+)'),
            
            # CPU metrics
            'cpu': re.compile(r'CPU0: (\d+)%.*CPU1: (\d+)%'),
            'cpu_freq': re.compile(r'CPU Freq: (\d+)MHz'),
            
            # Memory metrics
            'heap': re.compile(r'Heap - free: (\d+), largest: (\d+)'),
            'internal_ram': re.compile(r'Internal - free: (\d+), largest: (\d+)'),
            'psram': re.compile(r'PSRAM - free: (\d+), largest: (\d+)'),
            
            # Performance timing
            'perf': re.compile(r'\[PERF\] Render: ([\d.]+)ms, Flush: ([\d.]+)ms, FPS: ([\d.]+)'),
            'timing': re.compile(r'render_time=([\d.]+)ms.*flush_time=([\d.]+)ms'),
            
            # HTTP metrics
            'http_request': re.compile(r'HTTP.*request.*(?:GET|POST|PUT)'),
            'http_error': re.compile(r'HTTP.*(?:error|failed|Error)'),
            
            # System metrics
            'task_count': re.compile(r'Tasks: (\d+)'),
            'wifi_disconnect': re.compile(r'WiFi.*disconnect|WIFI_REASON_')
        }
        
        # Display settings
        self.update_interval = 1.0  # seconds
        self.last_display_update = 0
        self.show_graph = True
        self.graph_height = 10
        
    def parse_line(self, line: str):
        """Extract performance metrics from log line"""
        super().parse_line(line)
        
        # FPS metrics
        fps_match = self.metric_patterns['fps'].search(line)
        if fps_match:
            self.metrics['fps'].append(float(fps_match.group(1)))
            self.metrics['fps_avg'].append(float(fps_match.group(2)))
            self.metrics['skip_rate'].append(float(fps_match.group(3)))
        else:
            fps_simple = self.metric_patterns['fps_simple'].search(line)
            if fps_simple:
                self.metrics['fps'].append(float(fps_simple.group(1)))
        
        # CPU metrics
        cpu_match = self.metric_patterns['cpu'].search(line)
        if cpu_match:
            self.metrics['cpu0'].append(int(cpu_match.group(1)))
            self.metrics['cpu1'].append(int(cpu_match.group(2)))
        
        # Memory metrics
        heap_match = self.metric_patterns['heap'].search(line)
        if heap_match:
            self.metrics['heap_free'].append(int(heap_match.group(1)))
            self.metrics['heap_largest'].append(int(heap_match.group(2)))
        
        internal_match = self.metric_patterns['internal_ram'].search(line)
        if internal_match:
            self.metrics['heap_free'].append(int(internal_match.group(1)))
            self.metrics['heap_largest'].append(int(internal_match.group(2)))
        
        psram_match = self.metric_patterns['psram'].search(line)
        if psram_match:
            self.metrics['psram_free'].append(int(psram_match.group(1)))
            self.metrics['psram_largest'].append(int(psram_match.group(2)))
        
        # Performance timing
        perf_match = self.metric_patterns['perf'].search(line)
        if perf_match:
            self.metrics['render_time'].append(float(perf_match.group(1)))
            self.metrics['flush_time'].append(float(perf_match.group(2)))
        
        # HTTP metrics
        if self.metric_patterns['http_request'].search(line):
            self.metrics['http_requests'] += 1
        if self.metric_patterns['http_error'].search(line):
            self.metrics['http_errors'] += 1
        
        # WiFi disconnects
        if self.metric_patterns['wifi_disconnect'].search(line):
            self.metrics['wifi_disconnects'] += 1
        
        # Task count
        task_match = self.metric_patterns['task_count'].search(line)
        if task_match:
            self.metrics['task_count'] = int(task_match.group(1))
    
    def format_line(self, line: str) -> str:
        """Override to update display periodically"""
        formatted = super().format_line(line)
        
        # Update display if interval has passed
        current_time = time.time()
        if current_time - self.last_display_update >= self.update_interval:
            self.display_metrics()
            self.last_display_update = current_time
        
        return formatted
    
    def display_metrics(self):
        """Display performance metrics dashboard"""
        # Clear previous display (move cursor up)
        if hasattr(self, 'last_display_lines'):
            print(f"\033[{self.last_display_lines}A", end='')
        
        lines = []
        
        # Header
        lines.append(f"\n{Colors.BOLD}{'='*60}")
        lines.append(f"ESP32 Performance Monitor - {datetime.datetime.now().strftime('%H:%M:%S')}")
        lines.append(f"{'='*60}{Colors.RESET}")
        
        # FPS Metrics
        if self.metrics['fps']:
            current_fps = self.metrics['fps'][-1]
            avg_fps = statistics.mean(self.metrics['fps'])
            
            fps_color = Colors.GREEN if current_fps > 50 else Colors.YELLOW if current_fps > 10 else Colors.RED
            lines.append(f"{Colors.CYAN}Display Performance:{Colors.RESET}")
            lines.append(f"  FPS: {fps_color}{current_fps:6.1f}{Colors.RESET} (avg: {avg_fps:6.1f})")
            
            if self.metrics['skip_rate']:
                skip_rate = self.metrics['skip_rate'][-1]
                skip_color = Colors.GREEN if skip_rate > 90 else Colors.YELLOW if skip_rate > 50 else Colors.RED
                lines.append(f"  Skip Rate: {skip_color}{skip_rate:5.1f}%{Colors.RESET}")
            
            if self.show_graph:
                lines.append(self.create_graph(self.metrics['fps'], "FPS", 0, 100))
        
        # CPU Metrics
        if self.metrics['cpu0'] and self.metrics['cpu1']:
            cpu0 = self.metrics['cpu0'][-1]
            cpu1 = self.metrics['cpu1'][-1]
            total_cpu = (cpu0 + cpu1) / 2
            
            cpu_color = Colors.GREEN if total_cpu < 50 else Colors.YELLOW if total_cpu < 80 else Colors.RED
            lines.append(f"\n{Colors.CYAN}CPU Usage:{Colors.RESET}")
            lines.append(f"  Core 0: {self.get_bar(cpu0, 100, 20)} {cpu0:3d}%")
            lines.append(f"  Core 1: {self.get_bar(cpu1, 100, 20)} {cpu1:3d}%")
        
        # Memory Metrics
        if self.metrics['heap_free']:
            heap_free = self.metrics['heap_free'][-1]
            heap_mb = heap_free / 1024 / 1024
            
            mem_color = Colors.GREEN if heap_mb > 0.1 else Colors.YELLOW if heap_mb > 0.05 else Colors.RED
            lines.append(f"\n{Colors.CYAN}Memory:{Colors.RESET}")
            lines.append(f"  Internal RAM: {mem_color}{heap_mb:5.2f} MB{Colors.RESET} free")
            
            if self.metrics['heap_largest']:
                largest_kb = self.metrics['heap_largest'][-1] / 1024
                lines.append(f"  Largest Block: {largest_kb:6.1f} KB")
        
        if self.metrics['psram_free']:
            psram_free = self.metrics['psram_free'][-1]
            psram_mb = psram_free / 1024 / 1024
            lines.append(f"  PSRAM: {Colors.GREEN}{psram_mb:5.2f} MB{Colors.RESET} free")
        
        # Timing Metrics
        if self.metrics['render_time'] and self.metrics['flush_time']:
            render_ms = statistics.mean(list(self.metrics['render_time'])[-10:])
            flush_ms = statistics.mean(list(self.metrics['flush_time'])[-10:])
            total_ms = render_ms + flush_ms
            
            lines.append(f"\n{Colors.CYAN}Frame Timing (avg last 10):{Colors.RESET}")
            lines.append(f"  Render: {render_ms:5.2f} ms")
            lines.append(f"  Flush:  {flush_ms:5.2f} ms")
            lines.append(f"  Total:  {total_ms:5.2f} ms ({1000/total_ms if total_ms > 0 else 0:.1f} FPS max)")
        
        # Network Metrics
        lines.append(f"\n{Colors.CYAN}Network:{Colors.RESET}")
        lines.append(f"  HTTP Requests: {self.metrics['http_requests']:5d}")
        if self.metrics['http_errors'] > 0:
            lines.append(f"  HTTP Errors:   {Colors.RED}{self.metrics['http_errors']:5d}{Colors.RESET}")
        if self.metrics['wifi_disconnects'] > 0:
            lines.append(f"  WiFi Disconnects: {Colors.YELLOW}{self.metrics['wifi_disconnects']:3d}{Colors.RESET}")
        
        # Print all lines
        for line in lines:
            print(line)
        
        self.last_display_lines = len(lines) + 2
    
    def get_bar(self, value: float, max_value: float, width: int) -> str:
        """Create a progress bar"""
        filled = int((value / max_value) * width)
        bar = '█' * filled + '░' * (width - filled)
        
        if value < 50:
            color = Colors.GREEN
        elif value < 80:
            color = Colors.YELLOW
        else:
            color = Colors.RED
        
        return f"{color}{bar}{Colors.RESET}"
    
    def create_graph(self, data: deque, label: str, min_val: float, max_val: float) -> str:
        """Create a simple ASCII graph"""
        if not data:
            return ""
        
        # Take last 40 samples
        samples = list(data)[-40:]
        if not samples:
            return ""
        
        # Normalize to graph height
        graph_lines = []
        for h in range(self.graph_height, 0, -1):
            line = "  "
            threshold = min_val + (max_val - min_val) * (h / self.graph_height)
            
            for val in samples:
                if val >= threshold:
                    line += "█"
                else:
                    line += " "
            
            if h == self.graph_height:
                line += f" {max_val:.0f}"
            elif h == 1:
                line += f" {min_val:.0f}"
            
            graph_lines.append(line)
        
        return "\n".join(graph_lines)
    
    def run(self, log_file=None):
        """Override run to show initial display"""
        # Show initial metrics display
        self.display_metrics()
        super().run(log_file)

def main():
    parser = argparse.ArgumentParser(
        description='ESP32 Performance Monitor',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Real-time performance monitoring for ESP32 devices.
Displays FPS, CPU usage, memory stats, and network metrics.

Examples:
  monitor-performance.py                    # Monitor default device
  monitor-performance.py 192.168.1.100      # Monitor specific IP
  monitor-performance.py -u 0.5             # Update every 0.5 seconds
  monitor-performance.py --no-graph         # Disable graphs
  monitor-performance.py -f "PERF|CORES"    # Also show matching logs
        """
    )
    
    parser.add_argument('host', nargs='?', default='esp32-dashboard.local',
                        help='Device hostname or IP')
    parser.add_argument('-p', '--port', type=int, default=23,
                        help='Telnet port (default: 23)')
    parser.add_argument('-u', '--update', type=float, default=1.0,
                        help='Update interval in seconds (default: 1.0)')
    parser.add_argument('--no-graph', action='store_true',
                        help='Disable graphs')
    parser.add_argument('-f', '--filter', action='append',
                        help='Additional log filters')
    parser.add_argument('-l', '--log', metavar='FILE',
                        help='Save raw logs to file')
    
    args = parser.parse_args()
    
    # Create monitor
    monitor = PerformanceMonitor(args.host, args.port)
    monitor.update_interval = args.update
    monitor.show_graph = not args.no_graph
    
    # Add filters for performance logs
    monitor.add_filter(r'\[PERF\]|\[CORES\]|FPS:|CPU\d:|Memory:|Heap')
    
    # Add user filters
    if args.filter:
        for f in args.filter:
            monitor.add_filter(f)
    
    # Connect and run
    if monitor.connect():
        try:
            monitor.run(args.log)
        except KeyboardInterrupt:
            print(f"\n\n{Colors.YELLOW}Monitoring stopped{Colors.RESET}")
    else:
        print(f"{Colors.RED}Failed to connect to {args.host}:{args.port}{Colors.RESET}")
        sys.exit(1)

if __name__ == '__main__':
    main()