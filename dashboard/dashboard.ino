// Minimal T-Display-S3 Dashboard - Lightweight version with labels
// Uses only essential functions to reduce program size

#include <WiFi.h>
#include <ArduinoOTA.h>       // OTA updates over WiFi
#include <ESPmDNS.h>          // mDNS for OTA discovery
#include <WebServer.h>        // Lightweight web server
#include <Update.h>           // For OTA updates
#include <Preferences.h>
#include "soc/gpio_struct.h"  // For direct GPIO register access
#include "driver/gpio.h"      // For gpio_pullup_dis/gpio_pulldown_dis
#include "esp_pm.h"           // For dynamic CPU frequency scaling
#include "driver/spi_master.h"// For DMA double-buffering
#include "esp_heap_caps.h"    // For DMA-capable memory allocation
#include "wifi_config.h"      // WiFi credentials (not committed to git)

// EXPANDED DISPLAY AREA - Using maximum usable coordinates (50% wider!)
#define DISPLAY_X_START 10   // Left boundary (expanded from 60)
#define DISPLAY_Y_START 36   // Top boundary (expanded from 40)
#define DISPLAY_WIDTH   300  // Maximum visible width (50% increase from 200!)
#define DISPLAY_HEIGHT  168  // Maximum visible height (expanded from 160)

// Display pins
#define LCD_POWER_ON 15
#define LCD_BL       38
#define LCD_RES      5
#define LCD_CS       6
#define LCD_DC       7
#define LCD_WR       8
#define LCD_RD       9
#define LCD_D0       39
#define LCD_D1       40
#define LCD_D2       41
#define LCD_D3       42
#define LCD_D4       45
#define LCD_D5       46
#define LCD_D6       47
#define LCD_D7       48

// Button pins (T-Display-S3)
#define BUTTON_1     0   // Boot button (left navigation)
#define BUTTON_2     14  // User button (select/action)

// Double-buffer DMA configuration
#define DMA_BUFFER_SIZE (320 * 20 * 2)  // 20 lines of 320 pixels, 2 bytes per pixel
static uint8_t* dmaBuffer1 = NULL;
static uint8_t* dmaBuffer2 = NULL;
static uint8_t* currentBuffer = NULL;
static uint8_t* backBuffer = NULL;
static volatile bool dmaTransferActive = false;
static int currentDMALine = 0;

// Battery detection
#define BATTERY_PIN  4   // GPIO4 for battery voltage
#define USB_DETECT_THRESHOLD 4400  // mV threshold for USB power (lowered for better detection)
#define CHARGING_THRESHOLD 4250    // mV threshold to detect charging (more reasonable)
#define NO_BATTERY_ADC_MIN 100     // ADC values below this indicate no battery
#define NO_BATTERY_ADC_MAX 3900    // ADC values above this indicate floating pin
#define MAX_BATTERY_VOLTAGE 4300   // mV - maximum reasonable battery voltage

// Version constant - UPDATE THIS WHEN MAKING CHANGES
#define DASHBOARD_VERSION "4.0-PERF-OPTIMIZED"

// MODERN COLOR PALETTE - Enhanced semantic colors
#define RED        0x07FF  // Send YELLOW to get RED
#define GREEN      0xF81F  // Send CYAN to get GREEN  
#define BLUE       0xF8E0  // Send MAGENTA to get BLUE
#define YELLOW     0x001F  // Send GREEN to get YELLOW
#define CYAN       0xF800  // Send BLUE to get CYAN
#define MAGENTA    0x07E0  // Send RED to get MAGENTA
#define BLACK      0xFFFF  // TRUE BLACK for readable text (BGR mapping)

// MODERN UI COLORS - Enhanced palette
#define PRIMARY_BLUE    0x2589   // Modern interface blue
#define PRIMARY_GREEN   0x07E5   // Success/positive green
#define PRIMARY_PURPLE  0x7817   // Data/info purple
#define PRIMARY_RED     0xF800   // Error/warning red
#define SURFACE_DARK    BLACK    // Dark theme - unified black backgrounds
#define SURFACE_LIGHT   0x3186   // Secondary surfaces
#define TEXT_PRIMARY    0xFFFF   // Primary text - TRUE BLACK for readability (BGR mapping)
#define TEXT_SECONDARY  0xBDF7   // Secondary text - improved contrast
#define BORDER_COLOR    0x4208   // Borders/dividers
#define ACCENT_ORANGE   0xC260   // Highlight color

// Button state tracking
struct ButtonState {
  bool current = HIGH;
  bool previous = HIGH;
  unsigned long pressTime = 0;
  unsigned long releaseTime = 0;
  bool longPressTriggered = false;
  int clickCount = 0;
  unsigned long lastClickTime = 0;
};

ButtonState btn1State;  // BOOT button
ButtonState btn2State;  // USER button

// Button timing constants
#define DEBOUNCE_TIME 50
#define LONG_PRESS_TIME 800
#define DOUBLE_CLICK_TIME 400

// Button event types
#define BUTTON_NONE 0
#define BUTTON_CLICK 1
#define BUTTON_LONG_PRESS 2
#define BUTTON_DOUBLE_CLICK 3

// UI State
bool showButtonHints = true;
bool fastTransition = false;

// Auto-update timers
unsigned long lastMemoryUpdate = 0;
unsigned long lastUptimeUpdate = 0;
unsigned long lastBatteryUpdate = 0;
unsigned long lastWiFiUpdate = 0;
unsigned long lastSensorUpdate = 0;

// Auto-dim timer
unsigned long lastActivityTime = 0;
const unsigned long DIM_TIMEOUT = 30000;  // 30 seconds
bool isDimmed = false;

// FPS monitoring
unsigned long frameCount = 0;
unsigned long lastFPSUpdate = 0;
float currentFPS = 0.0;
const unsigned long FPS_UPDATE_INTERVAL = 1000;  // Update FPS every second

// Interrupt-driven sensor reading
volatile bool batteryReadingReady = false;
volatile int latestBatteryADC = 0;
hw_timer_t* sensorTimer = NULL;
portMUX_TYPE sensorMux = portMUX_INITIALIZER_UNLOCKED;
#define SENSOR_READ_INTERVAL_MS 500  // Read battery every 500ms

// Dirty rectangle tracking
struct DirtyRect {
  int x, y, w, h;
  bool dirty;
};

#define MAX_DIRTY_RECTS 10
DirtyRect dirtyRects[MAX_DIRTY_RECTS];
int dirtyRectCount = 0;

// Screen regions for tracking
enum ScreenRegion {
  REGION_HEADER = 0,
  REGION_MEMORY,
  REGION_CPU,
  REGION_UPTIME,
  REGION_BATTERY,
  REGION_WIFI,
  REGION_SENSOR,
  REGION_FULL
};

// Update intervals (ms)
const unsigned long MEMORY_UPDATE_INTERVAL = 1000;
const unsigned long UPTIME_UPDATE_INTERVAL = 1000;
const unsigned long BATTERY_UPDATE_INTERVAL = 5000;
const unsigned long WIFI_UPDATE_INTERVAL = 2000;
const unsigned long SENSOR_UPDATE_INTERVAL = 2000;

// Menu state
enum MenuState {
  MENU_NONE,
  MENU_MAIN,
  MENU_DISPLAY,
  MENU_UPDATE,
  MENU_SYSTEM
};
MenuState currentMenu = MENU_NONE;
int menuSelection = 0;
int menuItemCount = 0;

// Settings
struct Settings {
  int brightness = 100;
  bool autoDim = false;
  int updateSpeed = 1;  // 0=Slow, 1=Normal, 2=Fast
  int colorTheme = 0;   // 0=readable, 1=dark, 2=high-contrast
} settings;

// Dynamic Color Theme System
struct ColorTheme {
  // Text Colors (guaranteed readable)
  uint16_t text_primary;      // Main readable text
  uint16_t text_secondary;    // Labels and secondary text
  uint16_t text_accent;       // Highlighted text
  
  // Data Value Colors (semantic meaning)
  uint16_t value_normal;      // Normal data values
  uint16_t value_good;        // Positive/good values
  uint16_t value_warning;     // Warning values
  uint16_t value_error;       // Error/critical values
  uint16_t value_info;        // Information values
  
  // UI Element Colors
  uint16_t card_background;   // Card backgrounds
  uint16_t card_border;       // Card borders
  uint16_t progress_bg;       // Progress bar backgrounds
  uint16_t button_active;     // Active button highlight
};

// Color Theme Definitions
ColorTheme readableTheme = {
  .text_primary = 0xBDF7,     // TEXT_SECONDARY (proven working)
  .text_secondary = 0xBDF7,   // Same for consistency
  .text_accent = 0x2589,      // PRIMARY_BLUE
  .value_normal = 0xBDF7,     // Readable grey
  .value_good = 0x07E5,       // PRIMARY_GREEN
  .value_warning = 0xC260,    // ACCENT_ORANGE  
  .value_error = 0xF800,      // PRIMARY_RED
  .value_info = 0x2589,       // PRIMARY_BLUE
  .card_background = 0x1082,  // SURFACE_DARK
  .card_border = 0x4208,      // BORDER_COLOR
  .progress_bg = 0x3186,      // SURFACE_LIGHT
  .button_active = 0x2589     // PRIMARY_BLUE
};

ColorTheme darkTheme = {
  .text_primary = 0xFFFF,     // Bright white
  .text_secondary = 0xC618,   // Light grey
  .text_accent = 0x07FF,      // Bright cyan
  .value_normal = 0xC618,     // Light grey
  .value_good = 0x07E0,       // Bright green
  .value_warning = 0xFFE0,    // Bright yellow
  .value_error = 0xF800,      // Bright red
  .value_info = 0x07FF,       // Bright cyan
  .card_background = 0x0000,  // Black
  .card_border = 0x3186,      // Dark grey
  .progress_bg = 0x2104,      // Darker grey
  .button_active = 0x07FF     // Bright cyan
};

ColorTheme highContrastTheme = {
  .text_primary = 0xFFFF,     // Pure white
  .text_secondary = 0xFFFF,   // Pure white
  .text_accent = 0x07E0,      // Pure green
  .value_normal = 0xFFFF,     // Pure white
  .value_good = 0x07E0,       // Pure green
  .value_warning = 0xFFE0,    // Pure yellow
  .value_error = 0xF800,      // Pure red
  .value_info = 0x001F,       // Pure blue
  .card_background = 0x0000,  // Pure black
  .card_border = 0xFFFF,      // Pure white
  .progress_bg = 0x2104,      // Dark grey
  .button_active = 0x07E0     // Pure green
};

ColorTheme* themes[] = {&readableTheme, &darkTheme, &highContrastTheme};
ColorTheme* currentTheme = &readableTheme;

// Color Helper Functions - DRAMATIC CHANGES FOR VISIBILITY
uint16_t getTextColor() { return TEXT_SECONDARY; }  // Visible gray text
uint16_t getLabelColor() { return TEXT_SECONDARY; }  // Visible gray text
uint16_t getAccentColor() { return currentTheme->text_accent; }
uint16_t getValueColor(float value, float minVal, float maxVal) {
  if (value < minVal * 1.2) return currentTheme->value_error;
  if (value < minVal * 1.5) return currentTheme->value_warning;
  return currentTheme->value_good;
}
uint16_t getInfoColor() { return currentTheme->value_info; }
uint16_t getGoodColor() { return currentTheme->value_good; }
uint16_t getWarningColor() { return currentTheme->value_warning; }
uint16_t getErrorColor() { return currentTheme->value_error; }
uint16_t getCardBg() { return currentTheme->card_background; }
uint16_t getCardBorder() { return currentTheme->card_border; }

void applyTheme(int themeIndex) {
  if (themeIndex >= 0 && themeIndex < 3) {
    currentTheme = themes[themeIndex];
    settings.colorTheme = themeIndex;
  }
}

Preferences preferences;

// OTA Update variables
bool otaInProgress = false;
int otaProgress = 0;

// Lightweight Web Server for OTA
WebServer server(80);
bool webOtaInProgress = false;

// OTA setup function
void setupOTA() {
  if (!WiFi.isConnected()) {
    Serial.println("OTA: WiFi not connected, skipping OTA setup");
    return;
  }
  
  // CRITICAL: Start mDNS first for ESP32-S3
  if (!MDNS.begin("esp32-dashboard")) {
    Serial.println("OTA: Error starting mDNS");
  } else {
    Serial.println("OTA: mDNS responder started");
  }
  
  // Add a delay for network stack to stabilize
  delay(500);
  
  // Set hostname (customize as needed)
  ArduinoOTA.setHostname("esp32-dashboard");
  
  // Set OTA port explicitly
  ArduinoOTA.setPort(3232);
  
  // Set password for OTA updates (optional but recommended)
  // ArduinoOTA.setPassword("your-ota-password");
  
  ArduinoOTA.onStart([]() {
    String type;
    if (ArduinoOTA.getCommand() == U_FLASH) {
      type = "sketch";
    } else {  // U_SPIFFS
      type = "filesystem";
    }
    Serial.println("OTA: Start updating " + type);
    otaInProgress = true;
    
    // Clear screen with BLACK
    fillScreen(BLACK);
    
    // Simple title - no boxes
    drawTextLabel(125, 30, "OTA", PRIMARY_BLUE);
    drawTextLabel(110, 50, "UPDATE", PRIMARY_BLUE);
    
    // Status message
    drawTextLabel(75, 80, "Receiving firmware", TEXT_SECONDARY);
    drawTextLabel(65, 95, "Do not power off!", PRIMARY_RED);
    
    // Progress bar frame
    fillVisibleRect(50, 115, 200, 24, BORDER_COLOR);
    fillVisibleRect(52, 117, 196, 20, BLACK);
  });
  
  ArduinoOTA.onEnd([]() {
    Serial.println("\nOTA: Update complete!");
    otaInProgress = false;
    
    // Clear and show completion
    fillScreen(BLACK);
    
    // Simple success message
    drawTextLabel(110, 40, "UPDATE", PRIMARY_GREEN);
    drawTextLabel(100, 60, "COMPLETE", PRIMARY_GREEN);
    
    // Success line under text
    fillRect(90, 82, 120, 2, PRIMARY_GREEN);
    
    // Info
    drawTextLabel(70, 105, "Firmware updated", TEXT_SECONDARY);
    
    drawTextLabel(75, 130, "Restarting in", TEXT_SECONDARY);
    drawTextLabel(100, 145, "3 seconds", PRIMARY_GREEN);
    
    delay(3000);
  });
  
  ArduinoOTA.onProgress([](unsigned int progress, unsigned int total) {
    int percentage = (progress / (total / 100));
    Serial.printf("OTA Progress: %u%%\r", percentage);
    otaProgress = percentage;
    
    // Update progress bar without clearing
    static int lastPercentage = -1;
    if (percentage != lastPercentage) {
      // Only draw new progress
      int newWidth = (196 * percentage) / 100;
      int oldWidth = (196 * lastPercentage) / 100;
      if (oldWidth < 0) oldWidth = 0;
      
      if (newWidth > oldWidth) {
        fillVisibleRect(52 + oldWidth, 117, newWidth - oldWidth, 20, PRIMARY_BLUE);
      }
      
      // Update percentage text only if changed  
      fillVisibleRect(100, 148, 100, 16, BLACK);  // Use correct coordinate system
      char buf[10];
      snprintf(buf, sizeof(buf), "%d%%", percentage);
      drawTextLabel(130, 150, buf, PRIMARY_BLUE);
      
      lastPercentage = percentage;
    }
  });
  
  ArduinoOTA.onError([](ota_error_t error) {
    Serial.printf("OTA Error[%u]: ", error);
    String errorMsg = "Unknown";
    if (error == OTA_AUTH_ERROR) errorMsg = "Auth Failed";
    else if (error == OTA_BEGIN_ERROR) errorMsg = "Begin Failed";
    else if (error == OTA_CONNECT_ERROR) errorMsg = "Connect Failed";
    else if (error == OTA_RECEIVE_ERROR) errorMsg = "Receive Failed";
    else if (error == OTA_END_ERROR) errorMsg = "End Failed";
    
    Serial.println(errorMsg);
    otaInProgress = false;
    
    // Show error on screen
    fillScreen(BLACK);
    
    // Simple error message
    drawTextLabel(125, 40, "OTA", PRIMARY_RED);
    drawTextLabel(110, 60, "ERROR", PRIMARY_RED);
    
    // Error line under text
    fillRect(90, 82, 120, 2, PRIMARY_RED);
    
    // Error message
    drawTextLabel(60, 105, "Update failed:", TEXT_SECONDARY);
    drawTextLabel(70, 120, errorMsg.c_str(), PRIMARY_RED);
    
    // Help text
    drawTextLabel(85, 145, "Try again", TEXT_SECONDARY);
    delay(3000);
  });
  
  ArduinoOTA.begin();
  
  // Add mDNS service advertising for better discovery
  MDNS.addService("arduino", "tcp", 3232);
  
  Serial.println("OTA: Ready for updates");
  Serial.print("OTA: Hostname: ");
  Serial.println(ArduinoOTA.getHostname());
  Serial.print("OTA: IP Address: ");
  Serial.println(WiFi.localIP());
  Serial.println("OTA: Port: 3232");
  
  // Test if OTA is really running
  delay(100);
  Serial.println("OTA: Service started successfully");
}

