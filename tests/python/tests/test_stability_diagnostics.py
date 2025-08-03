#!/usr/bin/env python3
"""
Stability diagnostic tests for ESP32-S3 Dashboard.
These tests help identify what causes device crashes and instability.
"""

import time
import json
import logging
import pytest
import requests
from typing import Dict, List, Optional, Tuple
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class StabilityDiagnostics:
    """Diagnostic tests to identify stability issues."""
    
    def __init__(self, device_ip: str, timeout: int = 5):
        self.device_ip = device_ip
        self.base_url = f"http://{device_ip}"
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({'Connection': 'close'})
        
    def get_memory_stats(self) -> Optional[Dict]:
        """Get current memory statistics from device."""
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=self.timeout)
            if response.status_code == 200:
                data = response.json()
                return {
                    'free_heap': data.get('free_heap', 0),
                    'timestamp': datetime.now().isoformat(),
                    'uptime': data.get('uptime_seconds', 0)
                }
        except Exception as e:
            logger.error(f"Failed to get memory stats: {e}")
        return None
    
    def check_endpoint_impact(self, endpoint: str, method: str = "GET", 
                            data: Optional[Dict] = None) -> Dict:
        """Check memory impact of accessing a specific endpoint."""
        result = {
            'endpoint': endpoint,
            'method': method,
            'memory_before': None,
            'memory_after': None,
            'memory_leaked': None,
            'response_time': None,
            'status_code': None,
            'error': None,
            'caused_crash': False
        }
        
        # Get memory before
        before = self.get_memory_stats()
        if not before:
            result['error'] = "Failed to get initial memory stats"
            return result
        
        result['memory_before'] = before['free_heap']
        
        # Make request
        start_time = time.time()
        try:
            if method == "GET":
                response = self.session.get(f"{self.base_url}{endpoint}", timeout=self.timeout)
            elif method == "POST":
                response = self.session.post(f"{self.base_url}{endpoint}", 
                                           json=data, timeout=self.timeout)
            else:
                raise ValueError(f"Unsupported method: {method}")
                
            result['response_time'] = time.time() - start_time
            result['status_code'] = response.status_code
            
            # Wait a bit for memory to settle
            time.sleep(0.5)
            
            # Get memory after
            after = self.get_memory_stats()
            if after:
                result['memory_after'] = after['free_heap']
                result['memory_leaked'] = result['memory_before'] - result['memory_after']
            else:
                result['error'] = "Failed to get memory stats after request"
                result['caused_crash'] = True
                
        except requests.exceptions.Timeout:
            result['error'] = "Request timed out"
            result['response_time'] = self.timeout
            # Check if device is still alive
            if not self.get_memory_stats():
                result['caused_crash'] = True
        except requests.exceptions.ConnectionError as e:
            result['error'] = f"Connection error: {str(e)}"
            result['caused_crash'] = True
        except Exception as e:
            result['error'] = f"Unexpected error: {str(e)}"
            
        return result
    
    def test_request_rate_limit(self, endpoint: str, max_rate: int = 10, 
                               duration: int = 10) -> Dict:
        """Test how many requests per second the device can handle."""
        results = {
            'endpoint': endpoint,
            'max_rate': max_rate,
            'duration': duration,
            'successful_requests': 0,
            'failed_requests': 0,
            'timeouts': 0,
            'connection_errors': 0,
            'average_response_time': 0,
            'crash_detected': False,
            'max_concurrent': 0
        }
        
        interval = 1.0 / max_rate
        response_times = []
        start_time = time.time()
        
        while time.time() - start_time < duration:
            try:
                req_start = time.time()
                response = self.session.get(f"{self.base_url}{endpoint}", 
                                          timeout=self.timeout)
                req_time = time.time() - req_start
                
                if response.status_code == 200:
                    results['successful_requests'] += 1
                    response_times.append(req_time)
                else:
                    results['failed_requests'] += 1
                    
            except requests.exceptions.Timeout:
                results['timeouts'] += 1
            except requests.exceptions.ConnectionError:
                results['connection_errors'] += 1
                # Check if device crashed
                time.sleep(2)
                if not self.get_memory_stats():
                    results['crash_detected'] = True
                    break
                    
            # Rate limit
            time.sleep(interval)
            
        if response_times:
            results['average_response_time'] = sum(response_times) / len(response_times)
            
        return results
    
    def test_concurrent_connections(self, endpoint: str, 
                                  max_concurrent: int = 10) -> Dict:
        """Test how many concurrent connections the device can handle."""
        results = {
            'endpoint': endpoint,
            'max_concurrent': max_concurrent,
            'successful': 0,
            'failed': 0,
            'crash_detected': False,
            'memory_before': None,
            'memory_after': None
        }
        
        # Get initial memory
        before = self.get_memory_stats()
        if before:
            results['memory_before'] = before['free_heap']
        
        def make_request():
            try:
                response = requests.get(f"{self.base_url}{endpoint}", 
                                      timeout=self.timeout,
                                      headers={'Connection': 'close'})
                return response.status_code == 200
            except:
                return False
        
        # Test concurrent connections
        with ThreadPoolExecutor(max_workers=max_concurrent) as executor:
            futures = [executor.submit(make_request) for _ in range(max_concurrent)]
            
            for future in as_completed(futures):
                if future.result():
                    results['successful'] += 1
                else:
                    results['failed'] += 1
        
        # Wait and check if device is still alive
        time.sleep(2)
        after = self.get_memory_stats()
        if after:
            results['memory_after'] = after['free_heap']
        else:
            results['crash_detected'] = True
            
        return results
    
    def test_payload_size_limit(self, endpoint: str = "/api/config",
                              start_size: int = 100,
                              max_size: int = 10000,
                              step: int = 500) -> List[Dict]:
        """Test how large payloads the device can handle."""
        results = []
        
        for size in range(start_size, max_size + 1, step):
            # Create payload of specific size
            payload = {
                'test_data': 'x' * size,
                'size': size
            }
            
            result = {
                'payload_size': size,
                'success': False,
                'response_time': None,
                'error': None,
                'memory_impact': None
            }
            
            before = self.get_memory_stats()
            if not before:
                result['error'] = "Failed to get initial memory"
                results.append(result)
                break
                
            try:
                start_time = time.time()
                response = self.session.post(f"{self.base_url}{endpoint}",
                                           json=payload,
                                           timeout=self.timeout)
                result['response_time'] = time.time() - start_time
                result['success'] = response.status_code in [200, 201, 400]
                
                # Check memory impact
                time.sleep(0.5)
                after = self.get_memory_stats()
                if after:
                    result['memory_impact'] = before['free_heap'] - after['free_heap']
                    
            except Exception as e:
                result['error'] = str(e)
                # Check if device crashed
                if not self.get_memory_stats():
                    result['error'] = "Device crashed"
                    results.append(result)
                    break
                    
            results.append(result)
            time.sleep(1)  # Give device time to recover
            
        return results


