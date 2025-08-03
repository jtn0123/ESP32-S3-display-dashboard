"""Tests specifically designed to detect and diagnose device freezes"""

import pytest
import time
import requests
import threading
from typing import List, Dict, Any
from utils.base_test import ESP32TestBase as BaseTest


@pytest.mark.integration
@pytest.mark.critical
@pytest.mark.slow
class TestFreezeDetection(BaseTest):
    """Tests to detect conditions that cause device freezes"""
    
    def test_rapid_health_checks(self, tracked_request, test_context):
        """Rapid health checks without delay - tests web server overload"""
        self.log_info("Testing rapid health checks (no delay between requests)")
        
        freeze_detected = False
        last_success_time = time.time()
        successes = 0
        
        for i in range(100):
            try:
                response = tracked_request("GET", "/health", timeout=1)
                if response.status_code == 200:
                    successes += 1
                    last_success_time = time.time()
                else:
                    self.log_warning(f"Health check {i} returned {response.status_code}")
            except Exception as e:
                self.log_error(f"Health check {i} failed: {e}")
                if time.time() - last_success_time > 5:
                    freeze_detected = True
                    self.log_error(f"FREEZE DETECTED after {successes} successful requests")
                    break
        
        assert not freeze_detected, f"Device froze after {successes} rapid requests"
        assert successes > 50, f"Only {successes} successful requests out of 100"
    
    def test_memory_pressure_endpoints(self, tracked_request, test_context):
        """Test endpoints that use more memory"""
        self.log_info("Testing memory-intensive endpoints")
        
        # Get baseline memory
        response = tracked_request("GET", "/api/system")
        assert response.status_code == 200
        baseline_heap = response.json().get('free_heap', 0)
        self.log_info(f"Baseline heap: {baseline_heap:,} bytes")
        
        # Test memory-intensive operations
        memory_endpoints = [
            "/",  # Full HTML page
            "/api/metrics",  # JSON metrics
            "/api/logs",  # Log data
            "/api/config",  # Configuration
        ]
        
        for i in range(10):
            for endpoint in memory_endpoints:
                try:
                    response = tracked_request("GET", endpoint, timeout=5)
                    if response.status_code != 200:
                        self.log_warning(f"{endpoint} returned {response.status_code}")
                except Exception as e:
                    self.log_error(f"{endpoint} failed: {e}")
                    
                    # Check if we can still get system info
                    try:
                        sys_response = tracked_request("GET", "/api/system", timeout=2)
                        if sys_response.status_code == 200:
                            current_heap = sys_response.json().get('free_heap', 0)
                            self.log_error(f"Heap after failure: {current_heap:,} bytes")
                    except:
                        pass
                    
                    pytest.fail(f"Device became unresponsive at {endpoint}")
        
        # Check final memory state
        response = tracked_request("GET", "/api/system")
        if response.status_code == 200:
            final_heap = response.json().get('free_heap', 0)
            heap_loss = baseline_heap - final_heap
            self.log_info(f"Final heap: {final_heap:,} bytes (lost {heap_loss:,} bytes)")
    
    def test_concurrent_same_endpoint(self, tracked_request, test_context):
        """Test multiple concurrent requests to same endpoint"""
        self.log_info("Testing concurrent requests to same endpoint")
        
        endpoint = "/api/metrics"
        num_threads = 10
        results = []
        freeze_detected = threading.Event()
        
        def hammer_endpoint(thread_id):
            for i in range(10):
                if freeze_detected.is_set():
                    break
                    
                try:
                    response = tracked_request("GET", endpoint, timeout=3)
                    results.append((thread_id, i, response.status_code))
                except Exception as e:
                    results.append((thread_id, i, str(e)))
                    if "timeout" in str(e).lower():
                        freeze_detected.set()
        
        # Start threads
        threads = []
        for i in range(num_threads):
            t = threading.Thread(target=hammer_endpoint, args=(i,))
            t.start()
            threads.append(t)
        
        # Wait for completion
        for t in threads:
            t.join(timeout=30)
        
        # Check results
        timeouts = sum(1 for _, _, result in results if "timeout" in str(result).lower())
        successes = sum(1 for _, _, result in results if isinstance(result, int) and result == 200)
        
        self.log_info(f"Results: {successes} successes, {timeouts} timeouts out of {len(results)} requests")
        
        assert not freeze_detected.is_set(), "Device freeze detected during concurrent test"
        assert timeouts < len(results) * 0.2, f"Too many timeouts: {timeouts}/{len(results)}"
    
    def test_post_request_freeze(self, tracked_request, test_context):
        """Test if POST requests cause freezes"""
        self.log_info("Testing POST request handling")
        
        # Test config updates
        for i in range(20):
            try:
                # GET current config
                response = tracked_request("GET", "/api/config")
                assert response.status_code == 200
                
                # Try to POST update (may fail with 500)
                post_data = {
                    "brightness": 50 + i,
                    "test_value": f"test_{i}"
                }
                response = tracked_request("POST", "/api/config", json=post_data, timeout=5)
                
                # We expect this might fail, but device shouldn't freeze
                if response.status_code == 500:
                    self.log_warning(f"POST config failed as expected: {response.status_code}")
                elif response.status_code == 200:
                    self.log_info(f"POST config succeeded unexpectedly")
                    
            except Exception as e:
                self.log_error(f"Request {i} failed: {e}")
                
                # Verify device is still responsive
                try:
                    health = tracked_request("GET", "/health", timeout=2)
                    if health.status_code != 200:
                        pytest.fail(f"Device unresponsive after POST request {i}")
                except:
                    pytest.fail(f"Device frozen after POST request {i}")
            
            time.sleep(0.5)
    
    def test_network_status_spam(self, tracked_request, test_context):
        """Test rapid network status requests"""
        self.log_info("Testing network status endpoint under load")
        
        # Network endpoints might be more complex
        endpoints = [
            "/api/network/status",
            "/api/network/scan",  # This might be heavy
        ]
        
        for endpoint in endpoints:
            self.log_info(f"Testing {endpoint}")
            failures = 0
            
            for i in range(20):
                try:
                    response = tracked_request("GET", endpoint, timeout=5)
                    if response.status_code != 200:
                        failures += 1
                        self.log_warning(f"{endpoint} returned {response.status_code}")
                except Exception as e:
                    failures += 1
                    self.log_error(f"{endpoint} failed: {e}")
                    
                    if failures > 5:
                        pytest.fail(f"Too many failures on {endpoint}")
                
                time.sleep(0.2)  # Small delay
    
    def test_display_interaction_freeze(self, tracked_request, test_context):
        """Test if display-related endpoints cause issues"""
        self.log_info("Testing display-related endpoints")
        
        display_endpoints = [
            ("/api/display/settings", "GET", None),
            ("/api/display/brightness", "POST", {"brightness": 75}),
            ("/api/v1/display/screenshot", "POST", None),
        ]
        
        for endpoint, method, data in display_endpoints:
            self.log_info(f"Testing {method} {endpoint}")
            
            try:
                if method == "GET":
                    response = tracked_request("GET", endpoint, timeout=5)
                else:
                    response = tracked_request("POST", endpoint, json=data, timeout=5)
                
                self.log_info(f"{endpoint} returned {response.status_code}")
                
            except Exception as e:
                self.log_error(f"{endpoint} failed: {e}")
                
                # These endpoints might not exist, but shouldn't freeze device
                try:
                    health = tracked_request("GET", "/health", timeout=2)
                    assert health.status_code == 200, "Device unresponsive after display endpoint"
                except:
                    pytest.fail(f"Device frozen after {endpoint}")
    
    @pytest.mark.timeout(120)
    def test_long_running_stability(self, tracked_request, test_context):
        """Extended test to detect gradual degradation"""
        self.log_info("Starting 2-minute stability test")
        
        start_time = time.time()
        request_count = 0
        error_count = 0
        last_error_time = None
        response_times = []
        
        while time.time() - start_time < 120:  # 2 minutes
            try:
                req_start = time.time()
                response = tracked_request("GET", "/api/metrics", timeout=3)
                req_time = time.time() - req_start
                
                request_count += 1
                response_times.append(req_time)
                
                if response.status_code != 200:
                    error_count += 1
                    last_error_time = time.time()
                    
            except Exception as e:
                error_count += 1
                last_error_time = time.time()
                self.log_error(f"Request {request_count} failed: {e}")
                
                # Check for sustained errors
                if error_count > 5 and last_error_time:
                    if time.time() - last_error_time < 10:
                        self.log_error("Multiple errors in short time - possible freeze")
                        break
            
            time.sleep(1)  # 1 request per second
        
        # Analysis
        duration = time.time() - start_time
        success_rate = (request_count - error_count) / request_count if request_count > 0 else 0
        
        self.log_info(f"Test duration: {duration:.1f}s")
        self.log_info(f"Requests: {request_count}, Errors: {error_count}")
        self.log_info(f"Success rate: {success_rate:.1%}")
        
        if response_times:
            avg_time = sum(response_times) / len(response_times)
            max_time = max(response_times)
            self.log_info(f"Response times - Avg: {avg_time:.3f}s, Max: {max_time:.3f}s")
        
        assert success_rate > 0.95, f"Low success rate: {success_rate:.1%}"
        assert request_count > 100, f"Too few requests completed: {request_count}"