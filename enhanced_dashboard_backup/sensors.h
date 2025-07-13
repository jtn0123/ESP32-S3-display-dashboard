#ifndef SENSORS_H
#define SENSORS_H

#include <Arduino.h>
#include <Wire.h>

// Sensor Management System for T-Display-S3 Dashboard
// Phase 3D: Sensor Integration & Data Logging

// Sensor types
enum SensorType {
  SENSOR_BATTERY,
  SENSOR_TOUCH,
  SENSOR_SYSTEM,
  SENSOR_I2C_BME280,
  SENSOR_I2C_BME680,
  SENSOR_I2C_SHT30,
  SENSOR_ANALOG,
  SENSOR_DIGITAL
};

// Sensor data structure
struct SensorData {
  SensorType type;
  String name;
  float value;
  String unit;
  unsigned long timestamp;
  bool valid;
  float minValue;
  float maxValue;
  float avgValue;
  int readingCount;
};

// Sensor configuration
struct SensorConfig {
  SensorType type;
  String name;
  int pin;
  int i2cAddress;
  bool enabled;
  int readInterval;
  unsigned long lastRead;
  float calibrationOffset;
  float calibrationMultiplier;
};

// Data logging configuration
#define MAX_SENSORS 10
#define MAX_LOG_ENTRIES 100
#define LOG_INTERVAL_MS 5000  // Log every 5 seconds

// Sensor readings storage
extern SensorData sensors[MAX_SENSORS];
extern SensorConfig sensorConfigs[MAX_SENSORS];
extern int activeSensorCount;

// Data logging arrays
extern float batteryLog[MAX_LOG_ENTRIES];
extern float temperatureLog[MAX_LOG_ENTRIES];
extern float humidityLog[MAX_LOG_ENTRIES];
extern int logIndex;
extern bool logFull;

// Sensor management functions
void initSensorSystem();
void updateSensorSystem();
void readAllSensors();
void readSensor(int sensorIndex);
void logSensorData();

// Battery monitoring
void initBatteryMonitoring();
float readBatteryVoltage();
int getBatteryPercentage();

// Touch sensor monitoring
void readTouchSensors();
int getTouchValue(int pin);

// System sensors
void readSystemSensors();
float getCPUTemperature();
int getFreeMemoryPercent();
float getWiFiSignalStrength();

// I2C sensor support
void initI2CSensors();
bool detectI2CDevice(int address);
void scanI2CBus();

// BME280 sensor support (if connected)
bool initBME280();
float readBME280Temperature();
float readBME280Humidity();
float readBME280Pressure();

// Analog sensor support
void initAnalogSensors();
float readAnalogSensor(int pin);

// Data logging functions
void initDataLogging();
void addLogEntry(float battery, float temperature, float humidity);
void clearLogs();
void exportLogs();

// Data visualization helpers
void drawSensorGraph(int x, int y, int w, int h, float* data, int dataLength, String title, String unit);
void drawMiniGraph(int x, int y, int w, int h, float* data, int dataLength, uint16_t color);
float getLogMin(float* data, int length);
float getLogMax(float* data, int length);
float getLogAverage(float* data, int length);

// Sensor calibration
void calibrateSensor(int sensorIndex);
void saveSensorCalibration();
void loadSensorCalibration();

// Implementation
SensorData sensors[MAX_SENSORS];
SensorConfig sensorConfigs[MAX_SENSORS];
int activeSensorCount = 0;

// Data logging arrays
float batteryLog[MAX_LOG_ENTRIES];
float temperatureLog[MAX_LOG_ENTRIES];
float humidityLog[MAX_LOG_ENTRIES];
int logIndex = 0;
bool logFull = false;

