#!/usr/bin/env python3
import time
from machine import Pin

# Turn on LCD power
lcd_power = Pin(15, Pin.OUT)
lcd_power.value(1)
print("LCD power enabled on GPIO 15")

# Turn on backlight
backlight = Pin(38, Pin.OUT)
backlight.value(1)
print("Backlight enabled on GPIO 38")

# Keep running
while True:
    print("LCD should be powered...")
    time.sleep(5)