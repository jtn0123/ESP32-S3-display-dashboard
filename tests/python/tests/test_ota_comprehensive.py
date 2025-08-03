"""Comprehensive OTA update tests"""

import pytest
import time
import hashlib
import os
from pathlib import Path
from utils.base_test import ESP32TestBase as BaseTest


@pytest.mark.ota
@pytest.mark.integration
class TestOTAComprehensive(BaseTest):
    """Comprehensive OTA update testing"""
    
    def test_ota_endpoints_exist(self, tracked_request):
        """Test all OTA endpoints are available"""
        endpoints = [
            "/api/ota/status",
            "/api/ota/check", 
            "/api/ota/start",
            "/api/ota/progress",
            "/api/ota/cancel",
            "/api/ota/rollback",
            "/ota",  # Web UI endpoint
        ]
        
        for endpoint in endpoints:
            response = tracked_request("GET", endpoint)
            # 404 is OK, we just want to know what exists
            if response.status_code != 404:
                self.log_info(f"OTA endpoint exists: {endpoint} -> {response.status_code}")
            else:
                self.log_info(f"OTA endpoint missing: {endpoint}")
                
    def test_ota_status_structure(self, tracked_request):
        """Test OTA status response structure"""
        response = tracked_request("GET", "/api/ota/status")
        
        if response.status_code == 200:
            data = response.json()
            
            # Expected fields
            expected_fields = [
                "current_version",
                "state",  # idle, checking, downloading, ready, failed
                "last_check",
                "auto_update_enabled"
            ]
            
            for field in expected_fields:
                if field in data:
                    self.log_info(f"OTA status has {field}: {data[field]}")
                else:
                    self.log_warning(f"OTA status missing {field}")
                    
    def test_ota_check_mechanism(self, tracked_request, test_context):
        """Test OTA check mechanism"""
        # Trigger check
        response = tracked_request("POST", "/api/ota/check")
        
        if response.status_code in [200, 202, 204]:
            self.log_info("OTA check initiated successfully")
            
            # Wait and check status
            time.sleep(2)
            
            status_response = tracked_request("GET", "/api/ota/status")
            if status_response.status_code == 200:
                status = status_response.json()
                test_context.add_metric("ota_check", {
                    "check_initiated": True,
                    "current_state": status.get("state", "unknown")
                })
        else:
            self.log_warning(f"OTA check returned: {response.status_code}")
            
    def test_ota_binary_upload(self, http_client):
        """Test OTA binary upload endpoint"""
        # Create a small test binary
        test_binary = b"TEST_FIRMWARE_v1.0" + b"\x00" * 1000
        
        # Try multipart upload
        files = {"firmware": ("test.bin", test_binary, "application/octet-stream")}
        
        response = http_client.post("/api/ota/upload", files=files)
        
        if response.status_code == 200:
            self.log_info("OTA upload endpoint functional")
        elif response.status_code == 404:
            self.log_info("OTA upload endpoint not implemented")
        else:
            self.log_warning(f"OTA upload returned: {response.status_code}")
            
    def test_ota_validation(self, tracked_request):
        """Test OTA validation mechanisms"""
        # Test with invalid data
        invalid_updates = [
            {"binary": "not-base64"},
            {"url": "not-a-url"},
            {"version": ""},
            {"checksum": "invalid"}
        ]
        
        for invalid_data in invalid_updates:
            response = tracked_request("POST", "/api/ota/validate", json=invalid_data)
            
            if response.status_code in [400, 422]:
                self.log_info(f"Properly rejected invalid OTA data: {list(invalid_data.keys())[0]}")
            elif response.status_code == 404:
                self.log_info("OTA validation endpoint not implemented")
                break
                
    def test_ota_security_features(self, tracked_request):
        """Test OTA security features"""
        # Check if signed updates are required
        response = tracked_request("GET", "/api/ota/security")
        
        if response.status_code == 200:
            data = response.json()
            security_features = {
                "signature_required": data.get("signature_required", False),
                "checksum_validation": data.get("checksum_validation", False),
                "https_only": data.get("https_only", False),
                "rollback_protection": data.get("rollback_protection", False)
            }
            
            self.log_info("OTA Security features:")
            for feature, enabled in security_features.items():
                self.log_info(f"  {feature}: {'✓' if enabled else '✗'}")
        else:
            self.log_info("OTA security endpoint not available")
            
    def test_ota_rollback_capability(self, tracked_request):
        """Test OTA rollback functionality"""
        response = tracked_request("GET", "/api/ota/rollback")
        
        if response.status_code == 200:
            data = response.json()
            
            can_rollback = data.get("can_rollback", False)
            previous_version = data.get("previous_version", "unknown")
            rollback_count = data.get("rollback_count", 0)
            
            self.log_info(f"Rollback capability:")
            self.log_info(f"  Can rollback: {can_rollback}")
            self.log_info(f"  Previous version: {previous_version}")
            self.log_info(f"  Rollback count: {rollback_count}")
            
            # Test rollback trigger (dry run)
            if can_rollback:
                response = tracked_request("POST", "/api/ota/rollback", 
                                         json={"dry_run": True})
                if response.status_code == 200:
                    self.log_info("Rollback dry run successful")
                    
    def test_ota_progress_tracking(self, tracked_request, test_context):
        """Test OTA progress tracking"""
        # Check if progress endpoint exists
        response = tracked_request("GET", "/api/ota/progress")
        
        if response.status_code == 200:
            data = response.json()
            
            expected_fields = [
                "state",
                "progress_percent",
                "bytes_written",
                "total_bytes",
                "estimated_time_remaining"
            ]
            
            progress_info = {}
            for field in expected_fields:
                if field in data:
                    progress_info[field] = data[field]
                    
            test_context.add_metric("ota_progress", progress_info)
            
            # If OTA is in progress, monitor it
            if data.get("state") not in ["idle", None]:
                self.monitor_ota_progress(tracked_request, test_context)
                
    def monitor_ota_progress(self, tracked_request, test_context, timeout=120):
        """Monitor ongoing OTA update"""
        start_time = time.time()
        progress_history = []
        
        while time.time() - start_time < timeout:
            response = tracked_request("GET", "/api/ota/progress")
            
            if response.status_code == 200:
                data = response.json()
                progress_history.append({
                    "time": time.time() - start_time,
                    "percent": data.get("progress_percent", 0),
                    "state": data.get("state", "unknown")
                })
                
                state = data.get("state")
                if state in ["completed", "failed", "idle"]:
                    break
                    
            time.sleep(1)
            
        test_context.add_metric("ota_monitoring", {
            "duration": time.time() - start_time,
            "samples": len(progress_history),
            "final_state": progress_history[-1]["state"] if progress_history else "unknown"
        })
        
    def test_ota_auto_update_config(self, tracked_request):
        """Test OTA auto-update configuration"""
        # Get current config
        response = tracked_request("GET", "/api/config")
        
        if response.status_code == 200:
            config = response.json()
            
            ota_settings = {
                "ota_enabled": config.get("ota_enabled", False),
                "ota_check_interval_hours": config.get("ota_check_interval_hours", 0),
                "ota_url": config.get("ota_url", "")
            }
            
            self.log_info("OTA Configuration:")
            for key, value in ota_settings.items():
                self.log_info(f"  {key}: {value}")
                
            # Test toggling OTA
            if "ota_enabled" in config:
                new_config = config.copy()
                new_config["ota_enabled"] = not config["ota_enabled"]
                
                response = tracked_request("POST", "/api/config", json=new_config)
                if response.status_code == 200:
                    self.log_info("Successfully toggled OTA enabled state")
                    
                    # Restore original
                    tracked_request("POST", "/api/config", json=config)
                    
    def test_ota_partition_info(self, tracked_request):
        """Test OTA partition information"""
        response = tracked_request("GET", "/api/system")
        
        if response.status_code == 200:
            data = response.json()
            
            # Look for partition info
            partition_info = {
                "app_partition": data.get("app_partition", "unknown"),
                "running_partition": data.get("running_partition", "unknown"),
                "boot_count": data.get("boot_count", 0)
            }
            
            self.log_info("Partition Information:")
            for key, value in partition_info.items():
                if value != "unknown":
                    self.log_info(f"  {key}: {value}")
                    
    def test_ota_failure_recovery(self, tracked_request):
        """Test OTA failure recovery mechanisms"""
        # Check if device reports previous OTA failures
        response = tracked_request("GET", "/api/ota/history")
        
        if response.status_code == 200:
            data = response.json()
            
            failures = data.get("failures", [])
            if failures:
                self.log_info(f"Previous OTA failures: {len(failures)}")
                for failure in failures[:3]:  # Show last 3
                    self.log_info(f"  - {failure.get('timestamp', 'unknown')}: {failure.get('reason', 'unknown')}")
                    
    def test_ota_bandwidth_limiting(self, tracked_request):
        """Test if OTA downloads are bandwidth limited"""
        response = tracked_request("GET", "/api/ota/settings")
        
        if response.status_code == 200:
            data = response.json()
            
            bandwidth_limit = data.get("bandwidth_limit_kbps", 0)
            if bandwidth_limit > 0:
                self.log_info(f"OTA bandwidth limited to: {bandwidth_limit} kbps")
            else:
                self.log_info("No OTA bandwidth limiting")
                
    def test_ota_scheduling(self, tracked_request):
        """Test OTA scheduling features"""
        response = tracked_request("GET", "/api/ota/schedule")
        
        if response.status_code == 200:
            data = response.json()
            
            schedule_info = {
                "scheduled_time": data.get("next_check", "none"),
                "quiet_hours": data.get("quiet_hours", "none"),
                "update_on_battery": data.get("update_on_battery", True)
            }
            
            self.log_info("OTA Scheduling:")
            for key, value in schedule_info.items():
                self.log_info(f"  {key}: {value}")