void initSensorSystem() {
  Serial.println("=== Initializing Sensor System ===");
  
  // Initialize I2C
  Wire.begin(21, 22); // SDA=21, SCL=22
  
  // Initialize sensor configurations
  activeSensorCount = 0;
  
  // Battery sensor (always available)
  sensorConfigs[activeSensorCount] = {
    SENSOR_BATTERY, "Battery", 4, 0, true, 2000, 0, 0.0, 1.0
  };
  activeSensorCount++;
  
  // System sensors (always available)
  sensorConfigs[activeSensorCount] = {
    SENSOR_SYSTEM, "CPU Temp", 0, 0, true, 5000, 0, 0.0, 1.0
  };
  activeSensorCount++;
  
  sensorConfigs[activeSensorCount] = {
    SENSOR_SYSTEM, "Free Memory", 0, 0, true, 3000, 0, 0.0, 1.0
  };
  activeSensorCount++;
  
  // Touch sensors (first 3 touch pins)
  for (int i = 0; i < 3; i++) {
    sensorConfigs[activeSensorCount] = {
      SENSOR_TOUCH, "Touch " + String(i), i + 1, 0, true, 1000, 0, 0.0, 1.0
    };
    activeSensorCount++;
  }
  
  // Initialize specific sensor types
  initBatteryMonitoring();
  initI2CSensors();
  initAnalogSensors();
  initDataLogging();
  
  Serial.println("Sensor system initialized with " + String(activeSensorCount) + " sensors");
}

void updateSensorSystem() {
  unsigned long now = millis();
  
  // Read sensors based on their update intervals
  for (int i = 0; i < activeSensorCount; i++) {
    if (sensorConfigs[i].enabled && 
        (now - sensorConfigs[i].lastRead) >= sensorConfigs[i].readInterval) {
      
      readSensor(i);
      sensorConfigs[i].lastRead = now;
    }
  }
  
  // Log data periodically
  static unsigned long lastLogTime = 0;
  if (now - lastLogTime >= LOG_INTERVAL_MS) {
    logSensorData();
    lastLogTime = now;
  }
}

void readSensor(int sensorIndex) {
  if (sensorIndex >= activeSensorCount) return;
  
  SensorConfig& config = sensorConfigs[sensorIndex];
  SensorData& data = sensors[sensorIndex];
  
  data.type = config.type;
  data.name = config.name;
  data.timestamp = millis();
  data.valid = false;
  
  switch (config.type) {
    case SENSOR_BATTERY:
      data.value = readBatteryVoltage();
      data.unit = "V";
      data.valid = true;
      break;
      
    case SENSOR_TOUCH:
      data.value = getTouchValue(config.pin);
      data.unit = "";
      data.valid = true;
      break;
      
    case SENSOR_SYSTEM:
      if (config.name == "CPU Temp") {
        data.value = getCPUTemperature();
        data.unit = "°C";
        data.valid = true;
      } else if (config.name == "Free Memory") {
        data.value = getFreeMemoryPercent();
        data.unit = "%";
        data.valid = true;
      }
      break;
      
    case SENSOR_ANALOG:
      data.value = readAnalogSensor(config.pin);
      data.unit = "V";
      data.valid = true;
      break;
      
    default:
      data.valid = false;
      break;
  }
  
  // Update statistics
  if (data.valid) {
    if (data.readingCount == 0) {
      data.minValue = data.value;
      data.maxValue = data.value;
      data.avgValue = data.value;
    } else {
      data.minValue = min(data.minValue, data.value);
      data.maxValue = max(data.maxValue, data.value);
      data.avgValue = (data.avgValue * data.readingCount + data.value) / (data.readingCount + 1);
    }
    data.readingCount++;
  }
}

void initBatteryMonitoring() {
  // Configure GPIO4 for battery voltage measurement
  pinMode(4, INPUT);
  
  // Set up ADC for battery monitoring
  analogReadResolution(12); // 12-bit ADC resolution
  analogSetAttenuation(ADC_11db); // Allow reading up to 3.3V
  
  Serial.println("Battery monitoring initialized on GPIO4");
}

