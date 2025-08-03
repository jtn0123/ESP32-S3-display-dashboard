"""Device stability and freeze detection tests"""

import pytest
import time
import requests
import threading
import queue
from typing import List, Dict, Any
from utils.base_test import ESP32TestBase as BaseTest


@pytest.mark.integration
@pytest.mark.critical
class TestDeviceStability(BaseTest):
    """Tests to detect device freezes and stability issues"""
    
    def test_device_heartbeat_monitoring(self, tracked_request, test_context):
        """Monitor device heartbeat to detect freezes"""
        self.log_info("Starting heartbeat monitoring test")
        
        # Configuration
        check_interval = 2  # seconds
        test_duration = 30  # seconds
        max_response_time = 5  # seconds
        
        start_time = time.time()
        failures = []
        response_times = []
        
        while time.time() - start_time < test_duration:
            try:
                check_start = time.time()
                response = tracked_request("GET", "/health", timeout=max_response_time)
                response_time = time.time() - check_start
                
                if response.status_code != 200:
                    failures.append({
                        'time': time.time() - start_time,
                        'error': f'HTTP {response.status_code}',
                        'response_time': response_time
                    })
                else:
                    response_times.append(response_time)
                    
                # Check if response time is increasing (sign of impending freeze)
                if response_time > 2.0:
                    self.log_warning(f"Slow response detected: {response_time:.2f}s")
                    
            except requests.exceptions.Timeout:
                failures.append({
                    'time': time.time() - start_time,
                    'error': 'Timeout',
                    'response_time': max_response_time
                })
                self.log_error(f"Device timeout at {time.time() - start_time:.1f}s")
                
            except Exception as e:
                failures.append({
                    'time': time.time() - start_time,
                    'error': str(e),
                    'response_time': None
                })
                self.log_error(f"Device error: {e}")
            
            time.sleep(check_interval)
        
        # Analysis
        if failures:
            self.log_error(f"Device stability issues detected: {len(failures)} failures")
            for failure in failures:
                self.log_error(f"  - At {failure['time']:.1f}s: {failure['error']}")
        
        if response_times:
            avg_response = sum(response_times) / len(response_times)
            max_response = max(response_times)
            self.log_info(f"Response times - Avg: {avg_response:.3f}s, Max: {max_response:.3f}s")
            
            # Detect gradual slowdown
            if len(response_times) > 10:
                first_half = response_times[:len(response_times)//2]
                second_half = response_times[len(response_times)//2:]
                first_avg = sum(first_half) / len(first_half)
                second_avg = sum(second_half) / len(second_half)
                
                if second_avg > first_avg * 1.5:
                    self.log_warning(f"Performance degradation detected: {first_avg:.3f}s -> {second_avg:.3f}s")
        
        assert len(failures) == 0, f"Device stability test failed with {len(failures)} errors"
    
    def test_concurrent_request_stability(self, tracked_request, test_context):
        """Test device stability under concurrent requests"""
        self.log_info("Testing concurrent request handling")
        
        num_threads = 5
        requests_per_thread = 20
        results = queue.Queue()
        
        def make_requests(thread_id):
            thread_results = {'success': 0, 'failed': 0, 'errors': []}
            
            for i in range(requests_per_thread):
                try:
                    response = tracked_request("GET", f"/api/metrics?thread={thread_id}&req={i}", 
                                             timeout=10)
                    if response.status_code == 200:
                        thread_results['success'] += 1
                    else:
                        thread_results['failed'] += 1
                        thread_results['errors'].append(f"HTTP {response.status_code}")
                except Exception as e:
                    thread_results['failed'] += 1
                    thread_results['errors'].append(str(e))
                
                time.sleep(0.1)  # Small delay between requests
            
            results.put((thread_id, thread_results))
        
        # Start threads
        threads = []
        start_time = time.time()
        
        for i in range(num_threads):
            t = threading.Thread(target=make_requests, args=(i,))
            t.start()
            threads.append(t)
        
        # Wait for completion
        for t in threads:
            t.join(timeout=60)
        
        duration = time.time() - start_time
        
        # Collect results
        total_success = 0
        total_failed = 0
        all_errors = []
        
        while not results.empty():
            thread_id, thread_results = results.get()
            total_success += thread_results['success']
            total_failed += thread_results['failed']
            all_errors.extend(thread_results['errors'])
            
            if thread_results['errors']:
                self.log_warning(f"Thread {thread_id} had {len(thread_results['errors'])} errors")
        
        self.log_info(f"Concurrent test completed in {duration:.1f}s")
        self.log_info(f"Success: {total_success}, Failed: {total_failed}")
        
        # Allow some failures but not complete freeze
        failure_rate = total_failed / (total_success + total_failed) if (total_success + total_failed) > 0 else 1.0
        assert failure_rate < 0.5, f"High failure rate: {failure_rate:.1%}"
        assert total_success > 0, "No successful requests - device may be frozen"
    
    def test_memory_leak_detection(self, tracked_request, test_context):
        """Detect memory leaks that could cause freezes"""
        self.log_info("Starting memory leak detection")
        
        # Get initial memory state
        response = tracked_request("GET", "/api/system")
        assert response.status_code == 200
        initial_data = response.json()
        initial_heap = initial_data.get('free_heap', 0)
        
        self.log_info(f"Initial free heap: {initial_heap} bytes")
        
        # Perform repeated operations
        heap_samples = [initial_heap]
        
        for i in range(20):
            # Make various requests to exercise different code paths
            tracked_request("GET", "/")
            tracked_request("GET", "/api/config")
            tracked_request("GET", "/api/metrics")
            tracked_request("POST", "/api/config", json={"test": f"value_{i}"})
            
            # Check memory
            response = tracked_request("GET", "/api/system")
            if response.status_code == 200:
                data = response.json()
                current_heap = data.get('free_heap', 0)
                heap_samples.append(current_heap)
                
                # Check for sudden drops
                if current_heap < initial_heap * 0.5:
                    self.log_error(f"Significant heap loss: {initial_heap} -> {current_heap}")
            
            time.sleep(1)
        
        # Analyze trend
        if len(heap_samples) > 5:
            # Calculate average change
            heap_changes = [heap_samples[i+1] - heap_samples[i] 
                           for i in range(len(heap_samples)-1)]
            avg_change = sum(heap_changes) / len(heap_changes)
            
            self.log_info(f"Average heap change per iteration: {avg_change:.0f} bytes")
            
            # Check for consistent decrease (memory leak)
            decreasing_count = sum(1 for change in heap_changes if change < -1000)
            if decreasing_count > len(heap_changes) * 0.7:
                self.log_warning(f"Possible memory leak detected: {decreasing_count}/{len(heap_changes)} decreases")
            
            # Check final vs initial
            final_heap = heap_samples[-1]
            heap_loss = initial_heap - final_heap
            loss_percentage = (heap_loss / initial_heap) * 100
            
            self.log_info(f"Total heap loss: {heap_loss} bytes ({loss_percentage:.1f}%)")
            assert loss_percentage < 20, f"Excessive memory loss: {loss_percentage:.1f}%"
    
    def test_watchdog_functionality(self, tracked_request, test_context):
        """Test if watchdog is properly resetting frozen tasks"""
        self.log_info("Testing watchdog functionality")
        
        # Get initial uptime
        response = tracked_request("GET", "/api/system")
        assert response.status_code == 200
        initial_uptime = response.json().get('uptime_seconds', 0)
        
        # Monitor for resets
        uptimes = [initial_uptime]
        resets_detected = 0
        
        for i in range(10):
            time.sleep(3)
            
            try:
                response = tracked_request("GET", "/api/system", timeout=5)
                if response.status_code == 200:
                    current_uptime = response.json().get('uptime_seconds', 0)
                    uptimes.append(current_uptime)
                    
                    # Check if uptime decreased (indicates reset)
                    if current_uptime < uptimes[-2]:
                        resets_detected += 1
                        self.log_warning(f"Device reset detected at iteration {i}")
                        self.log_warning(f"Uptime went from {uptimes[-2]}s to {current_uptime}s")
            except Exception as e:
                self.log_error(f"Failed to check uptime: {e}")
        
        self.log_info(f"Watchdog test complete. Resets detected: {resets_detected}")
        assert resets_detected == 0, f"Device reset {resets_detected} times during test"
    
    def test_network_disconnect_recovery(self, tracked_request, test_context):
        """Test device behavior during network issues"""
        self.log_info("Testing network disconnect recovery")
        
        # Baseline check
        response = tracked_request("GET", "/health")
        assert response.status_code == 200
        
        # Simulate rapid connect/disconnect
        failures = []
        recoveries = []
        
        for i in range(5):
            # Make rapid requests
            for j in range(10):
                try:
                    response = tracked_request("GET", "/health", timeout=1)
                    if response.status_code != 200:
                        failures.append((i, j, response.status_code))
                except Exception as e:
                    failures.append((i, j, str(e)))
            
            # Brief pause
            time.sleep(2)
            
            # Check recovery
            try:
                response = tracked_request("GET", "/health", timeout=5)
                if response.status_code == 200:
                    recoveries.append(i)
                    self.log_info(f"Recovery confirmed after round {i}")
            except:
                self.log_error(f"Failed to recover after round {i}")
        
        self.log_info(f"Network test: {len(failures)} failures, {len(recoveries)} recoveries")
        assert len(recoveries) >= 4, "Device failed to recover from network stress"
    
    def test_continuous_operation(self, tracked_request, test_context):
        """Long-running test to detect gradual degradation"""
        self.log_info("Starting continuous operation test (60 seconds)")
        
        metrics = {
            'start_time': time.time(),
            'requests': 0,
            'successes': 0,
            'failures': 0,
            'response_times': [],
            'errors': []
        }
        
        test_duration = 60  # 1 minute continuous operation
        
        while time.time() - metrics['start_time'] < test_duration:
            try:
                start = time.time()
                response = tracked_request("GET", "/api/metrics", timeout=5)
                response_time = time.time() - start
                
                metrics['requests'] += 1
                metrics['response_times'].append(response_time)
                
                if response.status_code == 200:
                    metrics['successes'] += 1
                    
                    # Parse metrics to check device health
                    data = response.json()
                    if 'temperature' in data and data['temperature'] > 80:
                        self.log_warning(f"High temperature detected: {data['temperature']}°C")
                else:
                    metrics['failures'] += 1
                    metrics['errors'].append(f"HTTP {response.status_code}")
                    
            except Exception as e:
                metrics['failures'] += 1
                metrics['errors'].append(str(e))
                self.log_error(f"Request failed: {e}")
            
            time.sleep(0.5)  # 2 requests per second
        
        # Analysis
        duration = time.time() - metrics['start_time']
        success_rate = metrics['successes'] / metrics['requests'] if metrics['requests'] > 0 else 0
        
        self.log_info(f"Continuous operation test completed:")
        self.log_info(f"  Duration: {duration:.1f}s")
        self.log_info(f"  Requests: {metrics['requests']}")
        self.log_info(f"  Success rate: {success_rate:.1%}")
        
        if metrics['response_times']:
            avg_response = sum(metrics['response_times']) / len(metrics['response_times'])
            max_response = max(metrics['response_times'])
            self.log_info(f"  Avg response: {avg_response:.3f}s")
            self.log_info(f"  Max response: {max_response:.3f}s")
        
        # Check for degradation over time
        if len(metrics['response_times']) > 20:
            first_quarter = metrics['response_times'][:len(metrics['response_times'])//4]
            last_quarter = metrics['response_times'][3*len(metrics['response_times'])//4:]
            
            first_avg = sum(first_quarter) / len(first_quarter)
            last_avg = sum(last_quarter) / len(last_quarter)
            
            degradation = (last_avg - first_avg) / first_avg
            self.log_info(f"  Performance change: {degradation:+.1%}")
            
            if degradation > 0.5:  # 50% slower
                self.log_warning("Significant performance degradation detected")
        
        assert success_rate > 0.95, f"Low success rate: {success_rate:.1%}"
        assert metrics['requests'] > 100, f"Too few requests completed: {metrics['requests']}"