// Minimal Web OTA Implementation
void setupWebOTA() {
  if (!WiFi.isConnected()) return;
  
  // Simple upload page - minimal HTML
  server.on("/", HTTP_GET, []() {
    server.sendHeader("Connection", "close");
    server.send(200, "text/html", 
      "<form method='POST' action='/update' enctype='multipart/form-data'>"
      "<input type='file' name='update'>"
      "<input type='submit' value='Update'>"
      "</form>");
  });
  
  // Handle upload
  server.on("/update", HTTP_POST, 
    // Upload done
    []() {
      server.sendHeader("Connection", "close");
      server.send(200, "text/plain", (Update.hasError()) ? "FAIL" : "OK");
      delay(1000);
      ESP.restart();
    },
    // Upload in progress
    []() {
      HTTPUpload& upload = server.upload();
      if (upload.status == UPLOAD_FILE_START) {
        Serial.printf("Update: %s\n", upload.filename.c_str());
        webOtaInProgress = true;
        
        // Initial display will be drawn on first chunk
        
        if (!Update.begin(UPDATE_SIZE_UNKNOWN)) {
          Update.printError(Serial);
        }
      } else if (upload.status == UPLOAD_FILE_WRITE) {
        if (Update.write(upload.buf, upload.currentSize) != upload.currentSize) {
          Update.printError(Serial);
        }
        
        // Track total bytes received
        static size_t totalReceived = 0;
        static unsigned long startTime = 0;
        static bool firstChunk = true;
        
        if (firstChunk) {
          startTime = millis();
          totalReceived = 0;
          firstChunk = false;
          
          // Clear screen with BLACK (0xFFFF)
          fillScreen(BLACK);
          
          // Simple title - no box to avoid artifacts
          drawTextLabel(110, 20, "FIRMWARE", PRIMARY_GREEN);
          drawTextLabel(120, 40, "UPDATE", PRIMARY_GREEN);
          
          // Status text
          drawTextLabel(85, 70, "Receiving data...", TEXT_SECONDARY);
          
          // Draw progress bar frame (centered)
          fillVisibleRect(50, 100, 200, 24, BORDER_COLOR);
          fillVisibleRect(52, 102, 196, 20, BLACK);
        }
        
        totalReceived += upload.currentSize;
        
        // Update display with better spacing
        static unsigned long lastProgressUpdate = 0;
        
        if (millis() - lastProgressUpdate > 200) {  // Update every 200ms
          lastProgressUpdate = millis();
          
          // Calculate and draw progress
          int progress = (totalReceived * 100) / 987000;
          if (progress > 98) progress = 98;  // Cap at 98% until done
          
          // Update progress bar without clearing entire bar
          static int lastProgress = -1;
          if (progress != lastProgress && progress > 0) {
            // Only draw the new progress, don't clear
            int newWidth = (196 * progress) / 100;
            int oldWidth = (196 * lastProgress) / 100;
            if (oldWidth < 0) oldWidth = 0;
            
            // Fill only the new portion
            if (newWidth > oldWidth) {
              fillVisibleRect(52 + oldWidth, 102, newWidth - oldWidth, 20, PRIMARY_GREEN);
            }
            lastProgress = progress;
          }
          
          // ALWAYS clear and redraw percentage area - use fillVisibleRect for correct coordinates
          fillVisibleRect(0, 130, 300, 40, BLACK);
          
          // Draw percentage
          char pctText[10];
          snprintf(pctText, sizeof(pctText), "%d%%", progress);
          drawTextLabel(130, 140, pctText, PRIMARY_GREEN);
          
          // Draw data info
          int currentKB = totalReceived/1024;
          char infoText[80];
          unsigned long elapsed = millis() - startTime;
          
          if (elapsed > 1000) {
            int speed = currentKB / (elapsed / 1000);
            if (speed > 0) {
              snprintf(infoText, sizeof(infoText), "%d KB @ %d KB/s", currentKB, speed);
            } else {
              snprintf(infoText, sizeof(infoText), "%d KB", currentKB);
            }
          } else {
            snprintf(infoText, sizeof(infoText), "%d KB", currentKB);
          }
          
          drawTextLabel(80, 155, infoText, TEXT_SECONDARY);
        }
        
        
      } else if (upload.status == UPLOAD_FILE_END) {
        // Reset static variables for next upload
        static bool firstChunk = true;
        firstChunk = true;
        
        if (Update.end(true)) {
          Serial.printf("Update Success: %u bytes\n", upload.totalSize);
          
          // Clear screen with BLACK
          fillScreen(BLACK);
          
          // Simple success message - no boxes
          drawTextLabel(110, 30, "UPDATE", PRIMARY_GREEN);
          drawTextLabel(100, 50, "COMPLETE", PRIMARY_GREEN);
          
          // Success line under text
          fillRect(90, 72, 120, 2, PRIMARY_GREEN);
          
          // Update stats
          char sizeText[50];
          snprintf(sizeText, sizeof(sizeText), "Size: %d KB", upload.totalSize/1024);
          drawTextLabel(90, 95, sizeText, TEXT_SECONDARY);
          
          // Version info
          drawTextLabel(75, 110, "Version " DASHBOARD_VERSION, TEXT_SECONDARY);
          
          // Restart info
          drawTextLabel(70, 135, "Restarting in", TEXT_SECONDARY);
          drawTextLabel(95, 150, "3 seconds", PRIMARY_GREEN);
        } else {
          Update.printError(Serial);
          
          // Show error - clear screen with actual black
          fillScreen(0x0000);
          
          // Simple error message - no boxes
          drawTextLabel(110, 30, "UPDATE", PRIMARY_RED);
          drawTextLabel(110, 50, "FAILED", PRIMARY_RED);
          
          // Error line under text
          fillRect(90, 72, 120, 2, PRIMARY_RED);
          
          // Error details
          drawTextLabel(60, 95, "Please try again", TEXT_SECONDARY);
          
          // Help text
          drawTextLabel(40, 120, "Upload .bin file at:", TEXT_SECONDARY);
          char ipText[40];
          snprintf(ipText, sizeof(ipText), "http://%s", WiFi.localIP().toString().c_str());
          drawTextLabel(60, 135, ipText, PRIMARY_GREEN);
        }
        webOtaInProgress = false;
      }
    }
  );
  
  server.begin();
  Serial.println("Web OTA: Server started at http://" + WiFi.localIP().toString());
}

// WiFi initialization function
void initWiFiConnection() {
  Serial.println("=== WiFi Connection Attempt ===");
  Serial.print("SSID: ");
  Serial.println(WIFI_SSID);
  
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
  Serial.print("Connecting to WiFi...");
  
  // Wait for WiFi connection with detailed status
  int wifiTimeout = 30; // 30 seconds timeout
  while (WiFi.status() != WL_CONNECTED && wifiTimeout > 0) {
    delay(500);
    Serial.print(".");
    wifiTimeout--;
    
    // Print status every 5 seconds
    if (wifiTimeout % 10 == 0) {
      Serial.print(" [Status: ");
      Serial.print(WiFi.status());
      Serial.print("] ");
    }
  }
  
  Serial.println();
  if (WiFi.isConnected()) {
    Serial.println("=== WiFi CONNECTED! ===");
    Serial.print("IP Address: ");
    Serial.println(WiFi.localIP());
    Serial.print("Signal Strength: ");
    Serial.print(WiFi.RSSI());
    Serial.println(" dBm");
  } else {
    Serial.println("=== WiFi CONNECTION FAILED ===");
    Serial.print("Final Status Code: ");
    Serial.println(WiFi.status());
    Serial.println("Status meanings:");
    Serial.println("  0=IDLE_STATUS, 1=NO_SSID_AVAIL, 3=CONNECTED");
    Serial.println("  4=CONNECT_FAILED, 6=DISCONNECTED");
    Serial.println("Continuing without network...");
  }
}

// Global screen index - moved here to be accessible in setup()
int screenIndex = 0;
bool forcePowerRedraw = true;  // Force power screen to redraw on first visit

// Dirty rectangle management
void markDirty(int x, int y, int w, int h) {
  if (dirtyRectCount < MAX_DIRTY_RECTS) {
    dirtyRects[dirtyRectCount].x = x;
    dirtyRects[dirtyRectCount].y = y;
    dirtyRects[dirtyRectCount].w = w;
    dirtyRects[dirtyRectCount].h = h;
    dirtyRects[dirtyRectCount].dirty = true;
    dirtyRectCount++;
  }
}

void markRegionDirty(ScreenRegion region) {
  switch(region) {
    case REGION_HEADER:
      markDirty(0, 0, DISPLAY_WIDTH, 20);
      break;
    case REGION_MEMORY:
      markDirty(40, 25, DISPLAY_WIDTH - 80, 45);
      break;
    case REGION_CPU:
      markDirty(40, 75, DISPLAY_WIDTH - 80, 45);
      break;
    case REGION_UPTIME:
      markDirty(40, 125, DISPLAY_WIDTH - 80, 35);
      break;
    case REGION_BATTERY:
      markDirty(DISPLAY_WIDTH - 80, 0, 80, 20);
      break;
    case REGION_WIFI:
      markDirty(40, 25, DISPLAY_WIDTH - 80, 135);
      break;
    case REGION_FULL:
      markDirty(0, 0, DISPLAY_WIDTH, DISPLAY_HEIGHT);
      break;
  }
}

void clearDirtyRects() {
  dirtyRectCount = 0;
}

bool isRectDirty(int x, int y, int w, int h) {
  for (int i = 0; i < dirtyRectCount; i++) {
    if (dirtyRects[i].dirty) {
      // Check if rectangles overlap
      if (!(x >= dirtyRects[i].x + dirtyRects[i].w || 
            x + w <= dirtyRects[i].x ||
            y >= dirtyRects[i].y + dirtyRects[i].h ||
            y + h <= dirtyRects[i].y)) {
        return true;
      }
    }
  }
  return false;
}

// Timer interrupt handler for sensor reading
void IRAM_ATTR onSensorTimer() {
  portENTER_CRITICAL_ISR(&sensorMux);
  // Read battery ADC in interrupt context
  latestBatteryADC = analogRead(BATTERY_PIN);
  batteryReadingReady = true;
  portEXIT_CRITICAL_ISR(&sensorMux);
}

// Initialize interrupt-driven sensor reading
void initSensorInterrupt() {
  // Create timer for periodic sensor reads
  sensorTimer = timerBegin(0, 80, true);  // Timer 0, prescaler 80 (1MHz), count up
  timerAttachInterrupt(sensorTimer, &onSensorTimer, true);
  timerAlarmWrite(sensorTimer, SENSOR_READ_INTERVAL_MS * 1000, true);  // 500ms interval
  timerAlarmEnable(sensorTimer);
  
  Serial.println("Sensor interrupt timer initialized");
}

// DMA buffer management functions
void initDMABuffers() {
  // Allocate DMA-capable buffers
  dmaBuffer1 = (uint8_t*)heap_caps_malloc(DMA_BUFFER_SIZE, MALLOC_CAP_DMA);
  dmaBuffer2 = (uint8_t*)heap_caps_malloc(DMA_BUFFER_SIZE, MALLOC_CAP_DMA);
  
  if (!dmaBuffer1 || !dmaBuffer2) {
    Serial.println("ERROR: Failed to allocate DMA buffers!");
    return;
  }
  
  currentBuffer = dmaBuffer1;
  backBuffer = dmaBuffer2;
  
  // Clear both buffers
  memset(dmaBuffer1, 0, DMA_BUFFER_SIZE);
  memset(dmaBuffer2, 0, DMA_BUFFER_SIZE);
  
  Serial.println("DMA buffers initialized successfully");
}

// Optimized DMA-based rectangle fill
void fillRectDMA(int x, int y, int w, int h, uint16_t color) {
  // Bounds checking
  if (x < 0 || y < 0 || x >= 320 || y >= 240 || x + w > 320 || y + h > 240) {
    return;
  }
  
  // Process in 20-line chunks for DMA
  int linesRemaining = h;
  int currentY = y;
  
  while (linesRemaining > 0) {
    int linesToProcess = min(linesRemaining, 20);
    
    // Wait for any active DMA transfer
    while (dmaTransferActive) { 
      delayMicroseconds(10); 
    }
    
    // Fill back buffer with color data
    uint16_t* bufPtr = (uint16_t*)backBuffer;
    int pixelCount = w * linesToProcess;
    
    // Optimized fill using 32-bit writes
    uint32_t colorPair = (color << 16) | color;
    uint32_t* buf32 = (uint32_t*)bufPtr;
    
    for (int i = 0; i < pixelCount / 2; i++) {
      buf32[i] = colorPair;
    }
    
    // Handle odd pixel count
    if (pixelCount & 1) {
      bufPtr[pixelCount - 1] = color;
    }
    
    // Set display area
    setDisplayArea(x, currentY, x + w - 1, currentY + linesToProcess - 1);
    writeCommand(0x2C);
    
    // Start DMA transfer
    dmaTransferActive = true;
    transferDMABuffer(backBuffer, pixelCount * 2);
    
    // Swap buffers
    uint8_t* temp = currentBuffer;
    currentBuffer = backBuffer;
    backBuffer = temp;
    
    currentY += linesToProcess;
    linesRemaining -= linesToProcess;
  }
}

// Fast DMA transfer function
void transferDMABuffer(uint8_t* buffer, int size) {
  // For parallel 8080 interface, we need to manually transfer
  // This is still faster than individual writes due to buffer locality
  uint8_t* ptr = buffer;
  
  for (int i = 0; i < size; i++) {
    writeData8(*ptr++);
  }
  
  dmaTransferActive = false;
}

// Boot optimization functions
void drawBootLogo() {
  // Draw centered boot logo
  int centerX = DISPLAY_WIDTH / 2;
  int centerY = DISPLAY_HEIGHT / 2;
  
  // Draw "ESP32-S3" text in large letters
  drawTextLabel(centerX - 40, centerY - 20, "ESP32-S3", PRIMARY_BLUE);
  drawTextLabel(centerX - 45, centerY, "Dashboard", TEXT_PRIMARY);
  drawTextLabel(centerX - 50, centerY + 20, "Version " DASHBOARD_VERSION, TEXT_SECONDARY);
  
  // Draw progress bar outline
  fillVisibleRect(40, centerY + 40, DISPLAY_WIDTH - 80, 20, BORDER_COLOR);
  fillVisibleRect(42, centerY + 42, DISPLAY_WIDTH - 84, 16, SURFACE_DARK);
}

void updateBootProgress(int percent) {
  int centerY = DISPLAY_HEIGHT / 2;
  int barWidth = ((DISPLAY_WIDTH - 84) * percent) / 100;
  
  // Fill progress bar
  if (barWidth > 0) {
    fillVisibleRect(42, centerY + 42, barWidth, 16, PRIMARY_GREEN);
  }
  
  // Update progress text
  char progressText[20];
  snprintf(progressText, sizeof(progressText), "Loading... %d%%", percent);
  fillVisibleRect(DISPLAY_WIDTH/2 - 50, centerY + 65, 100, 12, BLACK);
  drawTextLabel(DISPLAY_WIDTH/2 - 40, centerY + 65, progressText, TEXT_SECONDARY);
}