float readBatteryVoltage() {
  // Read raw ADC value
  int rawValue = analogRead(4);
  
  // Convert to voltage (ESP32-S3 ADC reference is 3.3V)
  float voltage = (rawValue / 4095.0) * 3.3;
  
  // Apply voltage divider correction if needed
  // (T-Display-S3 may have voltage divider on battery input)
  voltage = voltage * 2.0; // Assuming 2:1 voltage divider
  
  return voltage;
}

int getBatteryPercentage() {
  float voltage = readBatteryVoltage();
  
  // LiPo battery voltage range: 3.0V (0%) to 4.2V (100%)
  float minVoltage = 3.0;
  float maxVoltage = 4.2;
  
  int percentage = ((voltage - minVoltage) / (maxVoltage - minVoltage)) * 100;
  
  // Clamp to 0-100%
  percentage = max(0, min(100, percentage));
  
  return percentage;
}

int getTouchValue(int pin) {
  // Map touch pin index to actual GPIO
  int touchPin = pin; // Assuming direct mapping for now
  return touchRead(touchPin);
}

float getCPUTemperature() {
  // Approximate CPU temperature based on chip revision and load
  // ESP32-S3 doesn't have built-in temperature sensor
  float baseTemp = 25.0; // Room temperature baseline
  float loadFactor = (100 - getFreeMemoryPercent()) * 0.3; // Memory load affects temperature
  
  return baseTemp + loadFactor;
}

int getFreeMemoryPercent() {
  int freeHeap = ESP.getFreeHeap();
  int totalHeap = ESP.getHeapSize();
  
  return (freeHeap * 100) / totalHeap;
}

float getWiFiSignalStrength() {
  extern WiFiStatus currentWiFiStatus;
  if (currentWiFiStatus == WIFI_CONNECTED) {
    return WiFi.RSSI();
  }
  return 0.0;
}

void initI2CSensors() {
  Serial.println("Scanning I2C bus for sensors...");
  scanI2CBus();
  
  // Try to initialize common sensors
  initBME280();
}

bool detectI2CDevice(int address) {
  Wire.beginTransmission(address);
  byte error = Wire.endTransmission();
  return (error == 0);
}

void scanI2CBus() {
  Serial.println("I2C device scan:");
  int deviceCount = 0;
  
  for (int address = 1; address < 127; address++) {
    if (detectI2CDevice(address)) {
      Serial.print("I2C device found at address 0x");
      if (address < 16) Serial.print("0");
      Serial.print(address, HEX);
      Serial.println();
      deviceCount++;
      
      // Add known sensor types
      if (address == 0x76 || address == 0x77) {
        Serial.println("  -> BME280/BME680 detected");
      } else if (address == 0x44) {
        Serial.println("  -> SHT30 detected");
      }
    }
  }
  
  if (deviceCount == 0) {
    Serial.println("No I2C devices found");
  } else {
    Serial.println("I2C scan complete");
  }
}

bool initBME280() {
  // Check if BME280 is present
  if (detectI2CDevice(0x76) || detectI2CDevice(0x77)) {
    Serial.println("BME280 sensor detected but driver not implemented");
    // TODO: Add BME280 driver implementation
    return false;
  }
  return false;
}

void initAnalogSensors() {
  // Configure analog pins for sensor reading
  Serial.println("Analog sensors ready");
}

float readAnalogSensor(int pin) {
  int rawValue = analogRead(pin);
  float voltage = (rawValue / 4095.0) * 3.3;
  return voltage;
}

void initDataLogging() {
  // Clear log arrays
  for (int i = 0; i < MAX_LOG_ENTRIES; i++) {
    batteryLog[i] = 0.0;
    temperatureLog[i] = 0.0;
    humidityLog[i] = 0.0;
  }
  
  logIndex = 0;
  logFull = false;
  
  Serial.println("Data logging initialized");
}

