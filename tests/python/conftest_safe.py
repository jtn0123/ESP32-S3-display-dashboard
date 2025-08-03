"""Safe fixtures that handle connection issues better"""
import pytest
import requests
import time
import uuid
from typing import Dict, Any, Optional


@pytest.fixture
def safe_tracked_request(device_info, test_context):
    """Create a safer tracked request function that handles 404s properly"""
    def make_request(method: str, path: str, **kwargs) -> requests.Response:
        # Add test context headers
        headers = kwargs.get('headers', {})
        headers['X-Test-ID'] = test_context['test_id']
        headers['X-Request-ID'] = str(uuid.uuid4())
        # Add Connection: close to prevent connection reuse issues
        headers['Connection'] = 'close'
        kwargs['headers'] = headers
        
        # Make absolute URL
        if not path.startswith('http'):
            url = f"http://{device_info['ip']}{path}"
        else:
            url = path
        
        # Set reasonable timeout
        if 'timeout' not in kwargs:
            kwargs['timeout'] = 5
        
        try:
            # Create new session for each request to avoid connection pool issues
            with requests.Session() as session:
                response = session.request(method, url, **kwargs)
                
                # Log request/response
                test_context['logs'].append({
                    'time': time.time(),
                    'method': method,
                    'path': path,
                    'status': response.status_code,
                    'response_time': response.elapsed.total_seconds()
                })
                
                return response
                
        except requests.exceptions.ConnectionError as e:
            # Log the error
            test_context['logs'].append({
                'time': time.time(),
                'method': method,
                'path': path,
                'status': 'CONNECTION_ERROR',
                'error': str(e)
            })
            raise
    
    return make_request


@pytest.fixture
def simple_request(device_info):
    """Simple request function without session reuse"""
    def make_request(method: str, path: str, **kwargs) -> requests.Response:
        # Make absolute URL
        if not path.startswith('http'):
            url = f"http://{device_info['ip']}{path}"
        else:
            url = path
        
        # Always use a fresh connection
        kwargs.setdefault('headers', {})['Connection'] = 'close'
        kwargs.setdefault('timeout', 5)
        
        return requests.request(method, url, **kwargs)
    
    return make_request