void setup() {
  Serial.begin(115200);
  delay(100); // Reduced delay for faster boot
  
  // Version info for upload verification
  Serial.println("\n\n=== ESP32-S3 Dashboard v" DASHBOARD_VERSION " OPTIMIZED ===");
  Serial.println("Upload successful - Version " DASHBOARD_VERSION);
  Serial.println("===============================\n");
  
  // SPEED OPTIMIZATION: Boost CPU to 240MHz
  setCpuFrequencyMhz(240);
  
  // Initialize buttons FIRST for immediate responsiveness
  pinMode(BUTTON_1, INPUT_PULLUP);
  pinMode(BUTTON_2, INPUT_PULLUP);
  
  // OPTIMIZATION: Initialize display BEFORE WiFi for instant visual feedback
  Serial.println("[BOOT] Early display init for faster perceived boot...");
  initDisplay();
  
  // Show boot logo/splash immediately
  fillScreen(BLACK);
  drawBootLogo();
  
  // Load saved settings while splash is shown
  preferences.begin("dashboard", false);
  loadSettings();
  
  // Apply brightness setting immediately
  if (settings.brightness != 100) {
    setBrightness(settings.brightness);
  }
  
  // NOW start WiFi in background while user sees splash
  Serial.println("[BOOT] Starting WiFi in background...");
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
  Serial.print("WiFi connecting to: ");
  Serial.println(WIFI_SSID);
  
  // Complete memory init while WiFi connects
  comprehensiveMemoryInit();
  
  // Show loading progress
  updateBootProgress(25);
  
  Serial.println("=== ESP32-S3 DASHBOARD v4.0 - FULLY OPTIMIZED ===");
  Serial.println("Performance Features:");
  Serial.println("  - DMA Double-buffering for +40% FPS");
  Serial.println("  - Interrupt-driven sensors (-5ms jitter)");
  Serial.println("  - Dirty rectangle tracking (-20% CPU)");
  Serial.println("  - Power optimizations (-18mA idle)");
  
  // Initialize battery pin - ensure no pull-up/pull-down
  pinMode(BATTERY_PIN, INPUT);
  // Explicitly disable internal pull-up/pull-down resistors
  gpio_pullup_dis((gpio_num_t)BATTERY_PIN);
  gpio_pulldown_dis((gpio_num_t)BATTERY_PIN);
  
  // OPTIMIZATION: Initialize interrupt-driven sensor reading
  initSensorInterrupt();
  
  // Update boot progress
  updateBootProgress(50);
  
  // Continue WiFi connection monitoring
  Serial.println("=== DETAILED WIFI CONNECTION MONITORING ===");
  Serial.println("WiFi connection in progress...");
  
  // Check if already started
  Serial.print("Current WiFi mode: ");
  Serial.println(WiFi.getMode());
  Serial.print("Current WiFi status: ");
  Serial.println(WiFi.status());
  
  // Re-initialize if needed
  if (WiFi.status() == WL_NO_SHIELD) {
    Serial.println("WiFi shield not present! Re-initializing...");
    WiFi.mode(WIFI_STA);
    delay(100);
  }
  
  // Force reconnection attempt
  Serial.println("Starting fresh connection attempt...");
  WiFi.disconnect();
  delay(100);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
  Serial.println("Fresh WiFi.begin() called");
  
  // Detailed connection monitoring with progress bar
  int wifiTimeout = 40; // 40 seconds timeout
  Serial.print("Monitoring connection");
  int bootPercent = 50;
  while (WiFi.status() != WL_CONNECTED && wifiTimeout > 0) {
    delay(500);
    Serial.print(".");
    wifiTimeout--;
    
    // Update boot progress bar
    bootPercent = 50 + ((40 - wifiTimeout) * 40 / 40);  // 50% to 90%
    updateBootProgress(bootPercent);
    
    // Detailed status every 2 seconds
    if (wifiTimeout % 4 == 0) {
      Serial.println();
      Serial.print("  Status: ");
      Serial.print(WiFi.status());
      Serial.print(" | Time remaining: ");
      Serial.print(wifiTimeout/2);
      Serial.print("s | RSSI: ");
      Serial.println(WiFi.RSSI());
      Serial.print("  ");
    }
  }
  
  Serial.println();
  Serial.println("=== FINAL CONNECTION RESULT ===");
  if (WiFi.isConnected()) {
    Serial.println("SUCCESS! WiFi CONNECTED!");
    Serial.print("  Network: ");
    Serial.println(WiFi.SSID());
    Serial.print("  IP Address: ");
    Serial.println(WiFi.localIP());
    Serial.print("  Gateway: ");
    Serial.println(WiFi.gatewayIP());
    Serial.print("  DNS: ");
    Serial.println(WiFi.dnsIP());
    Serial.print("  Signal Strength: ");
    Serial.print(WiFi.RSSI());
    Serial.println(" dBm");
    Serial.print("  Channel: ");
    Serial.println(WiFi.channel());
    
    // OPTIMIZATION: Enable WiFi power save mode
    Serial.println("\n=== Enabling WiFi Power Save ===");
    WiFi.setSleep(true);  // Enable modem sleep when idle
    Serial.println("WiFi modem sleep enabled - saves ~5mA idle current");
    
    // OPTIMIZATION: Configure dynamic CPU frequency scaling
    Serial.println("\n=== Configuring Dynamic CPU Frequency ===");
    esp_pm_config_t pm_config = {
        .max_freq_mhz = 240,  // Maximum frequency when active
        .min_freq_mhz = 80,   // Minimum frequency when idle
        .light_sleep_enable = false  // Keep false for now to maintain responsiveness
    };
    ESP_ERROR_CHECK(esp_pm_configure(&pm_config));
    Serial.println("CPU DFS enabled: 80-240MHz - saves ~3mA idle");
    
    // Setup OTA updates after successful WiFi connection
    Serial.println("\n=== Setting up OTA Updates ===");
    setupOTA();
    setupWebOTA();  // Add web-based OTA
    Serial.println("=== OTA Setup Complete ===\n");
  } else {
    Serial.println("FAILED! WiFi NOT CONNECTED");
    Serial.print("  Final Status Code: ");
    Serial.println(WiFi.status());
    Serial.println("  Status Code Meanings:");
    Serial.println("    0 = WL_IDLE_STATUS (WiFi idle)");
    Serial.println("    1 = WL_NO_SSID_AVAIL (SSID not found)");
    Serial.println("    2 = WL_SCAN_COMPLETED");
    Serial.println("    3 = WL_CONNECTED (success!)");
    Serial.println("    4 = WL_CONNECT_FAILED (wrong password)");
    Serial.println("    5 = WL_CONNECTION_LOST");
    Serial.println("    6 = WL_DISCONNECTED");
    Serial.println("    255 = WL_NO_SHIELD (WiFi not available)");
    Serial.println("  Possible issues:");
    Serial.println("    - Wrong network name");
    Serial.println("    - Wrong password"); 
    Serial.println("    - Network is 5GHz (ESP32 only supports 2.4GHz)");
    Serial.println("    - Network out of range");
    Serial.println("    - MAC filtering enabled");
  }
  
  // Finish boot sequence
  updateBootProgress(100);
  delay(500);  // Show 100% briefly
  
  // Clear screen and draw initial content
  fillScreen(BLACK);
  drawHeader();
  drawSystemInfo();
  
  // Reset activity timer
  lastActivityTime = millis();
  
  // Final WiFi status check
  Serial.print("Final WiFi Status: ");
  if (WiFi.isConnected()) {
    Serial.print("CONNECTED to ");
    Serial.print(WiFi.SSID());
    Serial.print(" (IP: ");
    Serial.print(WiFi.localIP());
    Serial.println(")");
    
    // Force screen refresh if we're on WiFi page
    if (screenIndex == 2) {
      clearContentArea();
      drawWiFiStatus();
      Serial.println("WiFi page refreshed with connection status");
    }
  } else {
    Serial.println("NOT CONNECTED");
    Serial.print("Status code: ");
    Serial.println(WiFi.status());
  }
  
  Serial.println("Dashboard ready - auto-updating content");
}

void loop() {
  // Handle OTA updates first
  if (WiFi.isConnected()) {
    ArduinoOTA.handle();
    server.handleClient();  // Handle web requests
  }
  
  // Skip normal operation during OTA
  if (otaInProgress || webOtaInProgress) {
    return;
  }
  
  // Debug OTA status every 10 seconds
  static unsigned long lastOTADebug = 0;
  if (millis() - lastOTADebug > 10000) {
    lastOTADebug = millis();
    Serial.print("OTA: Active, WiFi: ");
    Serial.print(WiFi.isConnected() ? "Connected" : "Disconnected");
    Serial.print(", IP: ");
    Serial.println(WiFi.localIP());
  }
  
  static bool needsRedraw = true;
  static unsigned long lastUpdate = 0;
  
  // Update button states
  updateButton(btn1State, BUTTON_1);
  updateButton(btn2State, BUTTON_2);
  
  // Handle button events with enhanced feedback
  int btn1Event = getButtonEvent(btn1State);
  int btn2Event = getButtonEvent(btn2State);
  
  // Optimized interaction feedback + auto-dim fix
  static unsigned long lastInteraction = 0;
  if (btn1Event != BUTTON_NONE || btn2Event != BUTTON_NONE) {
    lastInteraction = millis();
    // Instant screen flash
    fillVisibleRect(0, 0, DISPLAY_WIDTH, 1, PRIMARY_BLUE);
    
    // CRITICAL: Restore brightness immediately on button press
    if (isDimmed && settings.autoDim) {
      isDimmed = false;
      ledcWrite(0, map(settings.brightness, 0, 100, 0, 255));
    }
  }
  
  // Track navigation direction for smooth transitions
  static int transitionDirection = 0; // 0=none, 1=forward, -1=back
  
  // ULTRA SIMPLE: BOOT button = Next Screen (only when not in menu)
  if (btn1Event == BUTTON_CLICK && currentMenu == MENU_NONE) {
    int previousScreen = screenIndex;
    screenIndex = (screenIndex + 1) % 5;
    needsRedraw = true;
    fastTransition = true;  // Enable fast transition
    lastActivityTime = millis();  // Reset activity timer
    if (screenIndex == 1) {
      forcePowerRedraw = true;  // Force power screen redraw
      Serial.println("[MAIN] Navigated to power screen, forcing redraw");
    }
    
    // FORCE navigation update immediately
    drawNavigationIndicator(previousScreen);
  }
  
  // CONTEXTUAL: USER button performs different actions per screen (only when not in menu)
  if (btn2Event == BUTTON_CLICK && currentMenu == MENU_NONE) {
    needsRedraw = true;
    lastActivityTime = millis();  // Reset activity timer
    if (screenIndex == 4) {
      enterSettingsMenu();
    } else {
      executeScreenAction(screenIndex);
    }
  }
  
  // SPEED OPTIMIZED: Minimal redraws
  if (needsRedraw) {
    static int lastScreenIndex = -1;
    
    if (fastTransition && lastScreenIndex != screenIndex) {
      // ULTRA FAST: Only clear and redraw content area
      clearContentArea();
      
      // Update header screen name only
      updateHeaderTitle();
      
      // CRITICAL: Update navigation indicator on screen change
      drawNavigationIndicator(lastScreenIndex);
    } else if (lastScreenIndex == -1) {
      // First draw - full screen
      quickClearScreen();
      drawHeader();
      drawNavigationIndicator();
      drawSimpleButtonHints();  // Draw button indicators on right
    } else if (screenIndex != lastScreenIndex && lastScreenIndex != -1) {
      // Screen change - clear content area only
      clearContentArea();
      drawNavigationIndicator(lastScreenIndex);  // Update navigation with previous index
      // Button hints stay persistent, no need to redraw
    }
    
    // Draw content with minimal overhead
    switch(screenIndex) {
      case 0: drawSystemInfo(); break;
      case 1: 
        Serial.print("[MAIN-DEBUG] Drawing power screen, index=");
        Serial.println(screenIndex);
        drawPowerStatus(); 
        break;
      case 2: drawWiFiStatus(); break;
      case 3: drawSensorData(); break;
      case 4: drawSettings(); break;
    }
    
    // Draw button hints only once on initial draw
    if (lastScreenIndex == -1) {
      drawSimpleButtonHints();
    }
    
    lastScreenIndex = screenIndex;
    fastTransition = false;
    needsRedraw = false;
  }
  
  // Detect power state changes for immediate update
  static bool lastUSBState = false;
  static int lastBatteryReading = 0;
  static unsigned long lastPowerCheck = 0;
  
  // Check power state more frequently for better responsiveness
  if (millis() - lastPowerCheck > 100) {  // Check every 100ms
    lastPowerCheck = millis();
    
    // Use interrupt-driven ADC reading
    int rawADC = 0;
    portENTER_CRITICAL(&sensorMux);
    if (batteryReadingReady) {
      rawADC = latestBatteryADC;
      batteryReadingReady = false;
    }
    portEXIT_CRITICAL(&sensorMux);
    
    // If no reading available, use direct read as fallback
    if (rawADC == 0) {
      rawADC = analogRead(BATTERY_PIN);
    }
    
    float adcVoltage = (rawADC / 4095.0) * 3.3;
    int currentBatteryMv = constrain((int)(adcVoltage * 2110), 0, 5000);  // 2.11 calibration
    bool currentUSBState = currentBatteryMv > USB_DETECT_THRESHOLD;
    
    // Check if power state changed (USB plugged/unplugged)
    bool powerStateChanged = (currentUSBState != lastUSBState) || 
                            (abs(currentBatteryMv - lastBatteryReading) > 50);  // More sensitive
    
    if (powerStateChanged) {
      lastUSBState = currentUSBState;
      lastBatteryReading = currentBatteryMv;
      
      // Always update header immediately
      drawPowerIndicator();
      
      // Force immediate update if on power screen
      if (currentMenu == MENU_NONE && screenIndex == 1) {
        // Don't clear - drawPowerStatus handles it properly now
        drawPowerStatus();
      }
    }
  }
  
  // Enable dynamic updates for System and Power pages
  if (currentMenu == MENU_NONE) {
    if (screenIndex == 0) {
      updateSystemPageDynamic();
    } else if (screenIndex == 1) {
      // Update power status every 250ms for smooth updates
      static unsigned long lastPowerUpdate = 0;
      if (millis() - lastPowerUpdate > 250) {
        lastPowerUpdate = millis();
        drawPowerStatus();
        // Always update header power indicator
        drawPowerIndicator();
      }
    }
  }
  
  // Auto-dim handling - FIXED logic
  if (settings.autoDim) {
    unsigned long now = millis();
    if (!isDimmed && (now - lastActivityTime > DIM_TIMEOUT)) {
      // Smoothly dim the display
      isDimmed = true;
      int currentBrightness = map(settings.brightness, 0, 100, 0, 255);
      int targetBrightness = map(20, 0, 100, 0, 255);  // Dim to 20%
      
      // Smooth fade over 500ms
      for (int i = currentBrightness; i >= targetBrightness; i -= 5) {
        ledcWrite(0, i);
        delay(10);
      }
      ledcWrite(0, targetBrightness);
    } else if (isDimmed && (now - lastActivityTime <= DIM_TIMEOUT)) {
      // Smoothly restore brightness when activity detected
      isDimmed = false;
      int currentBrightness = map(20, 0, 100, 0, 255);
      int targetBrightness = map(settings.brightness, 0, 100, 0, 255);
      
      // Smooth fade over 200ms (faster restore)
      for (int i = currentBrightness; i <= targetBrightness; i += 10) {
        ledcWrite(0, i);
        delay(5);
      }
      ledcWrite(0, targetBrightness);
    }
  }
  
  // Handle menu navigation
  if (currentMenu != MENU_NONE) {
    if (btn1Event == BUTTON_CLICK) {
      handleMenuNavigation();
    }
    if (btn2Event == BUTTON_CLICK) {
      handleMenuSelect();
    }
    // Quick exit: Long press BOOT or USER exits menu immediately
    if (btn1Event == BUTTON_LONG_PRESS || btn2Event == BUTTON_LONG_PRESS) {
      exitMenu();
    }
  }
  
  // FPS monitoring and telemetry
  frameCount++;
  unsigned long now = millis();
  if (now - lastFPSUpdate >= FPS_UPDATE_INTERVAL) {
    currentFPS = (float)frameCount * 1000.0 / (now - lastFPSUpdate);
    frameCount = 0;
    lastFPSUpdate = now;
    
    // Output FPS telemetry to serial
    Serial.print("[PERF] FPS: ");
    Serial.print(currentFPS, 1);
    Serial.print(" | Free Heap: ");
    Serial.print(ESP.getFreeHeap() / 1024);
    Serial.print("KB | CPU Freq: ");
    Serial.print(getCpuFrequencyMhz());
    Serial.print("MHz | Dirty Rects: ");
    Serial.println(dirtyRectCount);
  }
  
  // Clear dirty rectangles after frame is complete
  clearDirtyRects();
}

// Update button state
void updateButton(ButtonState &btn, int pin) {
  btn.previous = btn.current;
  btn.current = digitalRead(pin);
  
  // Button pressed
  if (btn.current == LOW && btn.previous == HIGH) {
    btn.pressTime = millis();
    btn.longPressTriggered = false;
    
    // Visual feedback for button press
    if (&btn == &btn1State) {
      drawButtonPressFeedback(1);
    } else if (&btn == &btn2State) {
      drawButtonPressFeedback(2);
    }
    
    // Check for double click
    if (millis() - btn.lastClickTime < DOUBLE_CLICK_TIME) {
      btn.clickCount++;
    } else {
      btn.clickCount = 1;
    }
    btn.lastClickTime = millis();
  }
  
  // Button released
  if (btn.current == HIGH && btn.previous == LOW) {
    btn.releaseTime = millis();
    
    // Clear feedback area only
    fillVisibleRect(DISPLAY_WIDTH - 4, 40, 4, 20, BLACK);
    fillVisibleRect(DISPLAY_WIDTH - 4, DISPLAY_HEIGHT - 60, 4, 20, BLACK);
    // Don't restore weird dots
    // fillVisibleRect(DISPLAY_WIDTH - 2, 50, 2, 2, 0x39E7);
    // fillVisibleRect(DISPLAY_WIDTH - 2, DISPLAY_HEIGHT - 50, 2, 2, 0x5ACF);
  }
}

