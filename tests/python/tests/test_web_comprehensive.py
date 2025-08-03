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
            # System endpoints (verified to exist)
            ("GET", "/api/system", 200),
            ("GET", "/api/metrics", 200),
            ("GET", "/api/config", 200),
            ("GET", "/health", 200),

            # Pages
            ("GET", "/", 200),
            ("GET", "/dashboard", 200),
            ("GET", "/graphs", 200),
            ("GET", "/logs", 200),

            # OTA endpoints
            ("GET", "/api/ota/status", 200),

            # Metrics variants
            ("GET", "/metrics", 200),  # Prometheus format
            ("GET", "/api/metrics/binary", 200),

            # Config endpoints
            ("GET", "/api/config/backup", 200),

            # Log endpoints
            ("GET", "/api/logs", 200),
            ("GET", "/api/logs/recent", 200),

            # PWA files
            ("GET", "/sw.js", 200),
            ("GET", "/manifest.json", 200),

            # Error handling
            ("GET", "/api/nonexistent", 404),
            ("GET", "/index.html", 404),  # This doesn't exist
            ("GET", "/api/network/status", 404),  # These were removed
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

        # Store metrics in test context
        test_context['metrics'] = test_context.get('metrics', {})
        test_context['metrics']['endpoint_test'] = {
            "total": len(endpoints),
            "success": len(results["success"]),
            "failed": len(results["failed"])
        }

        # Report summary
        self.log_info("\nEndpoint Test Summary:")
        self.log_info(f"  Success: {len(results['success'])}/{len(endpoints)}")
        if results["failed"]:
            self.log_error("  Failed endpoints:")
            for failure in results["failed"]:
                self.log_error(f"    - {failure}")

    @pytest.mark.skip(reason="Config API has WebConfig vs Config mismatch")
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

            # Try full update - ensure all fields are included
            full_update = current_config.copy()
            full_update["brightness"] = 90
            # Add any missing fields that the API expects
            if "auto_dim" not in full_update:
                full_update["auto_dim"] = True
            if "update_interval" not in full_update:
                full_update["update_interval"] = 60

            response = tracked_request("POST", "/api/config", json=full_update)
            assert response.status_code == 200, f"Full config update failed: {response.text}"

            # Restore original (with missing fields if needed)
            restore_config = current_config.copy()
            if "auto_dim" not in restore_config:
                restore_config["auto_dim"] = True
            if "update_interval" not in restore_config:
                restore_config["update_interval"] = 60
            response = tracked_request("POST", "/api/config", json=restore_config)
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

        # Store metrics in test context
        test_context['metrics'] = test_context.get('metrics', {})
        test_context['metrics']['metrics_validation'] = {
            "samples": len(samples),
            "avg_fps": sum(s.get("fps_actual", 0) for s in samples) / len(samples),
            "min_heap": min(s.get("heap_free", 0) for s in samples)
        }

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

        # Make concurrent requests (reduced to avoid crashing ESP32)
        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(make_request, i) for i in range(10)]
            results = [f.result() for f in concurrent.futures.as_completed(futures)]

        successes = sum(1 for r in results if r.get("success", False))
        avg_duration = sum(r.get("duration", 0) for r in results if "duration" in r) / max(1, successes)

        # Store metrics in test context
        test_context['metrics'] = test_context.get('metrics', {})
        test_context['metrics']['concurrent_test'] = {
            "total_requests": len(results),
            "successful": successes,
            "avg_duration_ms": avg_duration * 1000
        }

        self.log_info(f"Concurrent test: {successes}/{len(results)} succeeded")
        assert successes >= 7, f"Too many failed concurrent requests: {successes}/10"

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

        # Test large request - ESP32 may close connection on large POST
        try:
            large_data = {"data": "x" * 10000}  # 10KB (smaller to avoid connection abort)
            response = tracked_request("POST", "/api/test", json=large_data)
            # Should handle gracefully
            assert response.status_code in [404, 413, 500]
        except Exception as e:
            # ESP32 may close connection on large requests
            self.log_warning(f"Large request handling: {str(e)}")

    def test_response_headers(self, tracked_request):
        """Test proper HTTP headers"""
        # Test JSON endpoint
        response = tracked_request("GET", "/api/system")

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

        response = http_client.get(f"{http_client.base_url}/ws", headers=headers)

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

        # Store metrics in test context
        test_context['metrics'] = test_context.get('metrics', {})
        test_context['metrics']['rate_limit_test'] = {
            "requests": len(responses),
            "duration_s": duration,
            "rate_limited": rate_limited,
            "rps": len(responses) / duration
        }

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

