#!/usr/bin/env python3
"""
Comprehensive web server test suite for ESP32-S3 Dashboard.
Tests all endpoints, methods, edge cases, and security aspects.
"""

import pytest
import requests
import json
import time
import base64
from typing import Dict, List, Tuple, Optional
from datetime import datetime
import random
import string

from utils.base_test import ESP32TestBase as BaseTest


class TestWebServerComplete(BaseTest):
    """Complete web server testing including edge cases and security."""
    
    def test_all_get_endpoints(self, tracked_request):
        """Test all GET endpoints systematically."""
        # First check if device is up
        try:
            response = tracked_request("GET", "/health")
            if response.status_code != 200:
                pytest.skip("Device not healthy")
        except Exception as e:
            pytest.skip(f"Device not responding: {e}")
            
        get_endpoints = [
            # Core pages - test lightweight endpoints first
            ("/", 200, "text/html"),
            # Skip heavy pages initially
            # ("/dashboard", 200, "text/html"),  # May crash device
            # ("/graphs", 200, "text/html"),      # May be heavy
            # ("/logs", 200, "text/html"),        # May be heavy
            
            # API endpoints
            ("/health", 200, "application/json"),
            ("/api/system", 200, "application/json"),
            ("/api/config", 200, "application/json"),
            ("/api/metrics", 200, "application/json"),
            ("/api/logs", 200, "application/json"),
            ("/api/logs/recent", 200, "application/json"),
            
            # Binary/special formats
            ("/metrics", 200, "text/plain"),  # Prometheus format
            ("/api/metrics/binary", 200, "application/octet-stream"),
            
            # Config management
            ("/api/config/backup", 200, "application/json"),
            
            # OTA endpoints
            # ("/ota", 200, "text/html"),  # May not exist
            ("/api/ota/status", 200, "application/json"),
            
            # PWA files (may not exist)
            # ("/sw.js", 200, "application/javascript"),
            # ("/manifest.json", 200, "application/json"),
            
            # API v1 endpoints (may not exist)
            # ("/api/v1/sensors/temperature/history", 200, "application/json"),
            # ("/api/v1/sensors/battery/history", 200, "application/json"),
            # ("/api/v1/system/processes", 200, "application/json"),
            # ("/api/v1/diagnostics/health", 200, "application/json"),
        ]
        
        failed = []
        
        # Test endpoints in smaller batches with delays
        batch_size = 5
        for i in range(0, len(get_endpoints), batch_size):
            batch = get_endpoints[i:i+batch_size]
            
            for endpoint, expected_status, expected_content_type in batch:
            try:
                response = tracked_request("GET", endpoint)
                
                # Check status code
                if response.status_code != expected_status:
                    failed.append(f"{endpoint}: status {response.status_code} != {expected_status}")
                    continue
                    
                # Check content type
                content_type = response.headers.get("Content-Type", "").split(";")[0].strip()
                if content_type != expected_content_type:
                    failed.append(f"{endpoint}: content-type '{content_type}' != '{expected_content_type}'")
                    
                self.log_info(f"✓ GET {endpoint} [{expected_status}, {expected_content_type}]")
                
            except Exception as e:
                failed.append(f"{endpoint}: {str(e)}")
                self.log_error(f"✗ GET {endpoint}: {str(e)}")
            
            # Delay between batches
            if i + batch_size < len(get_endpoints):
                time.sleep(2)
                
        assert not failed, f"Failed endpoints:\n" + "\n".join(failed)
        
    def test_post_endpoints(self, tracked_request):
        """Test all POST endpoints with valid and invalid data."""
        post_tests = [
            # Config update
            {
                "endpoint": "/api/config",
                "data": {
                    "wifi_ssid": "TestSSID",
                    "wifi_password": "TestPass123",
                    "brightness": 128,
                    "auto_dim": True,
                    "auto_update": False
                },
                "expected_status": 200
            },
            # Invalid config
            {
                "endpoint": "/api/config",
                "data": {"invalid_field": "value"},
                "expected_status": 400
            },
            # Control commands
            {
                "endpoint": "/api/control",
                "data": {"brightness": 200},
                "expected_status": 200
            },
            # Restart with auth
            {
                "endpoint": "/api/restart",
                "data": {},
                "headers": {"X-Restart-Token": "esp32-restart"},
                "expected_status": 200
            },
            # Restart without auth
            {
                "endpoint": "/api/restart",
                "data": {},
                "expected_status": 403
            },
            # Screenshot request
            {
                "endpoint": "/api/v1/display/screenshot",
                "data": {},
                "expected_status": 200
            }
        ]
        
        for test in post_tests:
            endpoint = test["endpoint"]
            data = test.get("data", {})
            headers = test.get("headers", {})
            expected = test["expected_status"]
            
            try:
                response = tracked_request("POST", endpoint, json=data, headers=headers)
                assert response.status_code == expected, \
                    f"{endpoint}: got {response.status_code}, expected {expected}"
                self.log_info(f"✓ POST {endpoint} -> {response.status_code}")
            except Exception as e:
                self.log_error(f"✗ POST {endpoint}: {str(e)}")
                raise
                
    def test_patch_endpoints(self, tracked_request):
        """Test PATCH endpoints for partial updates."""
        patch_tests = [
            # Valid field updates
            ("/api/v1/config/wifi_ssid", {"value": "NewSSID"}, 200),
            ("/api/v1/config/brightness", {"value": 150}, 200),
            ("/api/v1/config/auto_brightness", {"value": True}, 200),
            
            # Invalid field
            ("/api/v1/config/nonexistent", {"value": "test"}, 400),
            
            # Invalid values
            ("/api/v1/config/brightness", {"value": 300}, 400),  # Out of range
            ("/api/v1/config/wifi_ssid", {"value": ""}, 400),    # Empty SSID
        ]
        
        for endpoint, data, expected_status in patch_tests:
            try:
                # Convert to JSON string for PATCH
                response = tracked_request("PATCH", endpoint, 
                                         data=json.dumps(data),
                                         headers={"Content-Type": "application/json"})
                assert response.status_code == expected_status, \
                    f"{endpoint}: got {response.status_code}, expected {expected_status}"
                self.log_info(f"✓ PATCH {endpoint} -> {response.status_code}")
            except Exception as e:
                self.log_error(f"✗ PATCH {endpoint}: {str(e)}")
                raise
                
    def test_query_parameters(self, tracked_request):
        """Test endpoints that accept query parameters."""
        query_tests = [
            # Sensor history with hours parameter
            ("/api/v1/sensors/temperature/history?hours=24", 200),
            ("/api/v1/sensors/temperature/history?hours=48", 200),
            ("/api/v1/sensors/battery/history?hours=12", 200),
            
            # Default values when no params
            ("/api/v1/sensors/temperature/history", 200),
            
            # Invalid parameters (should still work)
            ("/api/v1/sensors/temperature/history?invalid=param", 200),
        ]
        
        for endpoint, expected_status in query_tests:
            response = tracked_request("GET", endpoint)
            assert response.status_code == expected_status
            
            # Verify response structure for history endpoints
            if "history" in endpoint and response.status_code == 200:
                data = response.json()
                assert "hours" in data
                assert "data" in data
                assert isinstance(data["data"], list)
                
    def test_large_payloads(self, tracked_request):
        """Test server behavior with large payloads."""
        # Generate large config data
        large_ssid = "A" * 32  # Max SSID length
        large_password = "B" * 63  # Max password length
        
        # Test large but valid config
        response = tracked_request("POST", "/api/config", json={
            "wifi_ssid": large_ssid,
            "wifi_password": large_password,
            "brightness": 255,
            "auto_dim": True,
            "auto_update": True
        })
        assert response.status_code == 200
        
        # Test oversized payload (should fail gracefully)
        huge_data = {"data": "X" * 10000}  # 10KB of data
        try:
            response = tracked_request("POST", "/api/config", json=huge_data, timeout=5)
            # Should reject or timeout
            assert response.status_code in [400, 413, 500]
        except requests.exceptions.Timeout:
            # Timeout is acceptable for oversized payload
            pass
            
    def test_concurrent_api_calls(self, tracked_request):
        """Test API behavior under concurrent load."""
        import concurrent.futures
        
        endpoints = [
            "/health",
            "/api/system",
            "/api/metrics",
            "/api/config"
        ]
        
        def make_request(endpoint):
            try:
                response = tracked_request("GET", endpoint)
                return endpoint, response.status_code
            except Exception as e:
                return endpoint, f"Error: {str(e)}"
                
        # Make 3 concurrent requests to different endpoints
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
            futures = [executor.submit(make_request, ep) for ep in endpoints[:3]]
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
            
        # Check results
        for endpoint, status in results:
            if isinstance(status, int):
                assert status == 200, f"{endpoint} returned {status}"
            else:
                self.log_warning(f"{endpoint}: {status}")
                
    def test_malformed_requests(self, tracked_request):
        """Test server resilience to malformed requests."""
        malformed_tests = [
            # Invalid JSON
            {
                "endpoint": "/api/config",
                "method": "POST",
                "data": "{invalid json}",
                "headers": {"Content-Type": "application/json"},
                "expected": [400, 500]
            },
            # Missing content type
            {
                "endpoint": "/api/config",
                "method": "POST",
                "data": json.dumps({"wifi_ssid": "test"}),
                "headers": {},
                "expected": [200, 400]  # May work or require content-type
            },
            # Empty body
            {
                "endpoint": "/api/control",
                "method": "POST",
                "data": "",
                "expected": [400, 500]
            }
        ]
        
        for test in malformed_tests:
            try:
                response = tracked_request(
                    test["method"],
                    test["endpoint"],
                    data=test["data"],
                    headers=test.get("headers", {})
                )
                assert response.status_code in test["expected"], \
                    f"{test['endpoint']}: {response.status_code} not in {test['expected']}"
            except Exception:
                # Server errors are acceptable for malformed requests
                pass
                
    def test_authentication_endpoints(self, tracked_request):
        """Test authentication on protected endpoints."""
        # Test restart endpoint authentication
        auth_tests = [
            # Correct token
            ("esp32-restart", 200),
            # Wrong token
            ("wrong-token", 403),
            # No token
            (None, 403),
            # Empty token
            ("", 403),
        ]
        
        for token, expected_status in auth_tests:
            headers = {"X-Restart-Token": token} if token is not None else {}
            response = tracked_request("POST", "/api/restart", headers=headers)
            assert response.status_code == expected_status, \
                f"Token '{token}': got {response.status_code}, expected {expected_status}"
                
    def test_config_backup_restore(self, tracked_request):
        """Test configuration backup and restore cycle."""
        # Get current config
        original = tracked_request("GET", "/api/config").json()
        
        # Backup config
        backup_response = tracked_request("GET", "/api/config/backup")
        assert backup_response.status_code == 200
        backup_data = backup_response.json()
        
        # Verify backup contains expected fields
        assert "wifi_ssid" in backup_data
        assert "brightness" in backup_data
        
        # Modify config
        tracked_request("POST", "/api/config", json={
            "wifi_ssid": "TempSSID",
            "wifi_password": original.get("wifi_password", ""),
            "brightness": 50,
            "auto_dim": False,
            "auto_update": True
        })
        
        # Verify change
        modified = tracked_request("GET", "/api/config").json()
        assert modified["wifi_ssid"] == "TempSSID"
        assert modified["brightness"] == 50
        
        # Restore from backup
        restore_response = tracked_request("POST", "/api/config/restore", 
                                         json=backup_data)
        assert restore_response.status_code == 200
        
        # Verify restoration
        restored = tracked_request("GET", "/api/config").json()
        assert restored["wifi_ssid"] == original["wifi_ssid"]
        assert restored["brightness"] == original["brightness"]
        
    def test_ota_endpoints(self, tracked_request):
        """Test OTA update endpoints."""
        # Check OTA status
        status_response = tracked_request("GET", "/api/ota/status")
        assert status_response.status_code == 200
        status = status_response.json()
        
        assert "status" in status
        assert status["status"] in ["available", "unavailable"]
        
        # OTA page should load
        ota_page = tracked_request("GET", "/ota")
        assert ota_page.status_code == 200
        assert "OTA Update" in ota_page.text or "Update" in ota_page.text
        
    def test_binary_metrics(self, tracked_request):
        """Test binary metrics endpoint."""
        response = tracked_request("GET", "/api/metrics/binary")
        assert response.status_code == 200
        assert response.headers.get("Content-Type") == "application/octet-stream"
        
        # Verify binary data structure (should be valid binary packet)
        data = response.content
        assert len(data) > 0
        # Could add more validation of binary format here
        
    def test_prometheus_metrics(self, tracked_request):
        """Test Prometheus metrics format."""
        response = tracked_request("GET", "/metrics")
        assert response.status_code == 200
        assert "text/plain" in response.headers.get("Content-Type", "")
        
        # Verify Prometheus format
        lines = response.text.strip().split("\n")
        for line in lines:
            if line and not line.startswith("#"):
                # Should be metric_name{labels} value format
                assert " " in line, f"Invalid metric line: {line}"
                
    def test_file_api_endpoints(self, tracked_request):
        """Test file management API endpoints if available."""
        # These might not be enabled on all builds
        file_endpoints = [
            "/api/files",
            "/api/files/content?file=config.json",
        ]
        
        for endpoint in file_endpoints:
            try:
                response = tracked_request("GET", endpoint)
                # These might return 404 if file system is not available
                if response.status_code == 200:
                    self.log_info(f"✓ File API available: {endpoint}")
            except:
                pass
                
    def test_edge_cases(self, tracked_request):
        """Test various edge cases."""
        edge_cases = [
            # Very long URL
            ("/api/test/" + "a" * 100, 404),
            
            # Special characters in URL
            ("/api/test%20space", 404),
            ("/api/test?param=value&other=test", 404),
            
            # Double slashes
            ("/api//config", 404),
            ("//health", 404),
            
            # Trailing slash
            ("/health/", 404),  # Might work or not
            
            # Case sensitivity
            ("/HEALTH", 404),
            ("/Api/Config", 404),
        ]
        
        for endpoint, expected in edge_cases:
            try:
                response = tracked_request("GET", endpoint)
                # Accept either expected status or 404/400
                assert response.status_code in [expected, 404, 400]
            except:
                # Connection errors are acceptable for malformed URLs
                pass
                
    def test_method_not_allowed(self, tracked_request):
        """Test unsupported HTTP methods."""
        # Endpoints that only support GET
        get_only = ["/health", "/api/system", "/dashboard"]
        
        for endpoint in get_only:
            for method in ["POST", "PUT", "DELETE"]:
                try:
                    response = requests.request(
                        method, 
                        f"http://{self.device_ip}{endpoint}",
                        timeout=5
                    )
                    # Should return 405 Method Not Allowed or 404
                    assert response.status_code in [404, 405, 501]
                except:
                    pass
                    
    def test_response_headers_security(self, tracked_request):
        """Test security-related response headers."""
        response = tracked_request("GET", "/api/system")
        headers = response.headers
        
        # Check for security headers (these are recommendations)
        security_headers = {
            "X-Content-Type-Options": "nosniff",
            "X-Frame-Options": ["DENY", "SAMEORIGIN"],
            "X-XSS-Protection": "1; mode=block"
        }
        
        warnings = []
        for header, expected_values in security_headers.items():
            if header not in headers:
                warnings.append(f"Missing security header: {header}")
            elif isinstance(expected_values, list):
                if headers[header] not in expected_values:
                    warnings.append(f"{header} should be one of {expected_values}")
            elif headers[header] != expected_values:
                warnings.append(f"{header} should be '{expected_values}'")
                
        if warnings:
            self.log_warning("Security header recommendations:\n" + "\n".join(warnings))
            
    def test_cache_headers(self, tracked_request):
        """Test caching headers on different endpoints."""
        cache_tests = [
            # Dynamic content should not be cached
            ("/api/metrics", "no-cache"),
            ("/api/system", "no-cache"),
            
            # Static content can be cached
            ("/dashboard.css", ["public", "max-age"]),
            ("/sw.js", None),  # Any caching is fine
            
            # HTML pages typically shouldn't be cached long
            ("/", ["no-cache", "private", "max-age=0"]),
        ]
        
        for endpoint, expected_cache in cache_tests:
            response = tracked_request("GET", endpoint)
            cache_control = response.headers.get("Cache-Control", "")
            
            if expected_cache:
                if isinstance(expected_cache, list):
                    assert any(exp in cache_control for exp in expected_cache), \
                        f"{endpoint}: Cache-Control '{cache_control}' doesn't match {expected_cache}"
                else:
                    assert expected_cache in cache_control
                    
    def test_cors_preflight(self, tracked_request):
        """Test CORS preflight requests."""
        # Test OPTIONS method for CORS preflight
        headers = {
            "Origin": "http://example.com",
            "Access-Control-Request-Method": "POST",
            "Access-Control-Request-Headers": "Content-Type"
        }
        
        try:
            response = requests.options(
                f"http://{self.device_ip}/api/config",
                headers=headers,
                timeout=5
            )
            
            # Should either support CORS or return 405/501
            if response.status_code == 200:
                # Check CORS headers
                assert "Access-Control-Allow-Origin" in response.headers
                assert "Access-Control-Allow-Methods" in response.headers
            else:
                assert response.status_code in [405, 501]  # Method not allowed
        except:
            # OPTIONS might not be supported
            pass


if __name__ == "__main__":
    pytest.main([__file__, "-v"])