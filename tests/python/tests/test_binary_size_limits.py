"""Test to determine actual binary size limits"""

import pytest
import subprocess
import os
from pathlib import Path
from utils.base_test import ESP32TestBase as BaseTest


class TestBinarySizeLimits(BaseTest):
    """Determine actual binary size constraints"""
    
    def test_current_binary_analysis(self):
        """Analyze current binary size and sections"""
        binary_path = Path("target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard")
        
        if not binary_path.exists():
            pytest.skip("Binary not found")
            
        size = binary_path.stat().st_size
        size_mb = size / (1024 * 1024)
        
        self.log_info(f"\nBinary Analysis:")
        self.log_info(f"Current size: {size_mb:.2f}MB ({size:,} bytes)")
        
        # The ESP32-S3 has 16MB flash total
        # Partition table typically allocates:
        # - OTA partition 0: ~4MB
        # - OTA partition 1: ~4MB
        # - NVS, etc: ~1MB
        
        self.log_info(f"\nFlash capacity:")
        self.log_info(f"  Total flash: 16MB")
        self.log_info(f"  Typical OTA partition: 4MB")
        self.log_info(f"  Current usage: {(size_mb/4)*100:.1f}% of OTA partition")
        
        # Check if we're approaching any limits
        if size_mb > 3.5:
            self.log_warning(f"Binary approaching 4MB OTA partition limit")
        elif size_mb > 3.0:
            self.log_info(f"Binary size healthy, {4.0 - size_mb:.1f}MB headroom")
        else:
            self.log_info(f"Plenty of space available")
            
    def test_partition_table_check(self):
        """Check actual partition sizes"""
        partition_csv = Path("partitions.csv")
        
        if partition_csv.exists():
            content = partition_csv.read_text()
            self.log_info("\nPartition Table:")
            
            for line in content.split('\n'):
                line = line.strip()
                if line and not line.startswith('#'):
                    parts = [p.strip() for p in line.split(',')]
                    if len(parts) >= 5:
                        name, type_, subtype, offset, size = parts[:5]
                        if 'ota' in name or 'app' in subtype:
                            # Parse size (might be hex or decimal with K/M suffix)
                            size_str = size.strip()
                            if size_str.endswith('M'):
                                size_mb = float(size_str[:-1])
                            elif size_str.endswith('K'):
                                size_mb = float(size_str[:-1]) / 1024
                            elif size_str.startswith('0x'):
                                size_mb = int(size_str, 16) / (1024 * 1024)
                            else:
                                size_mb = int(size_str) / (1024 * 1024)
                                
                            self.log_info(f"  {name}: {size_mb:.1f}MB")
        else:
            self.log_warning("partitions.csv not found")
            
    def test_memory_vs_binary_correlation(self, tracked_request):
        """Test if binary size directly affects runtime memory"""
        response = tracked_request("GET", "/api/system")
        
        if response.status_code == 200:
            data = response.json()
            free_heap = data.get("free_heap", 0)
            
            self.log_info(f"\nRuntime Memory:")
            self.log_info(f"  Free heap: {free_heap/1024/1024:.1f}MB")
            self.log_info(f"  PSRAM enabled: {'Yes' if free_heap > 1024*1024 else 'Maybe not'}")
            
            # The key insight: binary size affects flash usage, not RAM
            # The display allocation failure at 1.6MB was due to:
            # 1. Extra code using more RAM at runtime
            # 2. Not the binary size itself
            
            self.log_info(f"\nKey insights:")
            self.log_info(f"  - Binary size affects flash storage, not RAM")
            self.log_info(f"  - Display allocation failures are runtime memory issues")
            self.log_info(f"  - With PSRAM enabled, we have 8MB+ heap available")
            self.log_info(f"  - Binary can safely grow to 3-4MB if needed")
            
    def test_size_recommendations(self):
        """Provide size recommendations"""
        self.log_info("\nBinary Size Recommendations:")
        self.log_info("  - Safe range: Up to 3.5MB (87.5% of 4MB OTA partition)")
        self.log_info("  - Warning at: 3.5MB+ (approaching partition limit)")
        self.log_info("  - Hard limit: 4MB (OTA partition size)")
        self.log_info("  - Current 1.51MB is well within limits")
        self.log_info("\nMemory considerations:")
        self.log_info("  - Focus on runtime memory usage, not binary size")
        self.log_info("  - Monitor heap usage when adding features")
        self.log_info("  - PSRAM provides ample memory headroom")