void logSensorData() {
  // Find battery and temperature sensors
  float batteryVoltage = 0.0;
  float temperature = 0.0;
  float humidity = 0.0;
  
  for (int i = 0; i < activeSensorCount; i++) {
    if (sensors[i].valid) {
      if (sensors[i].type == SENSOR_BATTERY) {
        batteryVoltage = sensors[i].value;
      } else if (sensors[i].type == SENSOR_SYSTEM && sensors[i].name == "CPU Temp") {
        temperature = sensors[i].value;
      }
    }
  }
  
  addLogEntry(batteryVoltage, temperature, humidity);
}

void addLogEntry(float battery, float temperature, float humidity) {
  batteryLog[logIndex] = battery;
  temperatureLog[logIndex] = temperature;
  humidityLog[logIndex] = humidity;
  
  logIndex++;
  if (logIndex >= MAX_LOG_ENTRIES) {
    logIndex = 0;
    logFull = true;
  }
}

void drawSensorGraph(int x, int y, int w, int h, float* data, int dataLength, String title, String unit) {
  extern void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
  extern void drawString(int x, int y, String text, uint16_t color, FontSize size);
  extern ColorTheme currentTheme;
  
  // Draw graph background
  fillVisibleRect(x, y, w, h, currentTheme.surface);
  
  // Draw title
  drawString(x + 2, y + 2, title, currentTheme.textPrimary, FONT_SMALL);
  
  // Find data range
  float minVal = getLogMin(data, dataLength);
  float maxVal = getLogMax(data, dataLength);
  
  if (maxVal <= minVal) return; // No valid data
  
  // Draw data points
  int startIndex = logFull ? logIndex : 0;
  int endIndex = logFull ? logIndex + MAX_LOG_ENTRIES : logIndex;
  
  for (int i = 1; i < min(dataLength, w - 4); i++) {
    int dataIndex = (startIndex + i) % MAX_LOG_ENTRIES;
    int prevIndex = (startIndex + i - 1) % MAX_LOG_ENTRIES;
    
    if (dataIndex >= 0 && dataIndex < MAX_LOG_ENTRIES && 
        prevIndex >= 0 && prevIndex < MAX_LOG_ENTRIES) {
      
      float val1 = data[prevIndex];
      float val2 = data[dataIndex];
      
      if (val1 != 0.0 && val2 != 0.0) {
        int y1 = y + h - 2 - ((val1 - minVal) / (maxVal - minVal)) * (h - 4);
        int y2 = y + h - 2 - ((val2 - minVal) / (maxVal - minVal)) * (h - 4);
        
        // Draw line (simplified)
        fillVisibleRect(x + 2 + i - 1, y1, 1, 1, currentTheme.info);
        fillVisibleRect(x + 2 + i, y2, 1, 1, currentTheme.info);
      }
    }
  }
  
  // Draw current value
  String valueText = String(data[(logIndex - 1 + MAX_LOG_ENTRIES) % MAX_LOG_ENTRIES], 1) + unit;
  drawString(x + w - 40, y + h - 12, valueText, currentTheme.warning, FONT_SMALL);
}

float getLogMin(float* data, int length) {
  float minVal = data[0];
  for (int i = 1; i < length; i++) {
    if (data[i] != 0.0 && data[i] < minVal) {
      minVal = data[i];
    }
  }
  return minVal;
}

float getLogMax(float* data, int length) {
  float maxVal = data[0];
  for (int i = 1; i < length; i++) {
    if (data[i] > maxVal) {
      maxVal = data[i];
    }
  }
  return maxVal;
}

float getLogAverage(float* data, int length) {
  float sum = 0.0;
  int count = 0;
  
  for (int i = 0; i < length; i++) {
    if (data[i] != 0.0) {
      sum += data[i];
      count++;
    }
  }
  
  return count > 0 ? sum / count : 0.0;
}

#endif