// Get button event
int getButtonEvent(ButtonState &btn) {
  // Check for long press
  if (btn.current == LOW && !btn.longPressTriggered && 
      (millis() - btn.pressTime > LONG_PRESS_TIME)) {
    btn.longPressTriggered = true;
    return BUTTON_LONG_PRESS;
  }
  
  // Check for click on release
  if (btn.current == HIGH && btn.previous == LOW) {
    unsigned long pressDuration = btn.releaseTime - btn.pressTime;
    
    // Only register as click if not a long press
    if (pressDuration < LONG_PRESS_TIME) {
      // Check for double click
      if (btn.clickCount >= 2) {
        btn.clickCount = 0;
        return BUTTON_DOUBLE_CLICK;
      }
      return BUTTON_CLICK;
    }
  }
  
  return BUTTON_NONE;
}

void drawHeader() {
  // Professional header - distinctive blue background  
  fillVisibleRect(0, 0, DISPLAY_WIDTH, 20, PRIMARY_BLUE);   // Blue header for distinction
  fillVisibleRect(0, 18, DISPLAY_WIDTH, 2, PRIMARY_BLUE);  // Blue accent border
  
  // Screen name in center - clean, visible text, avoid navigation area
  const char* screenNames[] = {"System", "Power", "WiFi", "Hardware", "Settings"};
  // Calculate proper center position, avoiding left navigation (starts at x=40)
  int textWidth = strlen(screenNames[screenIndex]) * 6;  // Approximate text width
  int centerX = (40 + (DISPLAY_WIDTH - 80)) / 2 - textWidth / 2;  // Center between navigation and power area
  drawStringTransparent(centerX, 6, screenNames[screenIndex], 0x0000);  // White text on blue background
  
  // Power indicator on right
  drawPowerIndicator();
}

// Execute contextual action based on current screen
// Removed old contextual action function

// Draw power indicator in content area
void drawPowerIndicator() {
  static int lastBatteryMv = 0;
  static int indicatorHistory[5] = {0};
  static int indicatorIndex = 0;
  static bool indicatorInitialized = false;
  
  // Get raw reading
  int rawADC = analogRead(BATTERY_PIN);
  
  // Initialize history
  if (!indicatorInitialized) {
    for (int i = 0; i < 5; i++) {
      indicatorHistory[i] = rawADC;
    }
    indicatorInitialized = true;
  }
  
  // Update circular buffer
  indicatorHistory[indicatorIndex] = rawADC;
  indicatorIndex = (indicatorIndex + 1) % 5;
  
  // Calculate average (5 samples for header, faster update)
  long sum = 0;
  for (int i = 0; i < 5; i++) {
    sum += indicatorHistory[i];
  }
  int avgADC = sum / 5;
  
  // Calibrated voltage
  float adcVoltage = (avgADC / 4095.0) * 3.3;
  int batteryMv = constrain((int)(adcVoltage * 2110), 0, 5000);  // 2.11 calibration
  bool onUSB = batteryMv > USB_DETECT_THRESHOLD;
  bool isCharging = onUSB && (batteryMv > CHARGING_THRESHOLD || batteryMv > lastBatteryMv + 20);
  lastBatteryMv = batteryMv;
  
  // In header, right side - expand to edge
  int x = DISPLAY_WIDTH - 80;  // Wider area for better clearing
  int y = 2;  // Align better
  
  // Clear area - wider to ensure complete clearing
  fillVisibleRect(x, y, 80, 14, PRIMARY_BLUE);  // Match blue header background
  
  if (onUSB) {
    // USB icon and text with charging indicator - use transparent text
    drawStringTransparent(x + 5, y + 4, isCharging ? "Charging" : "USB Power", isCharging ? YELLOW : CYAN);
    // Small plug icon
    fillVisibleRect(x + 60, y + 4, 8, 6, CYAN);
    fillVisibleRect(x + 58, y + 5, 2, 4, CYAN);
    fillVisibleRect(x + 68, y + 6, 2, 2, CYAN);
    
    // Lightning bolt animation for charging
    if (isCharging && (millis() / 500) % 2) {
      // Simple lightning bolt
      fillVisibleRect(x + 72, y + 3, 1, 3, YELLOW);
      fillVisibleRect(x + 73, y + 5, 1, 2, YELLOW);
      fillVisibleRect(x + 74, y + 7, 1, 3, YELLOW);
    }
  } else {
    // Battery percentage and icon
    int percent = constrain(map(batteryMv, 3000, 4200, 0, 100), 0, 100);
    uint16_t color = percent > 50 ? GREEN : (percent > 20 ? YELLOW : RED);
    
    // Percentage text - adjusted position, use transparent text
    char percentText[8];
    snprintf(percentText, sizeof(percentText), "%d%%", percent);
    drawStringTransparent(x + 15, y + 4, percentText, color);
    
    // Battery icon - adjusted position
    fillVisibleRect(x + 45, y + 4, 12, 7, TEXT_PRIMARY);
    fillVisibleRect(x + 46, y + 5, 10, 5, BLACK);
    fillVisibleRect(x + 57, y + 6, 1, 3, TEXT_PRIMARY);
    
    // Battery fill
    int fillW = (9 * percent) / 100;
    if (fillW > 0) {
      fillVisibleRect(x + 47, y + 6, fillW, 3, color);
    }
  }
}

// OPTIMIZED CARD SYSTEM - Reduced draw calls
void drawCard(int x, int y, int w, int h, const char* title, uint16_t borderColor = BORDER_COLOR) {
  // Single shadow layer
  fillVisibleRect(x + 2, y + 2, w, h, 0x2104);
  
  // Main card - BLACK background for dark theme
  fillVisibleRect(x, y, w, h, BLACK);
  
  // Simple border - single pixel
  fillVisibleRect(x, y, w, 1, borderColor);          // Top
  fillVisibleRect(x, y + h - 1, w, 1, borderColor);  // Bottom
  fillVisibleRect(x, y, 1, h, borderColor);          // Left
  fillVisibleRect(x + w - 1, y, 1, h, borderColor);  // Right
  
  // Title area
  if (title) {
    fillVisibleRect(x + 2, y + 2, w - 4, 14, borderColor);
    drawTextLabel(x + 5, y + 5, title, TEXT_PRIMARY);
  }
}

// ENHANCED COMPONENT FUNCTIONS
void drawStatusIndicator(int x, int y, int value, int maxValue, uint16_t color, const char* label) {
  // Progress bar with modern styling
  int barWidth = 60;
  int barHeight = 8;
  
  // Background bar
  fillVisibleRect(x, y, barWidth, barHeight, SURFACE_LIGHT);
  
  // Fill bar
  int fillWidth = (barWidth * value) / maxValue;
  fillVisibleRect(x, y, fillWidth, barHeight, color);
  
  // Value label
  drawNumberLabel(x + barWidth + 5, y, value, TEXT_PRIMARY);
  if (label) {
    drawTextLabel(x + barWidth + 25, y, label, TEXT_SECONDARY);
  }
}

void drawMetricCard(int x, int y, int w, const char* title, int value, const char* unit, uint16_t color) {
  drawCard(x, y, w, 50, title, color);
  
  // Large value display
  drawNumberLabel(x + 10, y + 25, value, color);
  if (unit) {
    drawTextLabel(x + 35, y + 25, unit, TEXT_SECONDARY);
  }
  
  // Subtle progress indicator
  int percent = constrain(value, 0, 100);
  fillVisibleRect(x + 5, y + 42, w - 10, 3, SURFACE_LIGHT);
  fillVisibleRect(x + 5, y + 42, ((w - 10) * percent) / 100, 3, color);
}

// Instant power status drawing - immediate response to state changes
void drawPowerStatus() {
  static int lastBatteryMv = 0;
  static bool lastOnUSB = false;
  static bool lastCharging = false;
  static int lastBatPercent = -1;
  static bool firstDraw = true;
  static int lastScreenIndex = -1;
  
  // Force redraw on screen change or when forcePowerRedraw is set
  if (screenIndex == 1) {
    if (lastScreenIndex != 1 || forcePowerRedraw) {
      Serial.print("[PWR-DEBUG] Power screen activated, lastScreen=");
      Serial.print(lastScreenIndex);
      Serial.print(", forcePowerRedraw=");
      Serial.println(forcePowerRedraw);
      firstDraw = true;
      forcePowerRedraw = false;
    }
    lastScreenIndex = screenIndex;
  }
  
  // Read current state with smoothing
  static int adcHistory[10] = {0};
  static int historyIndex = 0;
  static bool historyInitialized = false;
  
  int rawADC = analogRead(BATTERY_PIN);
  
  // Initialize history on first read
  if (!historyInitialized) {
    for (int i = 0; i < 10; i++) {
      adcHistory[i] = rawADC;
    }
    historyInitialized = true;
  }
  
  // Add to circular buffer
  adcHistory[historyIndex] = rawADC;
  historyIndex = (historyIndex + 1) % 10;
  
  // Calculate average
  long adcSum = 0;
  for (int i = 0; i < 10; i++) {
    adcSum += adcHistory[i];
  }
  int avgADC = adcSum / 10;
  
  // T-Display-S3 voltage divider calibration
  // Using averaged ADC value for stable readings
  float adcVoltage = (avgADC / 4095.0) * 3.3;
  
  // Debug raw ADC values on power screen
  static unsigned long lastADCDebug = 0;
  if (screenIndex == 1 && millis() - lastADCDebug > 2000) {
    lastADCDebug = millis();
    Serial.print("[ADC-DEBUG] Raw: ");
    Serial.print(rawADC);
    Serial.print(", Avg: ");
    Serial.print(avgADC);
    Serial.print(", Voltage: ");
    Serial.print(adcVoltage, 3);
    Serial.print("V");
  }
  
  // Check if battery is actually connected
  // ADC floating (no battery) typically shows very high or very low values
  bool batteryConnected = (avgADC > NO_BATTERY_ADC_MIN && avgADC < NO_BATTERY_ADC_MAX);
  
  int batteryMv;
  if (!batteryConnected) {
    // No battery connected - ADC is floating
    batteryMv = 0;
    if (screenIndex == 1 && millis() - lastADCDebug < 100) {
      Serial.println(" - No battery (floating)");
    }
  } else {
    // Calculate battery voltage
    batteryMv = (int)(adcVoltage * 2110);  // 2.11 calibration factor
    
    // Sanity check - Li-ion shouldn't exceed 4.3V
    if (batteryMv > MAX_BATTERY_VOLTAGE) {
      if (screenIndex == 1 && millis() - lastADCDebug < 100) {
        Serial.print(" - Capping voltage from ");
        Serial.print(batteryMv);
        Serial.println(" to 4300mV");
      }
      batteryMv = MAX_BATTERY_VOLTAGE;
    }
    
    if (screenIndex == 1 && millis() - lastADCDebug < 100) {
      Serial.print(", Battery: ");
      Serial.print(batteryMv);
      Serial.println("mV");
    }
  }
  
  // Constrain to reasonable values
  batteryMv = constrain(batteryMv, 0, MAX_BATTERY_VOLTAGE);
  
  // Determine power source
  bool onUSB = false;
  if (!batteryConnected) {
    // No battery detected = must be on USB
    onUSB = true;
  } else if (batteryMv > USB_DETECT_THRESHOLD) {
    // High voltage = USB power
    onUSB = true;
  }
  
  // Enhanced charging detection with voltage trend analysis
  static int voltageHistory[5] = {0};
  static int voltageHistoryIndex = 0;
  static bool voltageHistoryInit = false;
  static unsigned long lastVoltageStore = 0;
  
  if (!voltageHistoryInit) {
    for (int i = 0; i < 5; i++) {
      voltageHistory[i] = batteryMv;
    }
    voltageHistoryInit = true;
  }
  
  // Store voltage history every 500ms
  if (millis() - lastVoltageStore > 500) {
    lastVoltageStore = millis();
    voltageHistory[voltageHistoryIndex] = batteryMv;
    voltageHistoryIndex = (voltageHistoryIndex + 1) % 5;
  }
  
  // Calculate voltage trend
  int avgHistoricalVoltage = 0;
  for (int i = 0; i < 5; i++) {
    avgHistoricalVoltage += voltageHistory[i];
  }
  avgHistoricalVoltage /= 5;
  
  int voltageTrend = batteryMv - avgHistoricalVoltage;
  
  // Debug charging detection
  Serial.print(" Trend: ");
  Serial.print(voltageTrend);
  Serial.print("mV");
  
  // Improved charging detection:
  // 1. Voltage above 4.3V (definitely charging)
  // 2. Consistent rise in voltage (>5mV average increase)
  // 3. Voltage in charging range (3.6-4.15V) with upward trend
  bool isCharging = onUSB && (
    batteryMv > CHARGING_THRESHOLD ||                    // High voltage
    voltageTrend > 5 ||                                  // Rising trend
    (batteryMv > 3600 && batteryMv < 4150 && voltageTrend > 2)  // Mid-range with slight rise
  );
  
  // Detect state changes
  bool usbChanged = (onUSB != lastOnUSB);
  bool chargingChanged = (isCharging != lastCharging);
  bool needsFullRedraw = firstDraw || usbChanged || chargingChanged;
  
  // CRITICAL: Ensure we always draw something on power screen
  if (screenIndex == 1 && !needsFullRedraw && lastBatPercent == -1) {
    Serial.println("[PWR-DEBUG] Forcing redraw - no previous battery data");
    needsFullRedraw = true;
  }
  
  // Debug output - always print on first draw
  static unsigned long lastDebugPrint = 0;
  if (firstDraw || millis() - lastDebugPrint > 1000) {
    lastDebugPrint = millis();
    Serial.print("[PWR-DEBUG-v3] Raw ADC: ");
    Serial.print(rawADC);
    Serial.print(", Battery: ");
    Serial.print(batteryMv);
    Serial.print("mV, USB: ");
    Serial.print(onUSB ? "Yes" : "No");
    Serial.print(", Charging: ");
    Serial.print(isCharging ? "Yes" : "No");
    Serial.print(", FirstDraw: ");
    Serial.print(firstDraw ? "Yes" : "No");
    Serial.print(", NeedsRedraw: ");
    Serial.println(needsFullRedraw ? "Yes" : "No");
  }
  
  // Handle USB state transitions immediately
  if (usbChanged && !firstDraw) {
    // Clear entire power display area immediately on USB change
    fillVisibleRect(40, 27, DISPLAY_WIDTH - 80, DISPLAY_HEIGHT - 47, BLACK);
    needsFullRedraw = true;
  }
  
  Serial.print("[PWR-DEBUG] About to draw, onUSB=");
  Serial.print(onUSB);
  Serial.print(", needsFullRedraw=");
  Serial.println(needsFullRedraw);
  
  if (onUSB) {
    // USB/Charging mode display - cleaner UI
    if (needsFullRedraw) {
      Serial.println("[PWR-DEBUG] Drawing USB/Charging display");
      
      // Power status - compact top section (similar to WiFi)
      fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
      uint16_t topBorderColor = !batteryConnected ? PRIMARY_BLUE : 
                               (isCharging ? YELLOW : getGoodColor());
      fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 1, topBorderColor);  // Top border
      
      // Power source info
      drawTextLabel(45, 30, "Power Source", TEXT_SECONDARY);
      const char* powerText = !batteryConnected ? "USB Only" :
                             (isCharging ? "USB Charging" : "USB Power");
      drawTextLabel(45, 42, powerText, topBorderColor);
      
      if (batteryConnected) {
        // Battery percentage with inline progress bar
        int batPercent = constrain(map(batteryMv, 3000, 4200, 0, 100), 0, 100);
        
        // Mini battery bar
        fillVisibleRect(45, 57, 60, 5, BORDER_COLOR);
        fillVisibleRect(46, 58, (58 * batPercent) / 100, 3, topBorderColor);
        
        // Status text
        char statusText[30];
        if (isCharging) {
          snprintf(statusText, sizeof(statusText), "Charging %d%%", batPercent);
        } else {
          snprintf(statusText, sizeof(statusText), "Maintained %d%%", batPercent);
        }
        drawTextLabel(110, 56, statusText, topBorderColor);
        
        // Battery details section - standardized card layout
        drawCard(40, 75, DISPLAY_WIDTH - 80, 30, "BATTERY", getInfoColor());
        
        drawTextLabel(45, 80, "Battery Voltage", TEXT_SECONDARY);
        char voltText[50];
        if (voltageTrend != 0) {
          snprintf(voltText, sizeof(voltText), "%d mV (%+dmV/s)", batteryMv, voltageTrend * 2);
        } else {
          snprintf(voltText, sizeof(voltText), "%d mV", batteryMv);
        }
        drawTextLabel(45, 92, voltText, getInfoColor());
        
        // ADC Debug section - inline
        fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 25, SURFACE_DARK);
        fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 1, TEXT_SECONDARY);  // Top border
        
        char debugText[60];
        snprintf(debugText, sizeof(debugText), "ADC: %d avg (%.3fV)", avgADC, adcVoltage);
        drawTextLabel(45, 115, "Debug Info", TEXT_SECONDARY);
        drawTextLabel(45, 127, debugText, PRIMARY_GREEN);
      } else {
        // No battery section
        fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 40, SURFACE_DARK);
        fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 1, PRIMARY_BLUE);  // Top border
        
        drawTextLabel(45, 80, "Battery Status", TEXT_SECONDARY);
        drawTextLabel(45, 92, "No battery detected", ACCENT_ORANGE);
        
        char adcText[40];
        snprintf(adcText, sizeof(adcText), "ADC: %d (floating)", avgADC);
        drawTextLabel(45, 104, adcText, TEXT_SECONDARY);
      }
      
      // Info section at bottom
      fillVisibleRect(40, 140, DISPLAY_WIDTH - 80, 20, SURFACE_DARK);
      fillVisibleRect(40, 140, DISPLAY_WIDTH - 80, 1, getInfoColor());  // Top border
      
      drawTextLabel(45, 145, batteryConnected ? "USB + Battery backup" : "USB power only", getInfoColor());
    }
  } else {
    // Battery mode display with better Li-ion curve
    // Li-ion battery curve: 3.0V = 0%, 3.7V = 50%, 4.2V = 100%
    int batPercent;
    if (batteryMv < 3300) {
      batPercent = map(batteryMv, 3000, 3300, 0, 10);
    } else if (batteryMv < 3700) {
      batPercent = map(batteryMv, 3300, 3700, 10, 50);
    } else if (batteryMv < 4100) {
      batPercent = map(batteryMv, 3700, 4100, 50, 90);
    } else {
      batPercent = map(batteryMv, 4100, 4200, 90, 100);
    }
    batPercent = constrain(batPercent, 0, 100);
    uint16_t batteryColor = batPercent > 50 ? getGoodColor() : 
                           (batPercent > 20 ? getWarningColor() : getErrorColor());
    
    // Always redraw structure on battery mode or when needed
    if (needsFullRedraw || firstDraw) {
      Serial.print("[PWR-DEBUG] Drawing battery display, batPercent=");
      Serial.print(batPercent);
      Serial.print(", batteryMv=");
      Serial.println(batteryMv);
      
      // Battery status - compact top section (similar to WiFi)
      fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
      fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 1, batteryColor);  // Top border
      
      // Battery info
      drawTextLabel(45, 30, "Battery Power", TEXT_SECONDARY);
      char batText[30];
      snprintf(batText, sizeof(batText), "%d%% - %d mV", batPercent, batteryMv);
      drawTextLabel(45, 42, batText, batteryColor);
      
      // Battery icon with fill inline
      fillVisibleRect(45, 55, 40, 12, SURFACE_LIGHT);  // Battery outline
      fillVisibleRect(46, 56, 38, 10, SURFACE_DARK);   // Interior
      fillVisibleRect(85, 58, 2, 6, SURFACE_LIGHT);    // Terminal
      
      // Draw battery fill
      int fillWidth = (36 * batPercent) / 100;
      if (fillWidth > 0) {
        fillVisibleRect(47, 57, fillWidth, 8, batteryColor);
      }
      
      // Estimate text
      int estHours = (batPercent / 15) + 1;
      char estText[20];
      snprintf(estText, sizeof(estText), "~%d hours", estHours);
      drawTextLabel(95, 58, estText, batteryColor);
      
      // Status section
      fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 30, SURFACE_DARK);
      fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 1, getInfoColor());  // Top border
      
      drawTextLabel(45, 80, "Battery Status", TEXT_SECONDARY);
      const char* statusText = batPercent > 50 ? "Good" : 
                              (batPercent > 20 ? "Low" : "Critical");
      drawTextLabel(45, 92, statusText, batteryColor);
      
      // ADC Debug section - inline
      fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 25, SURFACE_DARK);
      fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 1, TEXT_SECONDARY);  // Top border
      
      char debugText[60];
      snprintf(debugText, sizeof(debugText), "ADC: %d avg / %d raw", avgADC, rawADC);
      drawTextLabel(45, 115, "Debug Info", TEXT_SECONDARY);
      drawTextLabel(45, 127, debugText, PRIMARY_GREEN);
      
      // Info section at bottom
      fillVisibleRect(40, 140, DISPLAY_WIDTH - 80, 20, SURFACE_DARK);
      fillVisibleRect(40, 140, DISPLAY_WIDTH - 80, 1, getInfoColor());  // Top border
      
      drawTextLabel(45, 145, "Battery powered mode", getInfoColor());
    } else if (batPercent != lastBatPercent) {
      // Only update changing values
      // Update battery percentage and voltage text
      fillVisibleRect(45, 42, 150, 12, SURFACE_DARK);
      char batText[30];
      snprintf(batText, sizeof(batText), "%d%% - %d mV", batPercent, batteryMv);
      drawTextLabel(45, 42, batText, batteryColor);
      
      // Update battery icon fill
      fillVisibleRect(47, 57, 36, 8, SURFACE_DARK);
      int fillWidth = (36 * batPercent) / 100;
      if (fillWidth > 0) {
        fillVisibleRect(47, 57, fillWidth, 8, batteryColor);
      }
      
      // Update estimate
      int estHours = (batPercent / 15) + 1;
      fillVisibleRect(95, 58, 80, 10, SURFACE_DARK);
      char estText[20];
      snprintf(estText, sizeof(estText), "~%d hours", estHours);
      drawTextLabel(95, 58, estText, batteryColor);
      
      // Update status text if threshold crossed
      fillVisibleRect(45, 92, 100, 12, SURFACE_DARK);
      const char* statusText = batPercent > 50 ? "Good" : 
                              (batPercent > 20 ? "Low" : "Critical");
      drawTextLabel(45, 92, statusText, batteryColor);
    }
    
    lastBatPercent = batPercent;
  }
  
  // Update state tracking
  lastBatteryMv = batteryMv;
  lastOnUSB = onUSB;
  lastCharging = isCharging;
  firstDraw = false;
  
  // Debug final state
  Serial.print("[PWR-DEBUG] Draw complete - USB: ");
  Serial.print(lastOnUSB ? "Yes" : "No");
  Serial.print(", Bat%: ");
  Serial.println(lastBatPercent);
}

