#!/usr/bin/env python3
"""Run all tests and summarize results"""

import subprocess
import sys
import time

DEVICE_IP = sys.argv[1] if len(sys.argv) > 1 else "10.27.27.201"

# Test files to run
test_files = [
    "tests/test_device_stability.py",
    "tests/test_freeze_detection.py", 
    "tests/test_web_comprehensive.py",
    "tests/test_web_ui_comprehensive.py",
    "tests/test_ota_comprehensive.py",
    "tests/test_binary_size_limits.py"
]

# Additional standalone tests
standalone_tests = [
    ("debug_freeze.py", "Freeze Detection"),
    ("test_ota_simple.py", "OTA Improvements"),
    ("stress_test.py", "Stress Test (30s)", ["30"]),
]

print(f"ESP32-S3 Dashboard Test Suite")
print(f"Device: {DEVICE_IP}")
print("=" * 60)

# Check device is reachable
import requests
try:
    resp = requests.get(f"http://{DEVICE_IP}/health", timeout=5)
    version = "unknown"
    resp2 = requests.get(f"http://{DEVICE_IP}/", timeout=5)
    if "v5." in resp2.text:
        import re
        match = re.search(r'v5\.\d+', resp2.text)
        if match:
            version = match.group(0)
    print(f"✅ Device online - Version: {version}")
except Exception as e:
    print(f"❌ Device not reachable: {e}")
    sys.exit(1)

print("\n" + "=" * 60)

# Run pytest tests
print("\n📋 Running pytest test suites...\n")

results = {}
for test_file in test_files:
    test_name = test_file.split('/')[-1].replace('test_', '').replace('.py', '')
    print(f"\n--- {test_name.upper()} ---")
    
    cmd = ["python3", "-m", "pytest", test_file, "-v", "--tb=short", f"--device-ip={DEVICE_IP}", "-q"]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        
        # Parse output for pass/fail counts
        output = result.stdout + result.stderr
        
        # Look for pytest summary
        passed = failed = 0
        for line in output.split('\n'):
            if 'passed' in line and 'failed' in line:
                # Parse summary line like "1 failed, 2 passed in 10.5s"
                import re
                passed_match = re.search(r'(\d+) passed', line)
                failed_match = re.search(r'(\d+) failed', line)
                if passed_match:
                    passed = int(passed_match.group(1))
                if failed_match:
                    failed = int(failed_match.group(1))
            elif line.startswith('PASSED'):
                passed += 1
            elif line.startswith('FAILED'):
                failed += 1
        
        # If no summary found, check return code
        if passed == 0 and failed == 0:
            if result.returncode == 0:
                passed = 1  # At least something passed
            else:
                failed = 1  # Something failed
        
        results[test_name] = {'passed': passed, 'failed': failed}
        
        if failed == 0 and passed > 0:
            print(f"✅ {passed} tests passed")
        elif failed > 0:
            print(f"❌ {failed} failed, {passed} passed")
        else:
            print(f"⚠️  Could not determine results")
            
        # Show key failures
        for line in output.split('\n'):
            if 'FAILED' in line and '::' in line:
                print(f"   - {line.strip()}")
                
    except subprocess.TimeoutExpired:
        print(f"⏱️  Timeout after 120s")
        results[test_name] = {'passed': 0, 'failed': 1}
    except Exception as e:
        print(f"❌ Error running test: {e}")
        results[test_name] = {'passed': 0, 'failed': 1}

# Run standalone tests
print("\n\n📋 Running standalone tests...\n")

for test_info in standalone_tests:
    test_file = test_info[0]
    test_name = test_info[1]
    extra_args = test_info[2] if len(test_info) > 2 else []
    
    print(f"\n--- {test_name} ---")
    cmd = ["python3", test_file, DEVICE_IP] + extra_args
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        
        if result.returncode == 0:
            # Check output for success indicators
            output = result.stdout
            if "Device still responsive" in output or "Summary" in output:
                print(f"✅ Test completed successfully")
                results[test_name.lower().replace(' ', '_')] = {'passed': 1, 'failed': 0}
            else:
                print(f"⚠️  Test completed with warnings")
                results[test_name.lower().replace(' ', '_')] = {'passed': 1, 'failed': 0}
        else:
            print(f"❌ Test failed")
            results[test_name.lower().replace(' ', '_')] = {'passed': 0, 'failed': 1}
            
    except subprocess.TimeoutExpired:
        print(f"⏱️  Timeout")
        results[test_name.lower().replace(' ', '_')] = {'passed': 0, 'failed': 1}
    except Exception as e:
        print(f"❌ Error: {e}")
        results[test_name.lower().replace(' ', '_')] = {'passed': 0, 'failed': 1}

# Summary
print("\n" + "=" * 60)
print("📊 TEST SUMMARY")
print("=" * 60)

total_passed = sum(r['passed'] for r in results.values())
total_failed = sum(r['failed'] for r in results.values())

print(f"\nTotal: {total_passed} passed, {total_failed} failed")
print("\nDetailed Results:")

for test_name, result in sorted(results.items()):
    status = "✅" if result['failed'] == 0 else "❌"
    print(f"{status} {test_name:25} - {result['passed']} passed, {result['failed']} failed")

# Device health after tests
print("\n" + "=" * 60)
print("🏥 Final Device Health Check")
try:
    resp = requests.get(f"http://{DEVICE_IP}/api/system", timeout=5)
    if resp.status_code == 200:
        data = resp.json()
        heap = data.get('free_heap', 0)
        uptime = data.get('uptime_ms', 0) / 1000
        print(f"✅ Device responsive")
        print(f"   Free heap: {heap:,} bytes ({heap/1024/1024:.1f} MB)")
        print(f"   Uptime: {uptime:.0f} seconds")
except Exception as e:
    print(f"❌ Device not responding: {e}")

print("\n✅ = Working well")
print("❌ = Needs attention")
print("⚠️  = Minor issues")