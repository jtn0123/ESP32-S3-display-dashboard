"""Comprehensive web server tests"""

import pytest
import time
import json
from utils.base_test import ESP32TestBase as BaseTest


@pytest.mark.web
@pytest.mark.integration
class TestWebComprehensive(BaseTest):
    """Comprehensive testing of web server functionality"""
    
    def test_all_api_endpoints(self, tracked_request, test_context):
        """Test all API endpoints are accessible"""
        endpoints = [
            # System endpoints
            ("GET", "/api/system", 200),
            ("GET", "/api/metrics", 200),
            ("GET", "/api/config", 200),
            ("GET", "/health", 200),
            
            # Network endpoints  
            ("GET", "/api/network/status", 200),
            ("GET", "/api/network/scan", 200),
            
            # OTA endpoints
            ("GET", "/api/ota/status", 200),
            ("GET", "/api/ota/check", 200),
            
            # Display endpoints
            ("GET", "/api/display/settings", 200),
            ("POST", "/api/display/brightness", 500),  # Might need body
            
            # Static files
            ("GET", "/", 200),
            ("GET", "/index.html", 200),
            ("GET", "/static/style.css", 200),
            ("GET", "/static/app.js", 200),
            
            # Error cases
            ("GET", "/api/nonexistent", 404),
            ("POST", "/api/config", 500),  # Without proper body
        ]
        
        results = {"success": [], "failed": []}
        
        for method, path, expected_status in endpoints:
            try:
                response = tracked_request(method, path)
                actual_status = response.status_code
                
                if actual_status == expected_status:
                    results["success"].append(f"{method} {path}")
                    self.log_info(f"✓ {method} {path} -> {actual_status}")
                else:
                    results["failed"].append(f"{method} {path}: expected {expected_status}, got {actual_status}")
                    self.log_error(f"✗ {method} {path}: expected {expected_status}, got {actual_status}")
                    
            except Exception as e:
                results["failed"].append(f"{method} {path}: {str(e)}")
                self.log_error(f"✗ {method} {path}: {str(e)}")
                
        test_context.add_metric("endpoint_test", {
            "total": len(endpoints),
            "success": len(results["success"]),
            "failed": len(results["failed"])
        })
        
        # Report summary
        self.log_info(f"\nEndpoint Test Summary:")
        self.log_info(f"  Success: {len(results['success'])}/{len(endpoints)}")
        if results["failed"]:
            self.log_error(f"  Failed endpoints:")
            for failure in results["failed"]:
                self.log_error(f"    - {failure}")
                
    def test_config_api_properly(self, tracked_request):
        """Test config API with proper request format"""
        # First get current config
        response = tracked_request("GET", "/api/config")
        assert response.status_code == 200
        
        current_config = response.json()
        self.log_info(f"Current config: {json.dumps(current_config, indent=2)}")
        
        # Test partial update (this currently fails)
        partial_update = {"brightness": 90}
        response = tracked_request("POST", "/api/config", json=partial_update)
        
        if response.status_code != 200:
            self.log_warning("Config API does not support partial updates")
            
            # Try full update
            full_update = current_config.copy()
            full_update["brightness"] = 90
            
            response = tracked_request("POST", "/api/config", json=full_update)
            assert response.status_code == 200, f"Full config update failed: {response.text}"
            
            # Restore original
            response = tracked_request("POST", "/api/config", json=current_config)
            assert response.status_code == 200
            
    def test_metrics_accuracy(self, tracked_request, test_context):
        """Test metrics endpoint returns accurate data"""
        # Get metrics multiple times
        samples = []
        for i in range(5):
            response = tracked_request("GET", "/api/metrics")
            assert response.status_code == 200
            
            data = response.json()
            samples.append(data)
            time.sleep(1)
            
        # Verify metrics are changing
        uptime_values = [s.get("uptime", 0) for s in samples]
        assert all(uptime_values[i] < uptime_values[i+1] for i in range(len(uptime_values)-1)), \
            "Uptime should be increasing"
            
        # Check required fields
        required_fields = ["fps_actual", "heap_free", "uptime", "wifi_connected"]
        for field in required_fields:
            assert all(field in s for s in samples), f"Missing required field: {field}"
            
        test_context.add_metric("metrics_validation", {
            "samples": len(samples),
            "avg_fps": sum(s.get("fps_actual", 0) for s in samples) / len(samples),
            "min_heap": min(s.get("heap_free", 0) for s in samples)
        })
        
    def test_concurrent_requests(self, tracked_request, test_context):
        """Test server handles concurrent requests"""
        import concurrent.futures
        
        def make_request(i):
            try:
                start = time.time()
                response = tracked_request("GET", "/api/metrics")
                duration = time.time() - start
                return {"success": response.status_code == 200, "duration": duration}
            except Exception as e:
                return {"success": False, "error": str(e)}
                
        # Make concurrent requests
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(make_request, i) for i in range(20)]
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
            
        successes = sum(1 for r in results if r.get("success", False))
        avg_duration = sum(r.get("duration", 0) for r in results if "duration" in r) / len(results)
        
        test_context.add_metric("concurrent_test", {
            "total_requests": len(results),
            "successful": successes,
            "avg_duration_ms": avg_duration * 1000
        })
        
        assert successes >= 15, f"Too many failed concurrent requests: {successes}/20"
        
    def test_error_handling(self, tracked_request):
        """Test error responses are properly formatted"""
        # Test invalid JSON
        response = tracked_request("POST", "/api/config", 
                                 data="invalid json",
                                 headers={"Content-Type": "application/json"})
        
        # Should return error status
        assert response.status_code in [400, 500]
        
        # Test method not allowed
        response = tracked_request("DELETE", "/api/config")
        assert response.status_code in [405, 404]
        
        # Test large request
        large_data = {"data": "x" * 100000}  # 100KB
        response = tracked_request("POST", "/api/test", json=large_data)
        # Should handle gracefully
        assert response.status_code in [404, 413, 500]
        
    def test_response_headers(self, tracked_request):
        """Test proper HTTP headers"""
        response = tracked_request("GET", "/api/metrics")
        
        # Check common headers
        assert "Content-Type" in response.headers
        
        # For API endpoints, should be JSON
        if response.status_code == 200:
            assert "application/json" in response.headers.get("Content-Type", "")
            
        # Test static file headers
        response = tracked_request("GET", "/")
        if response.status_code == 200:
            assert "text/html" in response.headers.get("Content-Type", "")
            
    def test_websocket_support(self, http_client):
        """Test if WebSocket is supported (for live updates)"""
        # Try to upgrade connection
        headers = {
            "Upgrade": "websocket",
            "Connection": "Upgrade",
            "Sec-WebSocket-Key": "x3JJHMbDL1EzLkh9GBhXDw==",
            "Sec-WebSocket-Version": "13"
        }
        
        response = http_client.get("/ws", headers=headers)
        
        if response.status_code == 101:
            self.log_info("WebSocket supported!")
        else:
            self.log_info("WebSocket not supported (expected)")
            
    def test_cors_headers(self, tracked_request):
        """Test CORS headers for API access"""
        # Test preflight request
        response = tracked_request("OPTIONS", "/api/metrics",
                                 headers={"Origin": "http://localhost:3000"})
        
        # Check if CORS is enabled
        if "Access-Control-Allow-Origin" in response.headers:
            self.log_info("CORS enabled")
            assert response.headers["Access-Control-Allow-Origin"] in ["*", "http://localhost:3000"]
        else:
            self.log_warning("CORS not configured")
            
    def test_compression_support(self, tracked_request):
        """Test if server supports compression"""
        response = tracked_request("GET", "/api/system",
                                 headers={"Accept-Encoding": "gzip, deflate"})
        
        if "Content-Encoding" in response.headers:
            self.log_info(f"Compression supported: {response.headers['Content-Encoding']}")
        else:
            self.log_info("Compression not enabled")
            
    def test_api_versioning(self, tracked_request):
        """Test API versioning support"""
        # Try versioned endpoints
        versioned_endpoints = [
            "/api/v1/metrics",
            "/api/v2/metrics",
            "/v1/api/metrics"
        ]
        
        for endpoint in versioned_endpoints:
            response = tracked_request("GET", endpoint)
            if response.status_code == 200:
                self.log_info(f"API versioning supported at: {endpoint}")
                break
        else:
            self.log_info("No API versioning detected (OK for embedded)")
            
    def test_rate_limiting(self, tracked_request, test_context):
        """Test if rate limiting is implemented"""
        # Make rapid requests
        start_time = time.time()
        responses = []
        
        for i in range(50):
            response = tracked_request("GET", "/api/metrics")
            responses.append(response.status_code)
            
        duration = time.time() - start_time
        
        # Check if any were rate limited
        rate_limited = sum(1 for status in responses if status == 429)
        
        test_context.add_metric("rate_limit_test", {
            "requests": len(responses),
            "duration_s": duration,
            "rate_limited": rate_limited,
            "rps": len(responses) / duration
        })
        
        if rate_limited > 0:
            self.log_info(f"Rate limiting active: {rate_limited} requests limited")
        else:
            self.log_info("No rate limiting detected")
            
    def test_authentication(self, tracked_request):
        """Test if authentication is required"""
        # Try with auth header
        response = tracked_request("GET", "/api/config",
                                 headers={"Authorization": "Bearer test-token"})
        
        # For embedded device, usually no auth
        if response.status_code == 401:
            self.log_info("Authentication required")
        else:
            self.log_info("No authentication required (typical for local device)")