@pytest.fixture
def diagnostics(request):
    """Create diagnostics instance."""
    device_ip = request.config.getoption("--device-ip")
    return StabilityDiagnostics(device_ip)


class TestStabilityDiagnostics:
    """Stability diagnostic test cases."""
    
    def test_endpoint_memory_impact(self, diagnostics):
        """Test memory impact of each endpoint."""
        endpoints = [
            ("/health", "GET"),
            ("/api/system", "GET"),
            ("/api/metrics", "GET"),
            ("/api/config", "GET"),
            ("/", "GET"),
            ("/dashboard", "GET"),
            ("/logs", "GET"),
            ("/ota", "GET"),
        ]
        
        results = []
        for endpoint, method in endpoints:
            logger.info(f"Testing {method} {endpoint}")
            result = diagnostics.check_endpoint_impact(endpoint, method)
            results.append(result)
            
            # Log results
            if result['memory_leaked'] and result['memory_leaked'] > 1000:
                logger.warning(f"{endpoint} leaked {result['memory_leaked']} bytes")
            if result['caused_crash']:
                logger.error(f"{endpoint} caused device crash!")
                break
                
            # Wait between tests
            time.sleep(3)
        
        # Save results
        with open('endpoint_memory_impact.json', 'w') as f:
            json.dump(results, f, indent=2)
            
        # Check which endpoints are problematic
        problematic = [r for r in results if r['memory_leaked'] and r['memory_leaked'] > 5000]
        logger.info(f"Problematic endpoints: {[p['endpoint'] for p in problematic]}")
        
    def test_request_rates(self, diagnostics):
        """Test different request rates for key endpoints."""
        test_cases = [
            ("/health", 10, 10),      # 10 req/s for 10s
            ("/health", 5, 10),       # 5 req/s for 10s
            ("/health", 2, 10),       # 2 req/s for 10s
            ("/api/metrics", 5, 10),  # 5 req/s for 10s
            ("/api/metrics", 2, 10),  # 2 req/s for 10s
            ("/api/metrics", 1, 10),  # 1 req/s for 10s
        ]
        
        results = []
        for endpoint, rate, duration in test_cases:
            logger.info(f"Testing {endpoint} at {rate} req/s for {duration}s")
            result = diagnostics.test_request_rate_limit(endpoint, rate, duration)
            results.append(result)
            
            if result['crash_detected']:
                logger.error(f"Device crashed at {rate} req/s on {endpoint}")
                break
                
            # Recovery time
            time.sleep(5)
            
        # Save results
        with open('request_rate_limits.json', 'w') as f:
            json.dump(results, f, indent=2)
            
    def test_concurrent_limits(self, diagnostics):
        """Test concurrent connection limits."""
        test_cases = [
            ("/health", 2),
            ("/health", 4),
            ("/health", 6),
            ("/api/metrics", 2),
            ("/api/metrics", 4),
        ]
        
        results = []
        for endpoint, concurrent in test_cases:
            logger.info(f"Testing {concurrent} concurrent connections to {endpoint}")
            result = diagnostics.test_concurrent_connections(endpoint, concurrent)
            results.append(result)
            
            if result['crash_detected']:
                logger.error(f"Device crashed with {concurrent} concurrent connections")
                break
                
            # Recovery time
            time.sleep(5)
            
        # Save results
        with open('concurrent_limits.json', 'w') as f:
            json.dump(results, f, indent=2)
            
    def test_payload_limits(self, diagnostics):
        """Test payload size limits."""
        logger.info("Testing payload size limits")
        results = diagnostics.test_payload_size_limit(
            endpoint="/api/config",
            start_size=100,
            max_size=5000,
            step=500
        )
        
        # Save results
        with open('payload_limits.json', 'w') as f:
            json.dump(results, f, indent=2)
            
        # Find breaking point
        for result in results:
            if result.get('error') and 'crashed' in result['error']:
                logger.error(f"Device crashed at payload size: {result['payload_size']}")
                break
                
    def test_memory_leak_detection(self, diagnostics):
        """Test for memory leaks over repeated requests."""
        endpoint = "/api/metrics"
        iterations = 20
        
        memory_samples = []
        for i in range(iterations):
            stats = diagnostics.get_memory_stats()
            if not stats:
                logger.error(f"Failed to get memory stats at iteration {i}")
                break
                
            memory_samples.append(stats)
            
            # Make a request
            try:
                response = diagnostics.session.get(f"{diagnostics.base_url}{endpoint}",
                                                 timeout=diagnostics.timeout)
            except:
                logger.error(f"Request failed at iteration {i}")
                break
                
            time.sleep(2)
            
        # Analyze memory trend
        if len(memory_samples) > 1:
            first_heap = memory_samples[0]['free_heap']
            last_heap = memory_samples[-1]['free_heap']
            leak_per_request = (first_heap - last_heap) / len(memory_samples)
            
            logger.info(f"Memory leak analysis:")
            logger.info(f"  Initial heap: {first_heap}")
            logger.info(f"  Final heap: {last_heap}")
            logger.info(f"  Total leaked: {first_heap - last_heap}")
            logger.info(f"  Leak per request: {leak_per_request:.2f} bytes")
            
        # Save detailed results
        with open('memory_leak_analysis.json', 'w') as f:
            json.dump({
                'endpoint': endpoint,
                'iterations': iterations,
                'samples': memory_samples,
                'leak_detected': leak_per_request > 100 if 'leak_per_request' in locals() else None
            }, f, indent=2)


if __name__ == "__main__":
    # Can be run standalone for quick diagnostics
    import sys
    if len(sys.argv) > 1:
        device_ip = sys.argv[1]
    else:
        device_ip = "10.27.27.201"
        
    diag = StabilityDiagnostics(device_ip)
    
    print("Running quick diagnostic...")
    result = diag.check_endpoint_impact("/api/metrics")
    print(json.dumps(result, indent=2))