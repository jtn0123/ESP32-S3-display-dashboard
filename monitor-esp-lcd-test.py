#!/usr/bin/env python3
"""
ESP LCD Test Monitor - Parses and highlights test output
"""
import serial
import sys
import re
from datetime import datetime

# ANSI color codes
GREEN = '\033[92m'
YELLOW = '\033[93m'
RED = '\033[91m'
BLUE = '\033[94m'
MAGENTA = '\033[95m'
RESET = '\033[0m'
BOLD = '\033[1m'

def parse_line(line):
    """Parse and colorize log lines"""
    # Remove timestamps and module prefixes for clarity
    line = line.strip()
    
    # Critical ESP LCD message
    if "lcd_panel: new I80 bus" in line:
        return f"{GREEN}{BOLD}✓ ESP LCD INITIALIZED: {line}{RESET}"
    
    # Test progress
    if "[ESP_LCD_TEST]" in line:
        if "ERROR" in line or "Failed" in line:
            return f"{RED}✗ {line}{RESET}"
        elif "successfully" in line:
            return f"{GREEN}✓ {line}{RESET}"
        elif "FPS" in line:
            # Extract FPS number
            fps_match = re.search(r'(\d+\.?\d*)\s*FPS', line)
            if fps_match:
                fps = float(fps_match.group(1))
                if fps >= 25:
                    return f"{GREEN}{BOLD}✓ PERFORMANCE PASS: {line}{RESET}"
                else:
                    return f"{YELLOW}⚠ PERFORMANCE LOW: {line}{RESET}"
        elif any(color in line for color in ["Red", "Green", "Blue", "White", "black"]):
            return f"{BLUE}🎨 {line}{RESET}"
        else:
            return f"{MAGENTA}▶ {line}{RESET}"
    
    # Errors
    if "error" in line.lower() or "panic" in line.lower():
        return f"{RED}{BOLD}ERROR: {line}{RESET}"
    
    # Warnings
    if "warn" in line.lower():
        return f"{YELLOW}⚠ {line}{RESET}"
    
    # Info messages
    if "Creating I80 bus" in line or "Pin configuration" in line:
        return f"{BLUE}ℹ {line}{RESET}"
    
    return line

def main():
    print(f"{BOLD}ESP LCD Test Monitor{RESET}")
    print("=" * 50)
    print(f"Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 50)
    print()
    
    test_passed = False
    fps_result = None
    lcd_initialized = False
    
    try:
        # Read from stdin (piped from espflash monitor)
        for line in sys.stdin:
            parsed = parse_line(line)
            if parsed:
                print(parsed)
                
                # Track test status
                if "lcd_panel: new I80 bus" in line:
                    lcd_initialized = True
                if "FPS" in line:
                    fps_match = re.search(r'(\d+\.?\d*)\s*FPS', line)
                    if fps_match:
                        fps_result = float(fps_match.group(1))
                if "All tests completed successfully" in line:
                    test_passed = True
                    
    except KeyboardInterrupt:
        pass
    
    # Summary
    print()
    print("=" * 50)
    print(f"{BOLD}Test Summary:{RESET}")
    print(f"LCD Initialized: {'✓ Yes' if lcd_initialized else '✗ No'}")
    if fps_result:
        status = "✓ PASS" if fps_result >= 25 else "⚠ LOW"
        print(f"Performance: {fps_result:.1f} FPS {status}")
    print(f"Test Result: {'✓ PASSED' if test_passed else '✗ FAILED'}")
    print("=" * 50)

if __name__ == "__main__":
    main()