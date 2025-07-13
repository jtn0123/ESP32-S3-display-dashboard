# Makefile for ESP32-S3 Dashboard
# Usage: make upload, make compile, make clean

BOARD = esp32:esp32:lilygo_t_display_s3
PORT = /dev/cu.usbmodem101
SKETCH_DIR = dashboard

.PHONY: upload compile clean monitor help

help:
	@echo "ESP32-S3 Dashboard Make Commands:"
	@echo "  make compile  - Compile the sketch"
	@echo "  make upload   - Compile and upload to board"
	@echo "  make clean    - Clean build cache"
	@echo "  make monitor  - Open serial monitor"
	@echo "  make all      - Clean, compile, and upload"

compile:
	@echo "Compiling sketch..."
	@cd $(SKETCH_DIR) && arduino-cli compile --fqbn $(BOARD) .

upload:
	@echo "Compiling and uploading..."
	@cd $(SKETCH_DIR) && arduino-cli compile --fqbn $(BOARD) . && \
	arduino-cli upload -p $(PORT) --fqbn $(BOARD) .

clean:
	@echo "Cleaning build cache..."
	@rm -rf ~/Library/Caches/arduino/sketches/*

monitor:
	@echo "Opening serial monitor..."
	@arduino-cli monitor -p $(PORT)

all: clean compile upload
	@echo "Build and upload complete!"