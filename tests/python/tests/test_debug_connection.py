"""Debug test to understand connection issues"""
import requests
import pytest


class TestDebugConnection:
    """Debug why certain requests fail"""
    
    def test_nonexistent_endpoints(self, device_info):
        """Test what happens with non-existent endpoints"""
        endpoints = [
            "/api/v1/metrics",  # The failing endpoint
            "/nonexistent",     # Obviously wrong
            "/api/metrics",     # This works
        ]
        
        for endpoint in endpoints:
            print(f"\nTesting {endpoint}...")
            try:
                response = requests.get(
                    f"http://{device_info['ip']}{endpoint}",
                    timeout=5,
                    headers={'User-Agent': 'Mozilla/5.0'}
                )
                print(f"  ✅ Status: {response.status_code}")
                if response.status_code == 404:
                    print(f"  📄 Body: {response.text[:100]}")
            except Exception as e:
                print(f"  ❌ Error: {type(e).__name__}: {str(e)}")
    
    def test_with_custom_headers(self, device_info):
        """Test if custom headers cause issues"""
        test_cases = [
            ("No headers", {}),
            ("User-Agent only", {'User-Agent': 'Mozilla/5.0'}),
            ("Test headers", {'X-Test-ID': 'test123', 'X-Request-ID': 'req456'}),
            ("All headers", {
                'User-Agent': 'Mozilla/5.0',
                'X-Test-ID': 'test123',
                'X-Request-ID': 'req456'
            }),
        ]
        
        for name, headers in test_cases:
            print(f"\nTesting with {name}...")
            try:
                response = requests.get(
                    f"http://{device_info['ip']}/health",
                    timeout=5,
                    headers=headers
                )
                print(f"  ✅ Status: {response.status_code}")
            except Exception as e:
                print(f"  ❌ Error: {type(e).__name__}: {str(e)}")