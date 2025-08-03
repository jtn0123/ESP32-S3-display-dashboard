"""Base test class for ESP32 integration tests"""
import pytest
import logging
import time
from typing import Dict, Any, Optional

class ESP32TestBase:
    """Base class for ESP32 test cases"""
    
    @pytest.fixture(autouse=True)
    def setup_test_logging(self, caplog):
        """Set up logging for tests"""
        caplog.set_level(logging.INFO)
        self.logger = logging.getLogger(self.__class__.__name__)
    
    @pytest.fixture(autouse=True)
    def setup_device_info(self, device_info):
        """Set up device info for tests"""
        self.device_ip = device_info['ip']
        self.device_url = f"http://{self.device_ip}"
        self.device_info = device_info
    
    def log_info(self, message: str):
        """Log info message"""
        self.logger.info(message)
    
    def log_error(self, message: str):
        """Log error message"""
        self.logger.error(message)
    
    def log_warning(self, message: str):
        """Log warning message"""
        self.logger.warning(message)
    
    def wait_for_device(self, timeout: int = 30) -> bool:
        """Wait for device to be accessible"""
        import requests
        start_time = time.time()
        while time.time() - start_time < timeout:
            try:
                response = requests.get(f"{self.device_url}/health", timeout=2)
                if response.status_code == 200:
                    return True
            except:
                pass
            time.sleep(1)
        return False
    
    def assert_response_ok(self, response):
        """Assert that response is successful"""
        assert response.status_code in [200, 201, 204], \
            f"Expected success status, got {response.status_code}: {response.text}"
    
    def assert_json_contains(self, response, expected: Dict[str, Any]):
        """Assert that JSON response contains expected keys/values"""
        actual = response.json()
        for key, value in expected.items():
            assert key in actual, f"Expected key '{key}' not found in response"
            if value is not None:
                assert actual[key] == value, \
                    f"Expected {key}={value}, got {actual[key]}"