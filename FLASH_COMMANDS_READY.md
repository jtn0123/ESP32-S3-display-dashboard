# ESP_LCD Flash Commands Ready

## Device Status
The ESP32-S3 is currently in a bootloader crash loop and needs manual intervention to flash.

## Manual Recovery Steps
1. Hold the **BOOT** button (GPIO0)
2. While holding BOOT, press **RESET** button
3. Release RESET
4. Release BOOT
5. The device should now be in download mode

## Flash Commands to Execute

Once the device is in download mode, execute these commands in order:

### 1. Erase Flash
```bash
.embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py \
    --chip esp32s3 --port /dev/cu.usbmodem101 erase_flash
```

### 2. Flash All Three Components
```bash
.embuild/espressif/python_env/idf5.3_py3.13_env/bin/esptool.py \
    --chip esp32s3 --port /dev/cu.usbmodem101 --baud 460800 \
    write_flash \
    0x0     target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-f8090498544b0ecf/out/build/bootloader/bootloader.bin \
    0x8000  target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-f8090498544b0ecf/out/build/partition_table/partition-table.bin \
    0x10000 target/xtensa-esp32s3-espidf/release/build/esp-idf-sys-f8090498544b0ecf/out/build/libespidf.bin
```

## Expected Output After Flashing

### Boot Banner Should Show:
```
ESP-ROM:esp32s3-20210327
Build:Mar 27 2021
...
I (48) boot: ESP-IDF v5.3.3 3rd stage bootloader
I (48) boot: Compiled Jul 20 2025 20:16:xx
```

### If ESP_LCD Works:
```
[INFO] ESP32-S3 Dashboard v5.52-finalFix - OTA on Port 80
[INFO] === ESP_LCD Display Manager Final Fix ===
[INFO] I80 bus created successfully
[INFO] ST7789 panel created successfully
[DEBUG] DMA buffer address: 0x3fc... (32-byte aligned: true)
```

## Key Changes From Previous Attempts

1. **Bootloader Path**: Using the correct path in build directory
2. **App Binary**: Using `libespidf.bin` instead of ELF file
3. **CONFIG_APP_RODATA_SEGMENT_MERGE=y** added to sdkconfig.defaults

## Monitor Command
After successful flash:
```bash
./scripts/monitor.sh
```

Look for v5.3.3 bootloader and ESP_LCD initialization messages.