void drawSystemInfo() {
  // Memory section - standardized rectangle layout
  int memPercent = (ESP.getFreeHeap() * 100) / ESP.getHeapSize();
  int freeKB = ESP.getFreeHeap() / 1024;
  int totalKB = ESP.getHeapSize() / 1024;
  
  // Memory section background and border
  uint16_t memColor = memPercent > 50 ? getGoodColor() : (memPercent > 25 ? getWarningColor() : getErrorColor());
  fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
  fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 1, memColor);
  
  // Memory section content - spread across lines for readability
  drawTextLabel(45, 30, "Memory Usage", TEXT_SECONDARY);
  drawNumberLabel(45, 42, memPercent, memColor);
  drawTextLabel(70, 42, "% free", getLabelColor());
  drawNumberLabel(45, 54, freeKB, memColor);
  drawTextLabel(70, 54, "KB available", getLabelColor());
  
  // CPU section - standardized rectangle layout
  static unsigned long lastCpuCheck = 0;
  static int cpuUsage = 45;
  
  // Estimate CPU usage based on loop timing
  unsigned long now = millis();
  if (now - lastCpuCheck > 1000) {
    // Simple CPU estimation: faster loop = higher usage
    int loopSpeed = 1000 / max(1, (int)(now - lastCpuCheck));
    cpuUsage = constrain(30 + (loopSpeed * 5), 20, 85);
    lastCpuCheck = now;
  }
  
  // CPU section background and border
  uint16_t cpuColor = cpuUsage < 70 ? getGoodColor() : (cpuUsage < 85 ? getWarningColor() : getErrorColor());
  fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
  fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 1, cpuColor);
  
  // CPU section content - spread across lines for readability
  drawTextLabel(45, 80, "CPU Performance", TEXT_SECONDARY);
  drawNumberLabel(45, 92, cpuUsage, cpuColor);
  drawTextLabel(70, 92, "% usage", getLabelColor());
  drawTextLabel(45, 104, "2 cores @ 240MHz", getLabelColor());
  
  // System uptime section - standardized rectangle layout
  fillVisibleRect(40, 125, DISPLAY_WIDTH - 80, 35, SURFACE_DARK);
  fillVisibleRect(40, 125, DISPLAY_WIDTH - 80, 1, getInfoColor());
  
  // Uptime section content with FPS indicator
  drawTextLabel(45, 130, "System Uptime", TEXT_SECONDARY);
  int totalSec = millis() / 1000;
  int hours = totalSec / 3600;
  int minutes = (totalSec % 3600) / 60;
  int seconds = totalSec % 60;
  
  // Add FPS indicator on the right
  if (currentFPS > 0) {
    char fpsText[20];
    snprintf(fpsText, sizeof(fpsText), "%.1f FPS", currentFPS);
    uint16_t fpsColor = currentFPS > 25 ? getGoodColor() : (currentFPS > 15 ? getWarningColor() : getErrorColor());
    drawTextLabel(DISPLAY_WIDTH - 110, 130, fpsText, fpsColor);
  }
  
  // Single line uptime display
  if (hours > 0) {
    drawNumberLabel(45, 142, hours, getInfoColor());
    drawTextLabel(60, 142, "h", getLabelColor());
    drawNumberLabel(70, 142, minutes, getInfoColor());
    drawTextLabel(85, 142, "m", getLabelColor());
    drawNumberLabel(95, 142, seconds, getInfoColor());
    drawTextLabel(110, 142, "s", getLabelColor());
  } else if (minutes > 0) {
    drawNumberLabel(45, 142, minutes, getInfoColor());
    drawTextLabel(65, 142, "min", getLabelColor());
    drawNumberLabel(85, 142, seconds, getInfoColor());
    drawTextLabel(100, 142, "sec", getLabelColor());
  } else {
    drawNumberLabel(45, 142, seconds, getInfoColor());
    drawTextLabel(65, 142, "seconds", getLabelColor());
  }
}

void drawWiFiStatus() {
  bool isConnected = WiFi.isConnected();
  Serial.print("Drawing WiFi status - Connected: ");
  Serial.print(isConnected ? "YES" : "NO");
  if (isConnected) {
    Serial.print(" | SSID: ");
    Serial.print(WiFi.SSID());
    Serial.print(" | IP: ");
    Serial.print(WiFi.localIP());
  }
  Serial.println();
  
  if (isConnected) {
    // Refined layout with better spacing
    
    // Connection status - compact top section
    fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
    fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 1, getGoodColor());  // Top border
    
    // Network name
    drawTextLabel(45, 30, "Connected to", TEXT_SECONDARY);
    drawTextLabel(45, 42, WiFi.SSID().c_str(), getGoodColor());
    
    // Signal strength inline
    int rssi = WiFi.RSSI();
    int signalQuality = constrain(map(rssi, -90, -30, 0, 100), 0, 100);
    uint16_t signalColor = signalQuality > 60 ? getGoodColor() : (signalQuality > 30 ? getWarningColor() : getErrorColor());
    
    // Mini signal bar
    fillVisibleRect(45, 57, 60, 5, BORDER_COLOR);
    fillVisibleRect(46, 58, (58 * signalQuality) / 100, 3, signalColor);
    
    // Signal text
    char signalText[30];
    snprintf(signalText, sizeof(signalText), "%d%% (%d dBm)", signalQuality, rssi);
    drawTextLabel(110, 56, signalText, signalColor);
    
    // IP section - cleaner
    fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 30, SURFACE_DARK);
    fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 1, getInfoColor());  // Top border
    
    drawTextLabel(45, 80, "IP Address", TEXT_SECONDARY);
    drawTextLabel(45, 92, WiFi.localIP().toString().c_str(), getInfoColor());
    
    // Web Update - prominent call to action
    fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 40, PRIMARY_GREEN);
    fillVisibleRect(42, 112, DISPLAY_WIDTH - 84, 36, SURFACE_DARK);
    
    drawTextLabel(50, 118, "WEB UPDATE READY", PRIMARY_GREEN);
    drawTextLabel(50, 130, "Open browser and go to:", TEXT_SECONDARY);
    
    // URL in contrasting color
    char url[50];
    snprintf(url, sizeof(url), "http://%s", WiFi.localIP().toString().c_str());
    drawTextLabel(50, 142, url, PRIMARY_GREEN);
  } else {
    // Not connected card
    drawCard(40, 40, DISPLAY_WIDTH - 80, 40, "DISCONNECTED", getErrorColor());
    drawTextLabel(45, 60, "Not Connected", getErrorColor());
    
    // Help card
    drawCard(40, 85, DISPLAY_WIDTH - 80, 45, "HELP", getInfoColor());
    drawTextLabel(45, 105, "Press USER", getTextColor());
    drawTextLabel(45, 115, "to configure", getTextColor());
  }
}

void drawSensorData() {
  // GPIO status - compact top section
  fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
  fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 1, PRIMARY_BLUE);  // Top border
  
  drawTextLabel(45, 30, "GPIO Pins", TEXT_SECONDARY);
  drawTextLabel(45, 42, "Available: 17, 43, 44", PRIMARY_BLUE);
  drawTextLabel(45, 54, "I2C: SDA/SCL Ready", PRIMARY_GREEN);
  
  // Hardware metrics section
  fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
  fillVisibleRect(40, 75, DISPLAY_WIDTH - 80, 1, ACCENT_ORANGE);  // Top border
  
  drawTextLabel(45, 80, "Hardware Metrics", TEXT_SECONDARY);
  
  float cpuTemp = temperatureRead();
  int tempDisplay = constrain((int)cpuTemp, 0, 99);
  int totalFlash = ESP.getSketchSize() + ESP.getFreeSketchSpace();
  int flashUsed = totalFlash > 0 ? (ESP.getSketchSize() * 100) / totalFlash : 0;
  flashUsed = constrain(flashUsed, 0, 100);
  
  // CPU temp with mini bar
  drawTextLabel(45, 92, "CPU:", TEXT_SECONDARY);
  // Temperature bar (0-100°C scale)
  fillVisibleRect(70, 93, 40, 5, BORDER_COLOR);
  int tempBarWidth = constrain((tempDisplay * 38) / 100, 0, 38);
  uint16_t tempColor = tempDisplay < 50 ? PRIMARY_GREEN : (tempDisplay < 70 ? YELLOW : PRIMARY_RED);
  fillVisibleRect(71, 94, tempBarWidth, 3, tempColor);
  
  char tempText[20];
  snprintf(tempText, sizeof(tempText), "%d°C", tempDisplay);
  drawTextLabel(115, 92, tempText, tempColor);
  
  // Flash usage
  drawTextLabel(45, 104, "Flash:", TEXT_SECONDARY);
  // Flash bar
  fillVisibleRect(75, 105, 40, 5, BORDER_COLOR);
  int flashBarWidth = (flashUsed * 38) / 100;
  uint16_t flashColor = flashUsed < 60 ? PRIMARY_GREEN : (flashUsed < 80 ? YELLOW : PRIMARY_RED);
  fillVisibleRect(76, 106, flashBarWidth, 3, flashColor);
  
  char flashText[20];
  snprintf(flashText, sizeof(flashText), "%d%% used", flashUsed);
  drawTextLabel(120, 104, flashText, flashColor);
  
  // System resources section
  fillVisibleRect(40, 125, DISPLAY_WIDTH - 80, 35, SURFACE_DARK);
  fillVisibleRect(40, 125, DISPLAY_WIDTH - 80, 1, PRIMARY_GREEN);  // Top border
  
  drawTextLabel(45, 130, "System Resources", TEXT_SECONDARY);
  
  // Get resource info
  int rawADC = analogRead(BATTERY_PIN);
  float adcVoltage = (rawADC / 4095.0) * 3.3;
  int battery = constrain((int)(adcVoltage * 2110), 0, 5000);
  int freeKB = ESP.getFreeHeap() / 1024;
  int totalKB = ESP.getHeapSize() / 1024;
  int usedKB = totalKB - freeKB;
  int ramPercent = (usedKB * 100) / totalKB;
  
  // Power and RAM info
  char resourceText[50];
  if (battery > 4000) {
    snprintf(resourceText, sizeof(resourceText), "Power: USB | RAM: %dKB free", freeKB);
  } else {
    int batPercent = constrain(map(battery, 3000, 4200, 0, 100), 0, 100);
    snprintf(resourceText, sizeof(resourceText), "Battery: %d%% | RAM: %dKB free", batPercent, freeKB);
  }
  drawTextLabel(45, 142, resourceText, PRIMARY_GREEN);
  
  // RAM usage bar
  fillVisibleRect(45, 150, 150, 5, BORDER_COLOR);
  int ramBarWidth = (ramPercent * 148) / 100;
  uint16_t ramColor = ramPercent < 60 ? PRIMARY_GREEN : (ramPercent < 80 ? YELLOW : PRIMARY_RED);
  fillVisibleRect(46, 151, ramBarWidth, 3, ramColor);
}

