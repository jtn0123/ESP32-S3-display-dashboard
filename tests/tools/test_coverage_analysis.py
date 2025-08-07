#!/usr/bin/env python3
"""
Test Coverage Analysis
Analyzes existing tests and identifies gaps in test coverage
"""

import os
import re
import glob
from collections import defaultdict


class TestCoverageAnalyzer:
    """Analyzer for test coverage of the ESP32-S3 Dashboard project."""

    def __init__(self):
        self.test_files = []
        self.source_files = []
        self.test_coverage = defaultdict(list)
        self.untested_features = []
        self.all_test_content = ""

    def find_all_tests(self):
        """Find all test files in the project"""
        # Find test files in root
        root_tests = []
        for file in os.listdir('.'):
            if file.startswith('test_') and (
                    file.endswith('.py')
                    or file.endswith('.sh')):
                root_tests.append(file)

        # Find test files in tests directory
        test_dir_files = []
        if os.path.exists('tests'):
            for root, _, files in os.walk('tests'):
                for file in files:
                    if file.endswith('.py') and not file.startswith('__'):
                        test_dir_files.append(os.path.join(root, file))

        self.test_files = root_tests + test_dir_files

        return self.test_files

    def find_source_files(self):
        """Find all Rust source files"""
        self.source_files = glob.glob("src/**/*.rs", recursive=True)
        return self.source_files

    def analyze_test_file(self, filepath):
        """Analyze a single test file to understand what it tests"""
        test_info = {
            'file': filepath,
            'type': 'python' if filepath.endswith('.py') else 'shell',
            'features_tested': [],
            'endpoints_tested': [],
            'components_tested': []
        }

        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()

                # Find HTTP endpoints being tested
                endpoints = re.findall(r'["\']/([\w/\-_]+)["\']', content)
                test_info['endpoints_tested'] = list(set(endpoints))

                # Find features mentioned
                if 'sse' in filepath.lower() or 'SSE' in content:
                    test_info['features_tested'].append('Server-Sent Events')
                if 'dark' in filepath.lower() or 'dark_mode' in content:
                    test_info['features_tested'].append('Dark Mode')
                if 'template' in filepath.lower() or 'template' in content:
                    test_info['features_tested'].append('Template Engine')
                if 'etag' in filepath.lower() or 'ETag' in content:
                    test_info['features_tested'].append('ETag/Caching')
                if 'stability' in filepath.lower() or 'heap' in content:
                    test_info['features_tested'].append('Stability/Memory')
                if 'network' in filepath.lower() or 'wifi' in content.lower():
                    test_info['features_tested'].append('Network/WiFi')
                if 'ota' in filepath.lower() or 'update' in content:
                    test_info['features_tested'].append('OTA Updates')

        except Exception as e:
            print(f"Error analyzing {filepath}: {e}")

        return test_info

    def analyze_source_coverage(self):
        """Analyze which source components have tests"""
        components = {
            'display': {'files': [], 'tested': False},
            'network': {'files': [], 'tested': False},
            'sensors': {'files': [], 'tested': False},
            'ui': {'files': [], 'tested': False},
            'config': {'files': [], 'tested': False},
            'boot': {'files': [], 'tested': False},
            'performance': {'files': [], 'tested': False},
            'power': {'files': [], 'tested': False},
            'ota': {'files': [], 'tested': False},
            'serial_debug': {'files': [], 'tested': False}
        }

        # Categorize source files
        for src_file in self.source_files:
            for component, info in components.items():
                if component in src_file:
                    info['files'].append(src_file)

        # Check test coverage
        self.all_test_content = ""
        for test_file in self.test_files:
            try:
                with open(test_file, 'r', encoding='utf-8') as f:
                    self.all_test_content += f.read().lower()
            except OSError:
                pass

        for component in components:
            if component in self.all_test_content:
                components[component]['tested'] = True

        return components

    def generate_report(self):
        """Generate comprehensive test coverage report"""
        print("ESP32-S3 Dashboard Test Coverage Analysis")
        print("=" * 60)

        # Find all tests
        tests = self.find_all_tests()
        print(f"\n📁 Found {len(tests)} test files:")

        # Analyze each test
        test_analysis = []
        all_endpoints = set()
        all_features = set()

        for test in sorted(tests):
            info = self.analyze_test_file(test)
            test_analysis.append(info)
            all_endpoints.update(info['endpoints_tested'])
            all_features.update(info['features_tested'])

            print(f"\n  • {os.path.basename(test)}")
            if info['features_tested']:
                features_str = ', '.join(info['features_tested'])
                print(f"    Features: {features_str}")
            if info['endpoints_tested']:
                endpoint_str = ', '.join(info['endpoints_tested'][:5])
                print(f"    Endpoints: {endpoint_str}")
                if len(info['endpoints_tested']) > 5:
                    more_count = len(info['endpoints_tested']) - 5
                    print(f"    ... and {more_count} more")

        # Source coverage
        print("\n\n📊 Source Component Coverage:")
        print("-" * 40)
        self.find_source_files()
        components = self.analyze_source_coverage()

        tested_count = 0
        for comp_name, comp_info in sorted(components.items()):
            status = "✅" if comp_info['tested'] else "❌"
            print(f"{status} {comp_name.upper()}: "
                  f"{len(comp_info['files'])} files")
            if comp_info['tested']:
                tested_count += 1

        coverage_percent = (tested_count / len(components)) * 100
        print(f"\nOverall Component Coverage: {coverage_percent:.1f}%")

        # Missing tests
        print("\n\n🔍 Test Coverage Gaps:")
        print("-" * 40)

        # Components without tests
        untested_components = [
            name for name, info in components.items()
            if not info['tested']
        ]
        if untested_components:
            print("Components without explicit tests:")
            for comp in untested_components:
                print(f"  ❌ {comp}")

        # Features that might need more testing
        print("\nFeatures needing additional tests:")
        missing_features = [
            "Display rendering performance",
            "Button input handling",
            "Temperature sensor accuracy",
            "Battery monitoring",
            "Power management (auto-dim, sleep)",
            "Dual-core task distribution",
            "PSRAM usage",
            "Error recovery mechanisms",
            "Configuration persistence (NVS)",
            "mDNS advertisement",
            "Telnet server stability",
            "WebSocket connections",
            "File upload/download",
            "Memory fragmentation over time",
            "WiFi reconnection logic",
            "Serial debug commands",
            "Boot diagnostics accuracy"
        ]

        for feature in missing_features:
            if feature.lower() not in self.all_test_content:
                print(f"  ⚠️  {feature}")

        # Endpoint coverage
        print("\n\n🌐 API Endpoint Coverage:")
        print("-" * 40)
        print(f"Tested endpoints: {len(all_endpoints)}")

        # Known endpoints from source
        known_endpoints = [
            "/", "/api/config", "/api/diagnostics", "/api/events",
            "/sse/logs", "/sse/stats", "/api/system", "/api/restart",
            "/api/metrics", "/health", "/ota/update", "/api/files",
            "/dashboard", "/logs",
            "/api/v1/sensors/temperature/history",
            "/api/v1/sensors/battery/history",
            "/api/v1/display/screenshot"
        ]

        untested_endpoints = [
            ep for ep in known_endpoints if ep not in all_endpoints
        ]
        if untested_endpoints:
            print("Untested endpoints:")
            for ep in untested_endpoints:
                print(f"  ⚠️  {ep}")

        # Test execution status
        print("\n\n🏃 Test Execution Requirements:")
        print("-" * 40)
        print("Tests requiring live device:")
        device_tests = [
            t for t in tests
            if 'mock' not in t and 'validate' not in t
        ]
        for test in device_tests[:10]:
            print(f"  • {os.path.basename(test)}")
        if len(device_tests) > 10:
            print(f"  ... and {len(device_tests) - 10} more")

        print("\nTests that can run without device:")
        offline_tests = [
            t for t in tests
            if 'mock' in t or 'validate' in t or 'local' in t
        ]
        for test in offline_tests:
            print(f"  • {os.path.basename(test)}")

        # Recommendations
        print("\n\n💡 Recommendations:")
        print("-" * 40)
        print("1. High Priority Tests to Add:")
        print("   - Serial debug command validation")
        print("   - Display driver performance benchmarks")
        print("   - Button input debouncing and responsiveness")
        print("   - Power management state transitions")
        print("   - Network reconnection scenarios")

        print("\n2. Test Infrastructure Improvements:")
        print("   - Mock ESP32 server for offline testing")
        print("   - Automated test runner with device detection")
        print("   - Performance regression detection")
        print("   - Memory leak detection over extended runs")
        print("   - Integration with CI/CD pipeline")

        print("\n3. Current Test Limitations:")
        print("   - Most tests require physical device")
        print("   - No automated performance benchmarking")
        print("   - Limited sensor simulation capabilities")
        print("   - No display output validation")

        return {
            'total_tests': len(tests),
            'component_coverage': coverage_percent,
            'untested_components': untested_components,
            'untested_endpoints': untested_endpoints,
            'recommendations': missing_features
        }


def main():
    """Main entry point for the test coverage analyzer."""
    analyzer = TestCoverageAnalyzer()
    results = analyzer.generate_report()

    print("\n\n📈 Summary:")
    print(f"Total test files: {results['total_tests']}")
    print(f"Component coverage: {results['component_coverage']:.1f}%")
    print("Components without tests: "
          f"{len(results['untested_components'])}")
    print(f"Untested endpoints: {len(results['untested_endpoints'])}")


if __name__ == "__main__":
    main()
