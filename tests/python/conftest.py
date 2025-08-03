"""Pytest configuration and fixtures"""
import pytest
import requests
import time
import uuid
import yaml
import os
import socket
import select
from typing import Dict, Any, Optional, Generator


def pytest_configure(config):
    """Configure pytest with custom markers"""
    config.addinivalue_line("markers", "web: Web server tests")
    config.addinivalue_line("markers", "integration: Integration tests requiring device")
    config.addinivalue_line("markers", "ota: OTA update tests")
    config.addinivalue_line("markers", "ui: Web UI tests")
    config.addinivalue_line("markers", "slow: Slow tests")


@pytest.fixture(scope="session")
def config_file():
    """Load test configuration"""
    config_path = os.path.join(os.path.dirname(__file__), 'config.yaml')
    with open(config_path, 'r') as f:
        return yaml.safe_load(f)


@pytest.fixture(scope="session")
def device_info(config_file):
    """Get device information from config or discovery"""
    # Try to use configured device
    if 'device' in config_file:
        device = config_file['device']
        if 'ip' in device:
            return {
                'ip': device['ip'],
                'hostname': device.get('hostname', 'esp32.local'),
                'port': device.get('port', 80)
            }
    
    # Try to discover device
    device_ip = discover_device()
    if device_ip:
        return {
            'ip': device_ip,
            'hostname': 'esp32.local',
            'port': 80
        }
    
    pytest.skip("No ESP32 device found")


def discover_device():
    """Discover ESP32 device on network"""
    # Simple implementation - check known IP
    known_ips = ['10.27.27.201', '192.168.1.100', '192.168.4.1']
    
    for ip in known_ips:
        try:
            response = requests.get(f"http://{ip}/health", timeout=2)
            if response.status_code == 200:
                return ip
        except:
            continue
    
    return None


@pytest.fixture
def http_client(device_info):
    """Create HTTP client for device"""
    base_url = f"http://{device_info['ip']}"
    session = requests.Session()
    session.base_url = base_url
    return session


@pytest.fixture
def test_context():
    """Create test context with unique ID"""
    return {
        'test_id': str(uuid.uuid4()),
        'start_time': time.time(),
        'logs': []
    }


@pytest.fixture
def tracked_request(http_client, test_context):
    """Create a tracked request function"""
    def make_request(method: str, path: str, **kwargs) -> requests.Response:
        # Add test context headers
        headers = kwargs.get('headers', {})
        headers['X-Test-ID'] = test_context['test_id']
        headers['X-Request-ID'] = str(uuid.uuid4())
        kwargs['headers'] = headers
        
        # Make absolute URL
        if not path.startswith('http'):
            url = f"{http_client.base_url}{path}"
        else:
            url = path
        
        # Make request
        response = http_client.request(method, url, **kwargs)
        
        # Log request/response
        test_context['logs'].append({
            'time': time.time(),
            'method': method,
            'path': path,
            'status': response.status_code,
            'response_time': response.elapsed.total_seconds()
        })
        
        return response
    
    return make_request


@pytest.fixture
def telnet_logs(device_info, test_context):
    """Capture logs from telnet server"""
    logs = []
    
    try:
        # Use raw socket instead of telnetlib (removed in Python 3.13)
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect((device_info['ip'], 23))
        sock.setblocking(False)
        
        # Start capturing
        start_time = time.time()
        while time.time() - start_time < 30:  # Max 30 seconds
            ready = select.select([sock], [], [], 0.1)
            if ready[0]:
                try:
                    data = sock.recv(4096)
                    if data:
                        log_lines = data.decode('utf-8', errors='ignore').split('\n')
                        for line in log_lines:
                            if line.strip():
                                logs.append({
                                    'time': time.time(),
                                    'line': line.strip(),
                                    'test_id': test_context['test_id']
                                })
                except socket.error:
                    break
        
        sock.close()
    except Exception as e:
        print(f"Failed to connect to telnet: {e}")
    
    return logs


@pytest.fixture
def page_url(device_info):
    """Get base URL for web UI tests"""
    return f"http://{device_info['ip']}"