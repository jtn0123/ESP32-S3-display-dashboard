#!/usr/bin/env python3
"""Simple test runner without pytest dependency"""

import sys
import importlib.util
import inspect
import traceback
import time
import requests

DEVICE_IP = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"

class TestContext:
    """Simple test context"""
    def __init__(self, device_ip):
        self.device_ip = device_ip
        self.base_url = f"http://{device_ip}"
        self.passed = 0
        self.failed = 0
        self.errors = []
    
    def tracked_request(self, method, path, **kwargs):
        """Make HTTP request"""
        url = f"{self.base_url}{path}" if not path.startswith("http") else path
        return requests.request(method, url, **kwargs)
    
    def log_info(self, msg):
        print(f"  {msg}")
    
    def log_error(self, msg):
        print(f"  ❌ {msg}")
        self.errors.append(msg)

def run_test_file(filename, context):
    """Run tests from a single file"""
    print(f"\n{'='*60}")
    print(f"Running tests from: {filename}")
    print(f"{'='*60}")
    
    # Load the module
    spec = importlib.util.spec_from_file_location("test_module", filename)
    module = importlib.util.module_from_spec(spec)
    
    try:
        spec.loader.exec_module(module)
    except Exception as e:
        print(f"❌ Failed to load module: {e}")
        return
    
    # Find test classes
    for name, obj in inspect.getmembers(module):
        if inspect.isclass(obj) and name.startswith("Test"):
            print(f"\n--- {name} ---")
            run_test_class(obj, context)

def run_test_class(test_class, context):
    """Run tests from a test class"""
    try:
        # Create instance with BaseTest compatibility
        if hasattr(test_class, '__bases__'):
            # Inject context methods
            instance = test_class()
            instance.device_ip = context.device_ip
            instance.base_url = context.base_url
            instance.tracked_request = context.tracked_request
            instance.log_info = context.log_info
            instance.log_error = context.log_error
        else:
            instance = test_class()
    except Exception as e:
        print(f"❌ Failed to create test instance: {e}")
        return
    
    # Find and run test methods
    for name, method in inspect.getmembers(instance, inspect.ismethod):
        if name.startswith("test_"):
            run_test_method(instance, name, method, context)

def run_test_method(instance, name, method, context):
    """Run a single test method"""
    print(f"\n  {name}...", end=" ")
    
    try:
        # Setup if exists
        if hasattr(instance, 'setup_method'):
            instance.setup_method()
        
        # Run test
        start = time.time()
        
        # Check method signature to pass context if needed
        sig = inspect.signature(method)
        if len(sig.parameters) > 0:
            # Method expects parameters, pass context
            method(context.tracked_request)
        else:
            # No parameters expected
            method()
        
        duration = time.time() - start
        print(f"✅ PASSED ({duration:.2f}s)")
        context.passed += 1
        
        # Teardown if exists
        if hasattr(instance, 'teardown_method'):
            instance.teardown_method()
            
    except Exception as e:
        print(f"❌ FAILED")
        print(f"    Error: {str(e)}")
        if "--verbose" in sys.argv:
            traceback.print_exc()
        context.failed += 1

def main():
    """Run all tests"""
    print(f"Simple Test Runner")
    print(f"Device IP: {DEVICE_IP}")
    
    # Check device is reachable
    try:
        resp = requests.get(f"http://{DEVICE_IP}/health", timeout=5)
        if resp.status_code != 200:
            print(f"❌ Device not healthy: {resp.status_code}")
            return 1
        print(f"✅ Device is reachable")
    except Exception as e:
        print(f"❌ Device not reachable: {e}")
        return 1
    
    # Create test context
    context = TestContext(DEVICE_IP)
    
    # Test files to run
    test_files = [
        "tests/test_device_stability.py",
        "tests/test_freeze_detection.py",
        "tests/test_web_comprehensive.py",
    ]
    
    # Run tests
    for test_file in test_files:
        try:
            run_test_file(test_file, context)
        except Exception as e:
            print(f"\n❌ Error running {test_file}: {e}")
            if "--verbose" in sys.argv:
                traceback.print_exc()
    
    # Summary
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"✅ Passed: {context.passed}")
    print(f"❌ Failed: {context.failed}")
    print(f"Total: {context.passed + context.failed}")
    
    if context.errors:
        print(f"\nErrors:")
        for error in context.errors[:10]:
            print(f"  - {error}")
    
    return 0 if context.failed == 0 else 1

if __name__ == "__main__":
    sys.exit(main())