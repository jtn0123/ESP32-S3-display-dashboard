"""Basic connectivity test - the first test that should pass"""
import requests
import pytest
import time


class TestBasicConnectivity:
    """Test basic device connectivity before running other tests"""
    
    def test_device_reachable_via_ping(self, device_info):
        """Test that device responds to ping"""
        import subprocess
        result = subprocess.run(
            ['ping', '-c', '1', device_info['ip']], 
            capture_output=True, 
            text=True
        )
        assert result.returncode == 0, f"Device not reachable via ping: {result.stderr}"
    
    def test_simple_http_get(self, device_info):
        """Test basic HTTP GET without any custom headers"""
        # Simple request with minimal headers
        response = requests.get(
            f"http://{device_info['ip']}/health",
            headers={'User-Agent': 'Mozilla/5.0'},  # Simple user agent
            timeout=5
        )
        assert response.status_code == 200, f"Got status {response.status_code}"
        
        # Verify response is JSON
        data = response.json()
        assert 'status' in data
        assert data['status'] == 'healthy'
    
    def test_with_pytest_headers(self, tracked_request):
        """Test if custom pytest headers cause issues"""
        try:
            response = tracked_request("GET", "/health")
            assert response.status_code == 200
        except Exception as e:
            pytest.fail(f"Failed with pytest headers: {str(e)}")
    
    def test_various_endpoints(self, device_info):
        """Test different endpoints to see what works"""
        endpoints = [
            ("/", "Home page"),
            ("/health", "Health check"),
            ("/api/metrics", "Metrics API"),
            ("/api/system", "System API"),
        ]
        
        results = []
        for endpoint, description in endpoints:
            try:
                response = requests.get(
                    f"http://{device_info['ip']}{endpoint}",
                    timeout=2
                )
                results.append(f"✅ {endpoint} ({description}): {response.status_code}")
            except Exception as e:
                results.append(f"❌ {endpoint} ({description}): {str(e)}")
        
        # Print results for debugging
        print("\nEndpoint connectivity results:")
        for result in results:
            print(f"  {result}")
        
        # At least health should work
        assert any("✅" in r and "/health" in r for r in results), "Health endpoint should work"