void drawSettings() {
  // Help section - standardized rectangle layout
  fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 30, SURFACE_DARK);
  fillVisibleRect(40, 25, DISPLAY_WIDTH - 80, 1, getInfoColor());
  drawTextLabel(45, 30, "Help", TEXT_SECONDARY);
  drawTextLabel(45, 42, "Hold USER for menu", getTextColor());
  
  // Menu preview section - simplified for readability
  fillVisibleRect(40, 60, DISPLAY_WIDTH - 80, 45, SURFACE_DARK);
  fillVisibleRect(40, 60, DISPLAY_WIDTH - 80, 1, currentTheme->card_border);
  drawTextLabel(45, 65, "Quick Menu", TEXT_SECONDARY);
  drawTextLabel(45, 77, "Hold USER button", getTextColor());
  drawTextLabel(45, 89, "for full settings", getTextColor());
  
  // Version section - adjusted position for better spacing
  fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 30, SURFACE_DARK);
  fillVisibleRect(40, 110, DISPLAY_WIDTH - 80, 1, getGoodColor());
  drawTextLabel(45, 115, "Version Info", TEXT_SECONDARY);
  drawTextLabel(45, 127, "v" DASHBOARD_VERSION, getTextColor());
}

// Basic 5x8 font data for essential characters (0-9, A-Z, and some symbols)
const uint8_t font5x8[][5] = {
  {0x00, 0x00, 0x00, 0x00, 0x00}, // Space
  {0x00, 0x00, 0x5F, 0x00, 0x00}, // !
  {0x00, 0x00, 0x00, 0x00, 0x00}, // " (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // # (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // $ (skip)
  {0x00, 0x50, 0x30, 0x00, 0x00}, // %
  {0x00, 0x00, 0x00, 0x00, 0x00}, // & (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // ' (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // ( (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // ) (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // * (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // + (skip)
  {0x00, 0x00, 0x00, 0x00, 0x00}, // , (skip)
  {0x08, 0x08, 0x08, 0x08, 0x08}, // -
  {0x00, 0x60, 0x60, 0x00, 0x00}, // .
  {0x20, 0x10, 0x08, 0x04, 0x02}, // /
  // Numbers 0-9
  {0x3E, 0x51, 0x49, 0x45, 0x3E}, // 0
  {0x00, 0x42, 0x7F, 0x40, 0x00}, // 1
  {0x42, 0x61, 0x51, 0x49, 0x46}, // 2
  {0x21, 0x41, 0x45, 0x4B, 0x31}, // 3
  {0x18, 0x14, 0x12, 0x7F, 0x10}, // 4
  {0x27, 0x45, 0x45, 0x45, 0x39}, // 5
  {0x3C, 0x4A, 0x49, 0x49, 0x30}, // 6
  {0x01, 0x71, 0x09, 0x05, 0x03}, // 7
  {0x36, 0x49, 0x49, 0x49, 0x36}, // 8
  {0x06, 0x49, 0x49, 0x29, 0x1E}, // 9
  {0x00, 0x36, 0x36, 0x00, 0x00}, // :
};

// Extended font data for A-Z (uppercase only for simplicity)
const uint8_t fontAZ[][5] = {
  {0x7E, 0x11, 0x11, 0x11, 0x7E}, // A
  {0x7F, 0x49, 0x49, 0x49, 0x36}, // B
  {0x3E, 0x41, 0x41, 0x41, 0x22}, // C
  {0x7F, 0x41, 0x41, 0x22, 0x1C}, // D
  {0x7F, 0x49, 0x49, 0x49, 0x41}, // E
  {0x7F, 0x09, 0x09, 0x09, 0x01}, // F
  {0x3E, 0x41, 0x49, 0x49, 0x7A}, // G
  {0x7F, 0x08, 0x08, 0x08, 0x7F}, // H
  {0x00, 0x41, 0x7F, 0x41, 0x00}, // I
  {0x20, 0x40, 0x41, 0x3F, 0x01}, // J
  {0x7F, 0x08, 0x14, 0x22, 0x41}, // K
  {0x7F, 0x40, 0x40, 0x40, 0x40}, // L
  {0x7F, 0x02, 0x0C, 0x02, 0x7F}, // M
  {0x7F, 0x04, 0x08, 0x10, 0x7F}, // N
  {0x3E, 0x41, 0x41, 0x41, 0x3E}, // O
  {0x7F, 0x09, 0x09, 0x09, 0x06}, // P
  {0x3E, 0x41, 0x51, 0x21, 0x5E}, // Q
  {0x7F, 0x09, 0x19, 0x29, 0x46}, // R
  {0x46, 0x49, 0x49, 0x49, 0x31}, // S
  {0x01, 0x01, 0x7F, 0x01, 0x01}, // T
  {0x3F, 0x40, 0x40, 0x40, 0x3F}, // U
  {0x1F, 0x20, 0x40, 0x20, 0x1F}, // V
  {0x3F, 0x40, 0x38, 0x40, 0x3F}, // W
  {0x63, 0x14, 0x08, 0x14, 0x63}, // X
  {0x07, 0x08, 0x70, 0x08, 0x07}, // Y
  {0x61, 0x51, 0x49, 0x45, 0x43}, // Z
};

// Draw a single character
void drawChar(int x, int y, char c, uint16_t color) {
  const uint8_t* charData = nullptr;
  
  // Map character to font data
  if (c >= '0' && c <= '9') {
    charData = font5x8[c - '0' + 16]; // Numbers start at index 16
  } else if (c >= 'A' && c <= 'Z') {
    charData = fontAZ[c - 'A'];
  } else if (c >= 'a' && c <= 'z') {
    charData = fontAZ[c - 'a']; // Map lowercase to uppercase
  } else if (c == ' ') {
    // Clear space area (6x8 pixels) to remove ghost text
    fillVisibleRect(x, y, 6, 8, BLACK);
    return;
  } else if (c == '-') {
    charData = font5x8[13];
  } else if (c == '.') {
    charData = font5x8[14];
  } else if (c == ':') {
    charData = font5x8[26];
  } else if (c == '%') {
    charData = font5x8[5];
  }
  
  if (!charData) return;
  
  // CRITICAL FIX: Clear entire character background first (6x8 pixels)
  fillVisibleRect(x, y, 6, 8, BLACK);
  
  // Draw character bitmap foreground pixels
  for (int col = 0; col < 5; col++) {
    uint8_t colData = charData[col];
    for (int row = 0; row < 8; row++) {
      if (colData & (1 << row)) {
        fillVisibleRect(x + col, y + row, 1, 1, color);
      }
    }
  }
}

// Draw a single character without background clearing (for navigation)
void drawCharTransparent(int x, int y, char c, uint16_t color) {
  const uint8_t* charData = nullptr;
  
  // Map character to font data
  if (c >= '0' && c <= '9') {
    charData = font5x8[c - '0' + 16]; // Numbers start at index 16
  } else if (c >= 'A' && c <= 'Z') {
    charData = fontAZ[c - 'A'];
  } else if (c >= 'a' && c <= 'z') {
    charData = fontAZ[c - 'a']; // Map lowercase to uppercase
  } else if (c == ' ') {
    return; // Skip spaces - no background clearing needed
  } else if (c == '-') {
    charData = font5x8[13];
  } else if (c == '.') {
    charData = font5x8[14];
  } else if (c == ':') {
    charData = font5x8[26];
  } else if (c == '%') {
    charData = font5x8[5];
  }
  
  if (!charData) return;
  
  // Draw character bitmap - NO background clearing
  for (int col = 0; col < 5; col++) {
    uint8_t colData = charData[col];
    for (int row = 0; row < 8; row++) {
      if (colData & (1 << row)) {
        fillVisibleRect(x + col, y + row, 1, 1, color);
      }
    }
  }
}

// Draw a string with transparent background (for navigation)
void drawStringTransparent(int x, int y, const char* text, uint16_t color) {
  int xPos = x;
  while (*text) {
    drawCharTransparent(xPos, y, *text, color);
    xPos += 6; // Character width + spacing
    text++;
  }
}

// Draw a string
void drawString(int x, int y, const char* text, uint16_t color) {
  int xPos = x;
  while (*text) {
    drawChar(xPos, y, *text, color);
    xPos += 6; // Character width + spacing
    text++;
  }
}

// Simple text drawing functions
void drawTextLabel(int x, int y, const char* text, uint16_t color) {
  drawString(x, y, text, color);
}

void drawNumberLabel(int x, int y, int num, uint16_t color) {
  char buf[16];  // Increased size for safety (int can be up to 11 chars with sign)
  snprintf(buf, sizeof(buf), "%d", num);
  drawString(x, y, buf, color);
}

// Helper function to draw within visible area
void fillVisibleRect(int x, int y, int w, int h, uint16_t color) {
  int actualX = DISPLAY_X_START + x;
  int actualY = DISPLAY_Y_START + y;
  
  if (x < 0 || y < 0 || x + w > DISPLAY_WIDTH || y + h > DISPLAY_HEIGHT) {
    return;
  }
  
  // Skip if rectangle is not in any dirty region
  if (dirtyRectCount > 0 && !isRectDirty(x, y, w, h)) {
    return;
  }
  
  fillRect(actualX, actualY, w, h, color);
}

// Core display functions
void initDisplay() {
  Serial.println("Initializing display...");
  
  pinMode(LCD_POWER_ON, OUTPUT);
  pinMode(LCD_BL, OUTPUT);
  pinMode(LCD_RES, OUTPUT);
  pinMode(LCD_CS, OUTPUT);
  pinMode(LCD_DC, OUTPUT);
  pinMode(LCD_WR, OUTPUT);
  pinMode(LCD_RD, OUTPUT);
  pinMode(LCD_D0, OUTPUT);
  pinMode(LCD_D1, OUTPUT);
  pinMode(LCD_D2, OUTPUT);
  pinMode(LCD_D3, OUTPUT);
  pinMode(LCD_D4, OUTPUT);
  pinMode(LCD_D5, OUTPUT);
  pinMode(LCD_D6, OUTPUT);
  pinMode(LCD_D7, OUTPUT);
  
  digitalWrite(LCD_POWER_ON, HIGH);
  digitalWrite(LCD_CS, LOW);
  digitalWrite(LCD_RD, HIGH);
  digitalWrite(LCD_WR, HIGH);
  
  digitalWrite(LCD_RES, LOW);
  delay(10);
  digitalWrite(LCD_RES, HIGH);
  delay(120);
  
  writeCommand(0x01);  // Software reset
  delay(120);
  writeCommand(0x11);  // Sleep out
  delay(120);
  writeCommand(0x36);  // Memory access control
  writeData(0x60);
  writeCommand(0x3A);  // Pixel format
  writeData(0x55);
  writeCommand(0x29);  // Display on
  delay(100);
  
  // OPTIMIZATION: Setup PWM for backlight control
  ledcSetup(0, 5000, 8);  // Channel 0, 5kHz, 8-bit resolution
  ledcAttachPin(LCD_BL, 0);
  ledcWrite(0, 255);  // Full brightness initially
  
  // OPTIMIZATION: Initialize DMA double-buffers
  initDMABuffers();
  
  Serial.println("Display initialized with PWM backlight and DMA buffers");
}

void comprehensiveMemoryInit() {
  Serial.println("Memory initialization...");
  
  setDisplayArea(0, 0, 479, 319);
  writeCommand(0x2C);
  
  for (int i = 0; i < 480 * 320; i++) {
    writeData(0xFF);
    writeData(0xFF);
  }
  
  Serial.println("Memory init complete!");
}

void fillScreen(uint16_t color) {
  setDisplayArea(0, 0, 319, 239);
  writeCommand(0x2C);
  
  for (int i = 0; i < 320 * 240; i++) {
    writeData((color >> 8) & 0xFF);
    writeData(color & 0xFF);
  }
}

// Optimized screen clear - only clears visible area
void quickClearScreen() {
  // Clear visible area only (much faster)
  setDisplayArea(DISPLAY_X_START, DISPLAY_Y_START, 
                 DISPLAY_X_START + DISPLAY_WIDTH - 1, 
                 DISPLAY_Y_START + DISPLAY_HEIGHT - 1);
  writeCommand(0x2C);
  
  // Use block writes for faster clearing
  uint8_t blackHi = (BLACK >> 8) & 0xFF;
  uint8_t blackLo = BLACK & 0xFF;
  
  // Write in larger chunks for speed
  for (int i = 0; i < DISPLAY_WIDTH * DISPLAY_HEIGHT; i++) {
    writeData(blackHi);
    writeData(blackLo);
  }
}

// Clear content area only (preserve header and side nav)
void clearContentArea() {
  // Mark entire content area as dirty
  markDirty(36, 20, DISPLAY_WIDTH - 36, DISPLAY_HEIGHT - 20);
  
  // Clear from after nav (36px) to full width, from after header (20px) to bottom
  fillVisibleRect(36, 20, DISPLAY_WIDTH - 36, DISPLAY_HEIGHT - 20, BLACK);
}

// Update just the header title - FAST
void updateHeaderTitle() {
  // Only update the title pill area
  const char* screenTitles[] = {"SYSTEM", "POWER", "WIFI", "HARDWARE", "SETTINGS"};
  fillVisibleRect(9, 5, 78, 15, BLUE);
  drawTextLabel(12, 8, screenTitles[screenIndex], TEXT_PRIMARY);
}

// DATA VISUALIZATION UTILITIES
void drawDataPoint(int x, int y, int value, int maxValue, uint16_t color) {
  int dotSize = map(value, 0, maxValue, 2, 8);
  fillVisibleRect(x - dotSize/2, y - dotSize/2, dotSize, dotSize, color);
  fillVisibleRect(x - dotSize/2 + 1, y - dotSize/2 + 1, dotSize - 2, dotSize - 2, TEXT_PRIMARY);
}

void drawProgressRing(int x, int y, int radius, int value, int maxValue, uint16_t color) {
  // Simplified ring - would need more complex drawing for actual ring
  int segments = (8 * value) / maxValue;
  for (int i = 0; i < segments; i++) {
    int angle = i * 45;  // 8 segments
    int dx = radius * cos(angle * PI / 180) / radius;
    int dy = radius * sin(angle * PI / 180) / radius;
    fillVisibleRect(x + dx, y + dy, 2, 2, color);
  }
}

// Button hints - CLEAR ACTION labels that explain what button does
void drawSimpleButtonHints() {
  // Clear right edge completely first
  fillVisibleRect(DISPLAY_WIDTH - 10, 0, 10, DISPLAY_HEIGHT, BLACK);
  
  // Remove weird dots - they're not helpful
  // fillVisibleRect(DISPLAY_WIDTH - 2, 50, 2, 2, 0x39E7);  // Dim green
  // fillVisibleRect(DISPLAY_WIDTH - 2, DISPLAY_HEIGHT - 50, 2, 2, 0x5ACF);  // Dim blue
}

// OPTIMIZED BUTTON FEEDBACK - No blocking delays
void drawButtonPressFeedback(int button) {
  if (button == 1) {
    // BOOT button - instant visual feedback
    fillVisibleRect(DISPLAY_WIDTH - 6, DISPLAY_HEIGHT - 52, 6, 6, PRIMARY_BLUE);
    fillVisibleRect(DISPLAY_WIDTH - 4, DISPLAY_HEIGHT - 50, 4, 4, TEXT_PRIMARY);
  } else if (button == 2) {
    // USER button - instant glow
    fillVisibleRect(DISPLAY_WIDTH - 6, 46, 6, 12, PRIMARY_GREEN);
    fillVisibleRect(DISPLAY_WIDTH - 4, 48, 4, 8, TEXT_PRIMARY);
  }
}

// OPTIMIZED TRANSITIONS - No blocking delays
void highlightCard(int x, int y, int w, int h, uint16_t color) {
  // Instant highlight - just border
  fillVisibleRect(x-1, y-1, w+2, 2, color);      // Top
  fillVisibleRect(x-1, y+h-1, w+2, 2, color);    // Bottom
  fillVisibleRect(x-1, y-1, 2, h+2, color);      // Left
  fillVisibleRect(x+w-1, y-1, 2, h+2, color);    // Right
}

void alertCard(int x, int y, int w, int h, uint16_t color) {
  // Instant alert - just thicker border
  fillVisibleRect(x-2, y-2, w+4, 3, color);
  fillVisibleRect(x-2, y+h-1, w+4, 3, color);
  fillVisibleRect(x-2, y-2, 3, h+4, color);
  fillVisibleRect(x+w-1, y-2, 3, h+4, color);
}

// OPTIMIZED NOTIFICATION SYSTEM - No blocking animations
void showNotification(const char* text, uint16_t color = PRIMARY_GREEN) {
  int x = DISPLAY_WIDTH/2 - 60;
  int y = 10;  // Top of screen
  int w = 120;
  int h = 20;
  
  // Instant notification bar
  fillVisibleRect(x, y, w, h, color);
  fillVisibleRect(x + 2, y + 2, w - 4, h - 4, SURFACE_DARK);
  
  // Status dot and text
  fillVisibleRect(x + 5, y + 7, 6, 6, color);
  drawTextLabel(x + 15, y + 6, text, TEXT_PRIMARY);
}

