import time
import subprocess

# Reset the device
subprocess.run(["espflash", "reset", "--port", "/dev/cu.usbmodem101"], capture_output=True)
time.sleep(1)

# Configure serial port
subprocess.run(["stty", "-f", "/dev/cu.usbmodem101", "115200", "cs8", "-cstopb", "-parenb"], capture_output=True)

# Read serial output
print("Reading serial output for 20 seconds...")
print("-" * 60)

try:
    with open("/dev/cu.usbmodem101", "rb") as serial:
        start_time = time.time()
        lcd_cam_found = False
        
        while time.time() - start_time < 20:
            try:
                # Read available data
                data = serial.read(1024)
                if data:
                    text = data.decode('utf-8', errors='replace')
                    print(text, end='', flush=True)
                    
                    # Check for key messages
                    if "LCD_CAM" in text:
                        lcd_cam_found = True
                        
            except Exception as e:
                pass
            
            time.sleep(0.01)
        
        print("\n" + "-" * 60)
        if lcd_cam_found:
            print("LCD_CAM test output detected\!")
        else:
            print("No LCD_CAM test output found - may be stuck during boot")
            
except Exception as e:
    print(f"Error: {e}")
