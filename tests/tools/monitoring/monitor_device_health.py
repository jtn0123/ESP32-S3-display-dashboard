#!/usr/bin/env python3
"""
Real-time device health monitor for ESP32-S3 Dashboard.
Tracks memory, uptime, and response times to identify stability issues.
"""

import time
import json
import requests
import argparse
import threading
from datetime import datetime
from collections import deque
try:
    import matplotlib.pyplot as plt
    import matplotlib.animation as animation
    from matplotlib.dates import DateFormatter
    import numpy as np
    PLOTTING_AVAILABLE = True
except ImportError:
    PLOTTING_AVAILABLE = False
    import statistics


class DeviceHealthMonitor:
    """Monitor device health metrics in real-time."""
    
    def __init__(self, device_ip: str, interval: float = 1.0):
        self.device_ip = device_ip
        self.base_url = f"http://{device_ip}"
        self.interval = interval
        self.running = False
        
        # Data storage (keep last 300 samples)
        self.max_samples = 300
        self.timestamps = deque(maxlen=self.max_samples)
        self.free_heap = deque(maxlen=self.max_samples)
        self.uptime = deque(maxlen=self.max_samples)
        self.response_times = deque(maxlen=self.max_samples)
        self.errors = deque(maxlen=self.max_samples)
        
        # Statistics
        self.total_requests = 0
        self.failed_requests = 0
        self.last_uptime = 0
        self.reboot_count = 0
        
    def collect_sample(self):
        """Collect a single health sample."""
        start_time = time.time()
        timestamp = datetime.now()
        
        try:
            response = requests.get(f"{self.base_url}/health", timeout=2)
            response_time = (time.time() - start_time) * 1000  # ms
            
            if response.status_code == 200:
                data = response.json()
                
                # Store data
                self.timestamps.append(timestamp)
                self.free_heap.append(data.get('free_heap', 0) / 1024)  # KB
                current_uptime = data.get('uptime_seconds', 0)
                self.uptime.append(current_uptime)
                self.response_times.append(response_time)
                self.errors.append(0)
                
                # Check for reboot
                if current_uptime < self.last_uptime:
                    self.reboot_count += 1
                    print(f"🔄 REBOOT DETECTED! Count: {self.reboot_count}")
                    self.log_event("REBOOT", f"Device rebooted. Uptime reset from {self.last_uptime}s to {current_uptime}s")
                
                self.last_uptime = current_uptime
                self.total_requests += 1
                
                # Check for low memory
                heap_kb = data.get('free_heap', 0) / 1024
                if heap_kb < 50:
                    print(f"⚠️  LOW MEMORY: {heap_kb:.1f} KB")
                    self.log_event("LOW_MEMORY", f"Free heap: {heap_kb:.1f} KB")
                    
            else:
                self.handle_error(f"HTTP {response.status_code}", timestamp, response_time)
                
        except requests.exceptions.Timeout:
            self.handle_error("Timeout", timestamp, 2000)
        except requests.exceptions.ConnectionError as e:
            self.handle_error(f"Connection failed: {str(e)}", timestamp, 0)
        except Exception as e:
            self.handle_error(f"Error: {str(e)}", timestamp, 0)
            
    def handle_error(self, error_msg: str, timestamp: datetime, response_time: float):
        """Handle collection errors."""
        self.failed_requests += 1
        self.timestamps.append(timestamp)
        self.free_heap.append(0)
        self.uptime.append(self.last_uptime)
        self.response_times.append(response_time)
        self.errors.append(1)
        
        print(f"❌ {error_msg}")
        self.log_event("ERROR", error_msg)
        
    def log_event(self, event_type: str, message: str):
        """Log important events to file."""
        with open('device_health_events.log', 'a') as f:
            f.write(f"{datetime.now().isoformat()} [{event_type}] {message}\n")
            
    def monitor_loop(self):
        """Main monitoring loop."""
        self.running = True
        while self.running:
            self.collect_sample()
            time.sleep(self.interval)
            
    def start(self):
        """Start monitoring in background thread."""
        self.thread = threading.Thread(target=self.monitor_loop)
        self.thread.daemon = True
        self.thread.start()
        
    def stop(self):
        """Stop monitoring."""
        self.running = False
        if hasattr(self, 'thread'):
            self.thread.join()
            
    def get_stats(self):
        """Get current statistics."""
        success_rate = ((self.total_requests - self.failed_requests) / self.total_requests * 100) if self.total_requests > 0 else 0
        
        return {
            'total_requests': self.total_requests,
            'failed_requests': self.failed_requests,
            'success_rate': success_rate,
            'reboot_count': self.reboot_count,
            'current_uptime': self.last_uptime,
            'avg_response_time': statistics.mean(self.response_times) if self.response_times and not PLOTTING_AVAILABLE else np.mean(self.response_times) if self.response_times and PLOTTING_AVAILABLE else 0,
            'current_heap_kb': self.free_heap[-1] if self.free_heap else 0
        }
        
    def save_data(self, filename: str = 'device_health_data.json'):
        """Save collected data to file."""
        data = {
            'device_ip': self.device_ip,
            'collection_interval': self.interval,
            'stats': self.get_stats(),
            'samples': [
                {
                    'timestamp': ts.isoformat(),
                    'heap_kb': heap,
                    'uptime': up,
                    'response_ms': rt,
                    'error': err
                }
                for ts, heap, up, rt, err in zip(
                    self.timestamps, self.free_heap, self.uptime, 
                    self.response_times, self.errors
                )
            ]
        }
        
        with open(filename, 'w') as f:
            json.dump(data, f, indent=2)
            