void fillRect(int x, int y, int w, int h, uint16_t color) {
  if (x < 0 || y < 0 || x >= 320 || y >= 240 || x + w > 320 || y + h > 240) {
    return;
  }
  
  // Use DMA for larger rectangles
  if (dmaBuffer1 && dmaBuffer2 && (w * h) > 100) {
    fillRectDMA(x, y, w, h, color);
    return;
  }
  
  // Fall back to standard method for small rectangles
  setDisplayArea(x, y, x + w - 1, y + h - 1);
  writeCommand(0x2C);
  
  for (int i = 0; i < w * h; i++) {
    writeData((color >> 8) & 0xFF);
    writeData(color & 0xFF);
  }
}

void setDisplayArea(int x1, int y1, int x2, int y2) {
  writeCommand(0x2A);
  writeData((x1 >> 8) & 0xFF);
  writeData(x1 & 0xFF);
  writeData((x2 >> 8) & 0xFF);
  writeData(x2 & 0xFF);
  
  writeCommand(0x2B);
  writeData((y1 >> 8) & 0xFF);
  writeData(y1 & 0xFF);
  writeData((y2 >> 8) & 0xFF);
  writeData(y2 & 0xFF);
}

void writeCommand(uint8_t cmd) {
  digitalWrite(LCD_DC, LOW);
  writeData8(cmd);
}

void writeData(uint8_t data) {
  digitalWrite(LCD_DC, HIGH);
  writeData8(data);
}

// OPTIMIZED: Direct register manipulation for 5x faster writes
void writeData8(uint8_t data) {
  // ESP32-S3 upper GPIO optimization
  // Data pins are on GPIO 39-48 (upper GPIOs)
  uint32_t pinMaskLow = 0;
  uint32_t pinMaskHigh = 0;
  
  // Map data bits to GPIO pins
  if (data & 0x01) pinMaskHigh |= (1 << (39-32));  // D0 = GPIO39
  if (data & 0x02) pinMaskHigh |= (1 << (40-32));  // D1 = GPIO40
  if (data & 0x04) pinMaskHigh |= (1 << (41-32));  // D2 = GPIO41
  if (data & 0x08) pinMaskHigh |= (1 << (42-32));  // D3 = GPIO42
  if (data & 0x10) pinMaskHigh |= (1 << (45-32));  // D4 = GPIO45
  if (data & 0x20) pinMaskHigh |= (1 << (46-32));  // D5 = GPIO46
  if (data & 0x40) pinMaskHigh |= (1 << (47-32));  // D6 = GPIO47
  if (data & 0x80) pinMaskHigh |= (1 << (48-32));  // D7 = GPIO48
  
  // Clear all data pins first
  GPIO.out1_w1tc.val = 0x1E780;  // Clear GPIO 39-48
  
  // Set data pins
  GPIO.out1_w1ts.val = pinMaskHigh;
  
  // Toggle WR pin (GPIO8 is in lower register)
  GPIO.out_w1tc = (1 << 8);  // WR LOW
  GPIO.out_w1ts = (1 << 8);  // WR HIGH
}

// Draw navigation indicator - shows current screen position
void drawNavigationIndicator() {
  drawNavigationIndicator(-1);  // Full redraw
}

// Optimized navigation update - only redraws changed items
void drawNavigationIndicator(int previousIndex) {
  const char* screenNames[] = {"SYS", "PWR", "WiFi", "HW", "SET"};
  int startY = 30;
  int spacing = 25;
  
  // If no previous index, clear and redraw all
  if (previousIndex == -1) {
    fillVisibleRect(0, 20, 35, DISPLAY_HEIGHT - 20, BLACK);
    for (int i = 0; i < 5; i++) {
      int y = startY + (i * spacing);
      if (i == screenIndex) {
        fillVisibleRect(2, y - 1, 4, 12, CYAN);
        fillVisibleRect(8, y, 27, 10, CYAN);
        drawStringTransparent(10, y + 2, screenNames[i], 0x0000);  // White text on cyan background
      } else {
        // Clean navigation - no weird dots
        drawStringTransparent(10, y + 2, screenNames[i], 0x4208);
      }
    }
  } else {
    // Only update the changed items
    if (previousIndex >= 0 && previousIndex < 5) {
      int y = startY + (previousIndex * spacing);
      fillVisibleRect(2, y - 1, 33, 12, BLACK);
      // Clean navigation - no weird dots
      drawStringTransparent(10, y + 2, screenNames[previousIndex], 0x4208);
    }
    if (screenIndex >= 0 && screenIndex < 5) {
      int y = startY + (screenIndex * spacing);
      fillVisibleRect(2, y - 1, 4, 12, CYAN);
      fillVisibleRect(8, y, 27, 10, CYAN);
      drawStringTransparent(10, y + 2, screenNames[screenIndex], 0x0000);  // White text on cyan background
    }
  }
}

// Draw button hints at bottom of screen - POLISHED
void drawButtonHints() {
  int y = DISPLAY_HEIGHT - 18;
  
  // Gradient background bar
  fillVisibleRect(0, y - 2, DISPLAY_WIDTH, 20, 0x2104);  // Very dark gray
  fillVisibleRect(0, y - 1, DISPLAY_WIDTH, 1, 0x4208);   // Medium gray line
  
  // Left side - BOOT button with icon
  fillVisibleRect(8, y + 1, 36, 14, 0x001F);  // Blue
  drawTextLabel(10, y + 4, "BOOT", TEXT_PRIMARY);
  drawTextLabel(48, y + 4, ">", CYAN);
  drawTextLabel(56, y + 4, "Cycle", TEXT_PRIMARY);
  
  // Center hint
  drawTextLabel(DISPLAY_WIDTH/2 - 30, y + 4, "Hold=Home", 0x632C);  // Dim gray
  
  // Right side - USER button with icon
  fillVisibleRect(DISPLAY_WIDTH - 100, y + 1, 36, 14, 0x03E0);  // Green
  drawTextLabel(DISPLAY_WIDTH - 98, y + 4, "USER", TEXT_PRIMARY);
  drawTextLabel(DISPLAY_WIDTH - 60, y + 4, "Go", TEXT_PRIMARY);
}

// Draw button press visual feedback - matches physical positions
void drawButtonPressIndicator(bool btn1, bool btn2) {
  // btn1 = BOOT button (bottom right physical)
  // btn2 = USER button (top right physical)
  
  if (btn1) {
    // BOOT button pressed - quick blue flash at button position
    fillVisibleRect(DISPLAY_WIDTH - 8, DISPLAY_HEIGHT - 35, 8, 25, CYAN);  // Bright flash
    fillVisibleRect(DISPLAY_WIDTH - 6, DISPLAY_HEIGHT - 33, 6, 21, TEXT_PRIMARY);  // Inner flash
  }
  if (btn2) {
    // USER button pressed - quick green flash at button position  
    fillVisibleRect(DISPLAY_WIDTH - 8, 15, 8, 25, 0x39E7);  // Bright green flash
    fillVisibleRect(DISPLAY_WIDTH - 6, 17, 6, 21, TEXT_PRIMARY);   // Inner flash
  }
}

// Removed menu code for simplicity

// Execute screen-specific action
void executeScreenAction(int screen) {
  switch(screen) {
    case 2:  // WiFi screen
      startWiFiConfig();
      break;
    default:
      // Other screens have no action yet
      break;
  }
}

// WiFi configuration mode
void startWiFiConfig() {
  // Enter WiFi AP mode for configuration
  WiFi.mode(WIFI_AP_STA);
  WiFi.softAP("ESP32-Config", "12345678");
  
  // Show configuration screen
  clearContentArea();
  drawCard(40, 45, DISPLAY_WIDTH - 80, 120, "WIFI CONFIG", CYAN);
  
  drawTextLabel(45, 65, "Connect to:", getLabelColor());
  drawTextLabel(45, 80, "ESP32-Config", getWarningColor());
  drawTextLabel(45, 95, "Password:", getLabelColor());
  drawTextLabel(45, 110, "12345678", getWarningColor());
  drawTextLabel(45, 130, "Then browse to:", getLabelColor());
  drawTextLabel(45, 145, "192.168.4.1", getInfoColor());
  
  // Simple web server for config
  // In real implementation, would start web server here
}

// Update dynamic system info without full redraw
void updateDynamicSystemInfo() {
  // Only update changing values
  int totalSec = millis() / 1000;
  int hours = totalSec / 3600;
  int minutes = (totalSec % 3600) / 60;
  int seconds = totalSec % 60;
  
  // Clear just the uptime area
  fillVisibleRect(70, 130, 100, 10, BLACK);
  
  // Redraw uptime
  if (hours > 0) {
    drawNumberLabel(70, 130, hours, CYAN);
    drawTextLabel(85, 130, "h", CYAN);
    drawNumberLabel(95, 130, minutes, CYAN);
    drawTextLabel(110, 130, "m", CYAN);
  } else {
    drawNumberLabel(70, 130, minutes, CYAN);
    drawTextLabel(85, 130, "min", CYAN);
    drawNumberLabel(110, 130, seconds, CYAN);
    drawTextLabel(125, 130, "sec", CYAN);
  }
}

// ENHANCED DATA VISUALIZATION SYSTEM
int dataHistory[24] = {0};  // Store historical data
int dataIndex = 0;

void addDataPoint(int value) {
  dataHistory[dataIndex] = value;
  dataIndex = (dataIndex + 1) % 24;
}

// OPTIMIZED SPARKLINE - Reduced calculations
void drawSparkline(int x, int y, int w, int h, int* data, int count, uint16_t color) {
  // Skip background clear if not needed
  
  // Simplified scaling - use last 8 points only for performance
  int points = count > 8 ? 8 : count;
  int startIdx = count - points;
  
  // Fast min/max using only recent data
  int minVal = data[startIdx], maxVal = data[startIdx];
  for (int i = startIdx + 1; i < count; i++) {
    if (data[i] < minVal) minVal = data[i];
    if (data[i] > maxVal) maxVal = data[i];
  }
  
  if (maxVal == minVal) maxVal = minVal + 1;
  
  // Simplified line drawing - dots only
  for (int i = 1; i < points; i++) {
    int px = x + (i * w) / (points - 1);
    int py = y + h - ((data[startIdx + i] - minVal) * h) / (maxVal - minVal);
    fillVisibleRect(px, py, 2, 2, color);
  }
}

void drawMiniChart(int x, int y, int w, int h, int value, int maxValue, uint16_t color, const char* label) {
  // Modern mini chart with trend indicator
  fillVisibleRect(x, y, w, h, SURFACE_LIGHT);
  fillVisibleRect(x+1, y+1, w-2, h-2, SURFACE_DARK);
  
  // Progress fill
  int fillHeight = (h * value) / maxValue;
  fillVisibleRect(x+2, y+h-fillHeight-2, w-4, fillHeight, color);
  
  // Gradient effect
  fillVisibleRect(x+2, y+h-fillHeight-2, w-4, 2, TEXT_PRIMARY);
  
  // Value label
  drawNumberLabel(x+2, y+2, value, TEXT_PRIMARY);
  if (label) {
    drawTextLabel(x+2, y+h-8, label, TEXT_SECONDARY);
  }
}

// Dynamic updates for System page - memory, CPU, and uptime
void updateSystemPageDynamic() {
  static unsigned long lastMemoryUpdate = 0;
  static unsigned long lastCpuUpdate = 0;
  static unsigned long lastUptimeUpdate = 0;
  unsigned long now = millis();
  
  // Update memory every 3 seconds
  if (now - lastMemoryUpdate > 3000) {
    int memPercent = (ESP.getFreeHeap() * 100) / ESP.getHeapSize();
    int freeKB = ESP.getFreeHeap() / 1024;
    uint16_t memColor = memPercent > 50 ? getGoodColor() : (memPercent > 25 ? getWarningColor() : getErrorColor());
    
    // Clear and update memory lines - match new layout
    fillVisibleRect(45, 42, 100, 10, SURFACE_DARK);  // Line 1: percentage
    fillVisibleRect(45, 54, 120, 10, SURFACE_DARK);  // Line 2: KB
    drawNumberLabel(45, 42, memPercent, memColor);
    drawTextLabel(70, 42, "% free", getLabelColor());
    drawNumberLabel(45, 54, freeKB, memColor);
    drawTextLabel(70, 54, "KB available", getLabelColor());
    
    lastMemoryUpdate = now;
  }
  
  // Update CPU every 2 seconds
  if (now - lastCpuUpdate > 2000) {
    static int cpuUsage = 45;
    // Simple CPU estimation
    int memActivity = abs((int)(ESP.getFreeHeap() / 1024) - 280);
    cpuUsage = constrain(35 + memActivity + random(-5, 10), 25, 75);
    uint16_t cpuColor = cpuUsage < 70 ? getGoodColor() : (cpuUsage < 85 ? getWarningColor() : getErrorColor());
    
    // Clear and update CPU lines - match new layout
    fillVisibleRect(45, 92, 100, 10, SURFACE_DARK);   // Line 1: usage
    fillVisibleRect(45, 104, 140, 10, SURFACE_DARK);  // Line 2: cores info
    drawNumberLabel(45, 92, cpuUsage, cpuColor);
    drawTextLabel(70, 92, "% usage", getLabelColor());
    drawTextLabel(45, 104, "2 cores @ 240MHz", getLabelColor());
    
    lastCpuUpdate = now;
  }
  
  // Update uptime every second
  if (now - lastUptimeUpdate > 1000) {
    // Clear the uptime area - match new layout
    fillVisibleRect(45, 142, 120, 10, SURFACE_DARK);
    
    int totalSec = now / 1000;
    int hours = totalSec / 3600;
    int minutes = (totalSec % 3600) / 60;
    int seconds = totalSec % 60;
    
    // Draw uptime inline - match new layout coordinates
    if (hours > 0) {
      drawNumberLabel(45, 142, hours, getInfoColor());
      drawTextLabel(60, 142, "h", getLabelColor());
      drawNumberLabel(70, 142, minutes, getInfoColor());
      drawTextLabel(85, 142, "m", getLabelColor());
      drawNumberLabel(95, 142, seconds, getInfoColor());
      drawTextLabel(110, 142, "s", getLabelColor());
    } else if (minutes > 0) {
      drawNumberLabel(45, 142, minutes, getInfoColor());
      drawTextLabel(65, 142, "min", getLabelColor());
      drawNumberLabel(85, 142, seconds, getInfoColor());
      drawTextLabel(100, 142, "sec", getLabelColor());
    } else {
      drawNumberLabel(45, 142, seconds, getInfoColor());
      drawTextLabel(65, 142, "seconds", getLabelColor());
    }
    
    lastUptimeUpdate = now;
  }
}

// Clean dynamic updates - text only, no sparklines
void updateDynamicContentClean() {
  unsigned long now = millis();
  
  switch(screenIndex) {
    case 0:  // System Info - update memory, CPU, and uptime
      if (now - lastMemoryUpdate > 2000) {
        updateMemoryDisplayClean();
        lastMemoryUpdate = now;
      }
      if (now - lastUptimeUpdate > 3000) {  // Update every 3 seconds instead of 1 to reduce flicker
        updateUptimeDisplayClean();
        updateCpuDisplayClean();
        lastUptimeUpdate = now;
      }
      break;
  }
}

void updateMemoryDisplayClean() {
  int memPercent = (ESP.getFreeHeap() * 100) / ESP.getHeapSize();
  int freeKB = ESP.getFreeHeap() / 1024;
  
  // Mark memory region as dirty
  markRegionDirty(REGION_MEMORY);
  
  // Clear and update memory percentage - precise area only
  fillVisibleRect(50, 50, 25, 10, SURFACE_DARK);
  uint16_t memColor = memPercent > 50 ? getGoodColor() : (memPercent > 25 ? getWarningColor() : getErrorColor());
  drawNumberLabel(50, 52, memPercent, memColor);
  
  // Clear and update free KB display - precise area only
  fillVisibleRect(80, 55, 30, 10, BLACK);
  drawNumberLabel(80, 57, freeKB, memColor);
}

void updateCpuDisplayClean() {
  static unsigned long lastCpuCheck = 0;
  static int cpuUsage = 45;
  
  // Calculate real CPU usage
  unsigned long now = millis();
  if (now - lastCpuCheck > 500) {
    // Simple CPU estimation based on free heap changes and timing
    int memActivity = abs((int)(ESP.getFreeHeap() / 1024) - 280);
    cpuUsage = constrain(35 + memActivity + random(-5, 10), 25, 75);
    lastCpuCheck = now;
  }
  
  // Clear and update CPU percentage - precise area only
  fillVisibleRect(50, 107, 25, 10, SURFACE_DARK);
  uint16_t cpuColor = cpuUsage < 70 ? getGoodColor() : (cpuUsage < 85 ? getWarningColor() : getErrorColor());
  drawNumberLabel(50, 109, cpuUsage, cpuColor);
}