class HealthPlotter:
    """Real-time plotting of health metrics."""
    
    def __init__(self, monitor: DeviceHealthMonitor):
        self.monitor = monitor
        self.fig, (self.ax1, self.ax2, self.ax3) = plt.subplots(3, 1, figsize=(10, 8))
        self.fig.suptitle(f'ESP32 Health Monitor - {monitor.device_ip}')
        
        # Configure axes
        self.ax1.set_ylabel('Free Heap (KB)')
        self.ax1.grid(True)
        
        self.ax2.set_ylabel('Response Time (ms)')
        self.ax2.grid(True)
        
        self.ax3.set_ylabel('Uptime (s)')
        self.ax3.set_xlabel('Time')
        self.ax3.grid(True)
        
        # Date formatter
        self.date_fmt = DateFormatter('%H:%M:%S')
        
    def update_plot(self, frame):
        """Update plot with latest data."""
        if not self.monitor.timestamps:
            return
            
        # Convert to lists for plotting
        times = list(self.monitor.timestamps)
        heap = list(self.monitor.free_heap)
        response = list(self.monitor.response_times)
        uptime = list(self.monitor.uptime)
        errors = list(self.monitor.errors)
        
        # Clear and redraw
        self.ax1.clear()
        self.ax2.clear()
        self.ax3.clear()
        
        # Plot heap memory
        self.ax1.plot(times, heap, 'b-', label='Free Heap')
        # Mark errors
        error_times = [t for t, e in zip(times, errors) if e > 0]
        error_heap = [h for h, e in zip(heap, errors) if e > 0]
        self.ax1.plot(error_times, error_heap, 'ro', label='Errors')
        self.ax1.set_ylabel('Free Heap (KB)')
        self.ax1.legend()
        self.ax1.grid(True)
        
        # Plot response times
        self.ax2.plot(times, response, 'g-', label='Response Time')
        self.ax2.set_ylabel('Response Time (ms)')
        self.ax2.set_ylim(0, max(response + [100]))
        self.ax2.legend()
        self.ax2.grid(True)
        
        # Plot uptime
        self.ax3.plot(times, uptime, 'm-', label='Uptime')
        self.ax3.set_ylabel('Uptime (s)')
        self.ax3.set_xlabel('Time')
        self.ax3.legend()
        self.ax3.grid(True)
        
        # Format x-axis
        for ax in [self.ax1, self.ax2, self.ax3]:
            ax.xaxis.set_major_formatter(self.date_fmt)
            plt.setp(ax.xaxis.get_majorticklabels(), rotation=45)
            
        # Update title with stats
        stats = self.monitor.get_stats()
        self.fig.suptitle(
            f'ESP32 Health Monitor - {self.monitor.device_ip} | '
            f'Success: {stats["success_rate"]:.1f}% | '
            f'Reboots: {stats["reboot_count"]} | '
            f'Heap: {stats["current_heap_kb"]:.1f}KB'
        )
        
        plt.tight_layout()
        
    def start(self):
        """Start animated plotting."""
        self.ani = animation.FuncAnimation(
            self.fig, self.update_plot, interval=1000, blit=False
        )
        plt.show()


def main():
    parser = argparse.ArgumentParser(description='Monitor ESP32 device health')
    parser.add_argument('--ip', default='10.27.27.201', help='Device IP address')
    parser.add_argument('--interval', type=float, default=1.0, help='Collection interval (seconds)')
    parser.add_argument('--no-plot', action='store_true', help='Disable real-time plotting')
    parser.add_argument('--duration', type=int, help='Monitor duration (seconds)')
    
    args = parser.parse_args()
    
    # Create monitor
    monitor = DeviceHealthMonitor(args.ip, args.interval)
    
    print(f"Starting health monitor for {args.ip}")
    print("Press Ctrl+C to stop")
    
    # Start monitoring
    monitor.start()
    
    try:
        if args.no_plot or not PLOTTING_AVAILABLE:
            # Just print stats periodically
            start_time = time.time()
            while True:
                time.sleep(10)
                stats = monitor.get_stats()
                print(f"\n📊 Stats: Requests: {stats['total_requests']} | "
                      f"Success: {stats['success_rate']:.1f}% | "
                      f"Reboots: {stats['reboot_count']} | "
                      f"Heap: {stats['current_heap_kb']:.1f}KB | "
                      f"Response: {stats['avg_response_time']:.1f}ms")
                      
                if args.duration and (time.time() - start_time) > args.duration:
                    break
        else:
            # Show real-time plot
            plotter = HealthPlotter(monitor)
            plotter.start()
            
    except KeyboardInterrupt:
        print("\nStopping monitor...")
        
    finally:
        monitor.stop()
        monitor.save_data()
        print(f"Data saved to device_health_data.json")
        print(f"Events logged to device_health_events.log")
        
        # Print final stats
        stats = monitor.get_stats()
        print(f"\nFinal Statistics:")
        print(f"  Total requests: {stats['total_requests']}")
        print(f"  Failed requests: {stats['failed_requests']}")
        print(f"  Success rate: {stats['success_rate']:.1f}%")
        print(f"  Device reboots: {stats['reboot_count']}")
        print(f"  Average response: {stats['avg_response_time']:.1f}ms")


if __name__ == "__main__":
    main()