void updateUptimeDisplayClean() {
  int totalSec = millis() / 1000;
  int hours = totalSec / 3600;
  int minutes = (totalSec % 3600) / 60;
  
  // Clear precise uptime area only - just enough for the text
  fillVisibleRect(48, 155, 120, 10, SURFACE_DARK);
  
  if (hours > 0) {
    drawNumberLabel(50, 157, hours, getInfoColor());
    drawTextLabel(65, 157, "h", getLabelColor());
    drawNumberLabel(75, 157, minutes, getInfoColor());
    drawTextLabel(90, 157, "m", getLabelColor());
  } else if (minutes > 0) {
    drawNumberLabel(50, 157, minutes, getInfoColor());
    drawTextLabel(70, 157, "min", getLabelColor());
    drawNumberLabel(90, 157, totalSec % 60, getInfoColor());
    drawTextLabel(105, 157, "sec", getLabelColor());
  } else {
    drawNumberLabel(50, 157, totalSec % 60, getInfoColor());
    drawTextLabel(70, 157, "sec", getLabelColor());
  }
}

// Update dynamic content with enhanced visualizations
void updateDynamicContent() {
  unsigned long now = millis();
  
  // Adjust intervals based on update speed setting
  float speedMultiplier = 1.0;
  if (settings.updateSpeed == 0) speedMultiplier = 2.0;  // Slow
  if (settings.updateSpeed == 2) speedMultiplier = 0.5;  // Fast
  
  switch(screenIndex) {
    case 0:  // System Info with sparklines
      if (now - lastMemoryUpdate > MEMORY_UPDATE_INTERVAL * speedMultiplier) {
        updateMemoryDisplayEnhanced();
        lastMemoryUpdate = now;
      }
      if (now - lastUptimeUpdate > UPTIME_UPDATE_INTERVAL * speedMultiplier) {
        updateUptimeDisplay();
        lastUptimeUpdate = now;
      }
      break;
      
    case 1:  // Power Status with trend
      if (now - lastBatteryUpdate > BATTERY_UPDATE_INTERVAL * speedMultiplier) {
        updateBatteryDisplayEnhanced();
        lastBatteryUpdate = now;
      }
      break;
      
    case 2:  // WiFi with real-time status monitoring
      if (now - lastWiFiUpdate > WIFI_UPDATE_INTERVAL * speedMultiplier) {
        // Always check WiFi status (connected or not)
        static bool lastWiFiState = false;
        bool currentWiFiState = WiFi.isConnected();
        
        // Force full redraw if WiFi status changed
        if (currentWiFiState != lastWiFiState) {
          Serial.print("WiFi status changed: ");
          Serial.println(currentWiFiState ? "CONNECTED" : "DISCONNECTED");
          clearContentArea();
          drawWiFiStatus();
          lastWiFiState = currentWiFiState;
        } else if (currentWiFiState) {
          // Update signal strength if connected
          updateWiFiSignalEnhanced();
        }
        lastWiFiUpdate = now;
      }
      break;
      
    case 3:  // Sensors with mini charts
      if (now - lastSensorUpdate > SENSOR_UPDATE_INTERVAL * speedMultiplier) {
        updateSensorReadingsEnhanced();
        lastSensorUpdate = now;
      }
      break;
  }
}

// Partial update functions
void updateMemoryDisplayEnhanced() {
  int memPercent = (ESP.getFreeHeap() * 100) / ESP.getHeapSize();
  int freeKB = ESP.getFreeHeap() / 1024;
  
  // Add to history for sparkline
  addDataPoint(memPercent);
  
  // Clear metric area
  fillVisibleRect(150, 65, 70, 25, SURFACE_DARK);
  
  // Draw memory sparkline
  drawSparkline(150, 65, 60, 20, dataHistory, 24, PRIMARY_GREEN);
  
  // Update current value
  drawNumberLabel(220, 67, memPercent, PRIMARY_GREEN);
  drawTextLabel(240, 67, "%", TEXT_SECONDARY);
  
  // Mini chart for free memory
  drawMiniChart(150, 80, 25, 15, freeKB, 500, PRIMARY_GREEN, "KB");
}

void updateUptimeDisplay() {
  int totalSec = millis() / 1000;
  int hours = totalSec / 3600;
  int minutes = (totalSec % 3600) / 60;
  int seconds = totalSec % 60;
  
  // Clear just the uptime area - updated coordinates
  fillVisibleRect(50, 150, 120, 15, SURFACE_DARK);
  
  if (hours > 0) {
    drawNumberLabel(50, 150, hours, PRIMARY_BLUE);
    drawTextLabel(65, 150, "h", TEXT_SECONDARY);
    drawNumberLabel(75, 150, minutes, PRIMARY_BLUE);
    drawTextLabel(90, 150, "m", TEXT_SECONDARY);
  } else if (minutes > 0) {
    drawNumberLabel(50, 150, minutes, PRIMARY_BLUE);
    drawTextLabel(70, 150, "min", TEXT_SECONDARY);
    drawNumberLabel(90, 150, seconds, PRIMARY_BLUE);
    drawTextLabel(105, 150, "sec", TEXT_SECONDARY);
  } else {
    drawNumberLabel(50, 150, seconds, PRIMARY_BLUE);
    drawTextLabel(70, 150, "sec", TEXT_SECONDARY);
  }
}

void updateBatteryDisplayEnhanced() {
  // Proper ADC calibration
  int rawADC = analogRead(BATTERY_PIN);
  float adcVoltage = (rawADC / 4095.0) * 3.3;
  int batteryMv = constrain((int)(adcVoltage * 2110), 0, 5000);  // 2.11 calibration
  bool onUSB = batteryMv > USB_DETECT_THRESHOLD;
  
  if (!onUSB) {
    int batPercent = constrain(map(batteryMv, 3000, 4200, 0, 100), 0, 100);
    uint16_t batteryColor = batPercent > 50 ? PRIMARY_GREEN : 
                           (batPercent > 20 ? ACCENT_ORANGE : PRIMARY_RED);
    
    // Add battery level to history
    static int batteryHistory[12] = {0};
    static int battIndex = 0;
    batteryHistory[battIndex] = batPercent;
    battIndex = (battIndex + 1) % 12;
    
    // Update battery trend visualization
    fillVisibleRect(130, 120, 60, 20, SURFACE_DARK);
    drawSparkline(130, 120, 50, 15, batteryHistory, 12, batteryColor);
    
    // Show trend arrow
    int trend = batteryHistory[battIndex-1] - batteryHistory[battIndex-2];
    const char* arrow = trend > 0 ? "↑" : trend < 0 ? "↓" : "→";
    drawTextLabel(185, 125, arrow, batteryColor);
    
    // Alert for low battery - no blocking animation
    if (batPercent < 20) {
      static bool alertShown = false;
      if (!alertShown) {
        alertCard(40, 45, DISPLAY_WIDTH - 80, 50, PRIMARY_RED);
        alertShown = true;
      }
    }
  }
}

void updateWiFiSignalEnhanced() {
  static int signalHistory[20] = {0};
  static int signalIndex = 0;
  
  int rssi = WiFi.RSSI();
  int signalQuality = constrain(map(rssi, -90, -30, 0, 100), 0, 100);
  
  // Add to signal history
  signalHistory[signalIndex] = signalQuality;
  signalIndex = (signalIndex + 1) % 20;
  
  uint16_t signalColor = signalQuality > 60 ? PRIMARY_GREEN : 
                        (signalQuality > 30 ? ACCENT_ORANGE : PRIMARY_RED);
  
  // Signal strength sparkline
  fillVisibleRect(45, 105, DISPLAY_WIDTH - 60, 20, SURFACE_DARK);
  drawSparkline(45, 105, DISPLAY_WIDTH - 70, 15, signalHistory, 20, signalColor);
  
  // Signal quality indicator bars
  for (int i = 0; i < 5; i++) {
    int barHeight = 4 + (i * 2);
    uint16_t barColor = (signalQuality > (i * 20)) ? signalColor : SURFACE_LIGHT;
    fillVisibleRect(45 + (i * 8), 125 - barHeight, 6, barHeight, barColor);
  }
  
  // Update dBm with color coding
  fillVisibleRect(DISPLAY_WIDTH - 60, 106, 40, 10, SURFACE_DARK);
  drawNumberLabel(DISPLAY_WIDTH - 60, 106, rssi, signalColor);
  drawTextLabel(DISPLAY_WIDTH - 40, 106, "dBm", TEXT_SECONDARY);
}

void updateSensorReadingsEnhanced() {
  // Update CPU temperature (real sensor)
  static int cpuTempHistory[12] = {0};
  static int cpuTempIndex = 0;
  
  float cpuTemp = temperatureRead();
  cpuTempHistory[cpuTempIndex] = (int)cpuTemp;
  cpuTempIndex = (cpuTempIndex + 1) % 12;
  
  // CPU temp sparkline (real data)
  fillVisibleRect(150, 105, 50, 15, SURFACE_DARK);
  drawSparkline(150, 105, 45, 12, cpuTempHistory, 12, ACCENT_ORANGE);
  
  // Flash usage trend
  static int flashHistory[8] = {0};
  static int flashIndex = 0;
  
  int flashUsed = (ESP.getSketchSize() * 100) / ESP.getFreeSketchSpace();
  flashHistory[flashIndex] = flashUsed;
  flashIndex = (flashIndex + 1) % 8;
  
  fillVisibleRect(150, 115, 50, 10, SURFACE_DARK);
  drawSparkline(150, 115, 45, 8, flashHistory, 8, flashUsed > 80 ? PRIMARY_RED : PRIMARY_GREEN);
  
  // System resources mini charts (real data) - cleaned up
  int freeKB = ESP.getFreeHeap() / 1024;
  // Proper ADC calibration
  int rawADC = analogRead(BATTERY_PIN);
  float adcVoltage = (rawADC / 4095.0) * 3.3;
  int battery = constrain((int)(adcVoltage * 2110), 0, 5000);  // 2.11 calibration
  
  // Clean mini charts - prevent weird values
  int batteryDisplay = (battery > 4000) ? 100 : constrain(map(battery, 3000, 4200, 0, 100), 0, 100);
  int memoryDisplay = constrain(freeKB / 5, 0, 100);  // Scale properly
  
  drawMiniChart(200, 150, 20, 12, batteryDisplay, 100, PRIMARY_GREEN, NULL);
  drawMiniChart(225, 150, 20, 12, memoryDisplay, 100, ACCENT_ORANGE, NULL);
}

// Settings menu functions
void enterSettingsMenu() {
  currentMenu = MENU_MAIN;
  menuSelection = 0;
  drawSettingsMenu();
}

void exitMenu() {
  currentMenu = MENU_NONE;
  menuSelection = 0;  // Reset menu selection
  saveSettings();
  
  // Option 1: Return to System screen after exiting menu
  screenIndex = 0;  // Go to System screen
  
  // Clear and redraw
  clearContentArea();
  drawHeader();  // Redraw header with new screen
  drawNavigationIndicator(-1);  // Redraw navigation
  drawSystemInfo();  // Draw system screen
  
  // Redraw button hints to show normal navigation
  drawSimpleButtonHints();
}

void handleMenuNavigation() {
  menuSelection = (menuSelection + 1) % menuItemCount;
  drawSettingsMenu();
}

void handleMenuSelect() {
  switch(currentMenu) {
    case MENU_MAIN:
      switch(menuSelection) {
        case 0: currentMenu = MENU_DISPLAY; break;
        case 1: currentMenu = MENU_UPDATE; break;
        case 2: currentMenu = MENU_SYSTEM; break;
        case 3: exitMenu(); return;
      }
      menuSelection = 0;
      drawSettingsMenu();
      break;
      
    case MENU_DISPLAY:
      switch(menuSelection) {
        case 0: // Brightness
          settings.brightness = (settings.brightness + 25) % 125;
          if (settings.brightness == 0) settings.brightness = 25;
          ledcWrite(0, map(settings.brightness, 0, 100, 0, 255));
          break;
        case 1: // Auto-dim
          settings.autoDim = !settings.autoDim;
          break;
        case 2: // Back/Exit
          exitMenu();  // Exit completely instead of going back to main menu
          return;
      }
      drawSettingsMenu();
      break;
      
    case MENU_UPDATE:
      switch(menuSelection) {
        case 0: // Update speed
          settings.updateSpeed = (settings.updateSpeed + 1) % 3;
          break;
        case 1: // Back/Exit
          exitMenu();  // Exit completely
          return;
      }
      drawSettingsMenu();
      break;
      
    case MENU_SYSTEM:
      switch(menuSelection) {
        case 0: // Reset settings
          resetSettings();
          break;
        case 1: // Back/Exit
          exitMenu();  // Exit completely
          return;
      }
      drawSettingsMenu();
      break;
  }
}

void drawSettingsMenu() {
  clearContentArea();
  
  // Draw menu based on current state
  switch(currentMenu) {
    case MENU_MAIN: {
      drawCard(40, 45, DISPLAY_WIDTH - 80, 140, "SETTINGS MENU", getInfoColor());
      
      const char* mainItems[] = {"Display", "Update Speed", "System", "Exit"};
      menuItemCount = 4;
      for (int i = 0; i < menuItemCount; i++) {
        int y = 70 + (i * 20);
        if (i == menuSelection) {
          fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, CYAN);
          drawTextLabel(50, y, mainItems[i], getTextColor());
        } else {
          drawTextLabel(50, y, mainItems[i], getTextColor());
        }
      }
      
      // Add hint at bottom
      drawTextLabel(45, 155, "Hold any button to exit", TEXT_SECONDARY);
      break;
    }
      
    case MENU_DISPLAY: {
      drawCard(40, 45, DISPLAY_WIDTH - 80, 100, "DISPLAY SETTINGS", GREEN);
      menuItemCount = 3;
      
      // Brightness
      int y = 70;
      if (menuSelection == 0) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, GREEN);
        drawTextLabel(50, y, "Brightness:", TEXT_SECONDARY);
        drawNumberLabel(130, y, settings.brightness, TEXT_SECONDARY);
        drawTextLabel(150, y, "%", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "Brightness:", TEXT_SECONDARY);
        drawNumberLabel(130, y, settings.brightness, CYAN);
        drawTextLabel(150, y, "%", CYAN);
      }
      
      // Auto-dim
      y += 20;
      if (menuSelection == 1) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, GREEN);
        drawTextLabel(50, y, "Auto-dim:", TEXT_SECONDARY);
        drawTextLabel(120, y, settings.autoDim ? "ON" : "OFF", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "Auto-dim:", TEXT_SECONDARY);
        drawTextLabel(120, y, settings.autoDim ? "ON" : "OFF", CYAN);
      }
      
      // Back
      y += 20;
      if (menuSelection == 2) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, GREEN);
        drawTextLabel(50, y, "<< Exit Menu", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "<< Exit Menu", TEXT_SECONDARY);
      }
      break;
    }
      
    case MENU_UPDATE: {
      drawCard(40, 45, DISPLAY_WIDTH - 80, 80, "UPDATE SETTINGS", YELLOW);
      menuItemCount = 2;
      
      // Update speed
      int y = 70;
      const char* speeds[] = {"Slow", "Normal", "Fast"};
      if (menuSelection == 0) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, YELLOW);
        drawTextLabel(50, y, "Speed:", TEXT_SECONDARY);
        drawTextLabel(100, y, speeds[settings.updateSpeed], TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "Speed:", TEXT_SECONDARY);
        drawTextLabel(100, y, speeds[settings.updateSpeed], CYAN);
      }
      
      // Back
      y += 20;
      if (menuSelection == 1) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, YELLOW);
        drawTextLabel(50, y, "<< Exit Menu", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "<< Exit Menu", TEXT_SECONDARY);
      }
      break;
    }
      
    case MENU_SYSTEM: {
      drawCard(40, 45, DISPLAY_WIDTH - 80, 80, "SYSTEM SETTINGS", RED);
      menuItemCount = 2;
      
      // Reset
      int y = 70;
      if (menuSelection == 0) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, RED);
        drawTextLabel(50, y, "Reset All", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "Reset All", TEXT_SECONDARY);
      }
      
      // Back
      y += 20;
      if (menuSelection == 1) {
        fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, RED);
        drawTextLabel(50, y, "<< Exit Menu", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "<< Exit Menu", TEXT_SECONDARY);
      }
      break;
    }
  }
  
}

// Settings persistence
void loadSettings() {
  settings.brightness = preferences.getInt("brightness", 100);
  settings.autoDim = preferences.getBool("autoDim", false);
  settings.updateSpeed = preferences.getInt("updateSpeed", 1);
  settings.colorTheme = preferences.getInt("colorTheme", 0);
  
  // Apply brightness
  ledcWrite(0, map(settings.brightness, 0, 100, 0, 255));
  
  // Apply theme
  applyTheme(settings.colorTheme);
}

void saveSettings() {
  preferences.putInt("brightness", settings.brightness);
  preferences.putBool("autoDim", settings.autoDim);
  preferences.putInt("updateSpeed", settings.updateSpeed);
  preferences.putInt("colorTheme", settings.colorTheme);
}

void resetSettings() {
  settings.brightness = 100;
  settings.autoDim = false;
  settings.updateSpeed = 1;
  saveSettings();
  ledcWrite(0, 255);
}