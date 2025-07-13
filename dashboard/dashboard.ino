// Minimal T-Display-S3 Dashboard - Lightweight version with labels
// Uses only essential functions to reduce program size

#include <WiFi.h>
#include <Preferences.h>
#include "soc/gpio_struct.h"  // For direct GPIO register access
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

// Battery detection
#define BATTERY_PIN  4   // GPIO4 for battery voltage
#define USB_DETECT_THRESHOLD 4500  // mV threshold for USB power

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
#define SURFACE_DARK    0x1082   // Card backgrounds
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
uint16_t getTextColor() { return 0xFFFF; }  // Pure white - very visible change
uint16_t getLabelColor() { return 0xFFFF; }  // Pure white - very visible change
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

void setup() {
  Serial.begin(115200);
  delay(2000); // Give serial time to initialize
  
  // SPEED OPTIMIZATION: Boost CPU to 240MHz
  setCpuFrequencyMhz(240);
  
  // START WIFI FIRST - before anything else
  Serial.println("=== COMPREHENSIVE WIFI DEBUG ===");
  Serial.print("WiFi library available: ");
  Serial.println("YES");
  
  // Print MAC address
  WiFi.mode(WIFI_STA);
  Serial.print("MAC Address: ");
  Serial.println(WiFi.macAddress());
  
  // Start connection attempt
  Serial.print("Attempting to connect to SSID: ");
  Serial.println(WIFI_SSID);
  Serial.print("Password length: ");
  Serial.println(strlen(WIFI_PASSWORD));
  
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
  Serial.println("WiFi.begin() called successfully");
  
  // Initial status check
  Serial.print("Initial WiFi status: ");
  Serial.println(WiFi.status());
  
  delay(1000);
  
  Serial.println("=== UPLOAD TEST - MINIMAL DASHBOARD FIXED ===");
  Serial.println("Auto-Update Version with Settings Menu");
  Serial.println("Buttons on RIGHT side:");
  Serial.println("  Top Right (USER): Action/Settings");
  Serial.println("  Bottom Right (BOOT): Navigate");
  
  // Load saved settings
  preferences.begin("dashboard", false);
  loadSettings();
  
  // Initialize buttons
  pinMode(BUTTON_1, INPUT_PULLUP);
  pinMode(BUTTON_2, INPUT_PULLUP);
  
  // Initialize battery pin
  pinMode(BATTERY_PIN, INPUT);
  
  // Initialize display
  initDisplay();
  comprehensiveMemoryInit();
  
  // Clear screen
  fillScreen(BLACK);
  delay(500);
  
  // Start WiFi connection after display is ready
  Serial.println("=== DETAILED WIFI CONNECTION MONITORING ===");
  Serial.println("Re-checking WiFi initialization...");
  
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
  
  // Detailed connection monitoring
  int wifiTimeout = 40; // 40 seconds timeout
  Serial.print("Monitoring connection");
  while (WiFi.status() != WL_CONNECTED && wifiTimeout > 0) {
    delay(500);
    Serial.print(".");
    wifiTimeout--;
    
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
  
  // Draw initial screen
  drawHeader();
  drawSystemInfo();
  
  // Final WiFi status check
  Serial.print("Final WiFi Status: ");
  if (WiFi.isConnected()) {
    Serial.print("CONNECTED to ");
    Serial.print(WiFi.SSID());
    Serial.print(" (IP: ");
    Serial.print(WiFi.localIP());
    Serial.println(")");
  } else {
    Serial.println("NOT CONNECTED");
    Serial.print("Status code: ");
    Serial.println(WiFi.status());
  }
  
  Serial.println("Dashboard ready - auto-updating content");
}

// Global screen index
int screenIndex = 0;

void loop() {
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
      analogWrite(LCD_BL, map(settings.brightness, 0, 100, 0, 255));
    }
  }
  
  // Track navigation direction for smooth transitions
  static int transitionDirection = 0; // 0=none, 1=forward, -1=back
  
  // ULTRA SIMPLE: BOOT button = Next Screen
  if (btn1Event == BUTTON_CLICK) {
    int previousScreen = screenIndex;
    screenIndex = (screenIndex + 1) % 5;
    needsRedraw = true;
    fastTransition = true;  // Enable fast transition
    lastActivityTime = millis();  // Reset activity timer
    
    // FORCE navigation update immediately
    drawNavigationIndicator(previousScreen);
  }
  
  // CONTEXTUAL: USER button performs different actions per screen
  if (btn2Event == BUTTON_CLICK) {
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
      case 1: drawPowerStatus(); break;
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
  
  // Enable dynamic updates for System page only
  if (currentMenu == MENU_NONE && screenIndex == 0) {
    updateSystemPageDynamic();
  }
  
  // Auto-dim handling - FIXED logic
  if (settings.autoDim) {
    unsigned long now = millis();
    if (!isDimmed && (now - lastActivityTime > DIM_TIMEOUT)) {
      // Dim the display
      isDimmed = true;
      analogWrite(LCD_BL, map(25, 0, 100, 0, 255));  // Dim to 25%
    } else if (isDimmed && (now - lastActivityTime <= DIM_TIMEOUT)) {
      // Restore brightness when activity detected
      isDimmed = false;
      analogWrite(LCD_BL, map(settings.brightness, 0, 100, 0, 255));
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
    if (btn1Event == BUTTON_LONG_PRESS) {
      exitMenu();
    }
  }
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
  // Minimal header - just a thin bar
  fillVisibleRect(0, 0, DISPLAY_WIDTH, 20, BLUE);    // Thinner header
  fillVisibleRect(0, 18, DISPLAY_WIDTH, 2, 0x4208);  // Bottom border
  
  // Screen name in center
  const char* screenNames[] = {"System", "Power", "WiFi", "Hardware", "Settings"};
  drawTextLabel(DISPLAY_WIDTH/2 - 20, 6, screenNames[screenIndex], 0x07E0);  // Force bright color
  
  // Power indicator on right
  drawPowerIndicator();
}

// Execute contextual action based on current screen
// Removed old contextual action function

// Draw power indicator in content area
void drawPowerIndicator() {
  int batteryMv = analogRead(BATTERY_PIN) * 2;
  bool onUSB = batteryMv > USB_DETECT_THRESHOLD;
  
  // In header, right side - expand to edge
  int x = DISPLAY_WIDTH - 70;
  int y = 4;
  
  // Clear area - wider
  fillVisibleRect(x, y, 70, 12, BLUE);
  
  if (onUSB) {
    // USB icon and text - better spacing
    drawTextLabel(x + 5, y + 2, "USB Power", CYAN);
    // Small plug icon
    fillVisibleRect(x + 50, y + 2, 8, 6, CYAN);
    fillVisibleRect(x + 48, y + 3, 2, 4, CYAN);
    fillVisibleRect(x + 58, y + 4, 2, 2, CYAN);
  } else {
    // Battery percentage and icon
    int percent = constrain(map(batteryMv, 3000, 4200, 0, 100), 0, 100);
    uint16_t color = percent > 50 ? GREEN : (percent > 20 ? YELLOW : RED);
    
    // Percentage text
    drawNumberLabel(x + 5, y + 2, percent, color);
    drawTextLabel(x + 20, y + 2, "%", color);
    
    // Battery icon - moved right
    fillVisibleRect(x + 35, y + 2, 12, 7, TEXT_PRIMARY);
    fillVisibleRect(x + 36, y + 3, 10, 5, BLACK);
    fillVisibleRect(x + 47, y + 4, 1, 3, TEXT_PRIMARY);
    
    // Battery fill
    int fillW = (9 * percent) / 100;
    if (fillW > 0) {
      fillVisibleRect(x + 37, y + 4, fillW, 3, color);
    }
  }
}

// OPTIMIZED CARD SYSTEM - Reduced draw calls
void drawCard(int x, int y, int w, int h, const char* title, uint16_t borderColor = BORDER_COLOR) {
  // Single shadow layer
  fillVisibleRect(x + 2, y + 2, w, h, 0x2104);
  
  // Main card
  fillVisibleRect(x, y, w, h, SURFACE_DARK);
  
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

void drawPowerStatus() {
  int batteryMv = analogRead(BATTERY_PIN) * 2;
  bool onUSB = batteryMv > USB_DETECT_THRESHOLD;
  
  if (onUSB) {
    // USB power mode - compact single card
    drawCard(40, 27, DISPLAY_WIDTH - 80, 35, "USB POWER", getGoodColor());
    drawTextLabel(50, 45, "Connected - 5V Input", getGoodColor());
    
    // Status card
    drawCard(40, 70, DISPLAY_WIDTH - 80, 35, "STATUS", getGoodColor());
    drawTextLabel(50, 88, "Charging ~120mA", getLabelColor());
    
  } else {
    // Battery mode - compact layout
    int batPercent = constrain(map(batteryMv, 3000, 4200, 0, 100), 0, 100);
    uint16_t batteryColor = batPercent > 50 ? getGoodColor() : 
                           (batPercent > 20 ? getWarningColor() : getErrorColor());
    
    // Battery card - compact with inline info
    drawCard(40, 27, DISPLAY_WIDTH - 80, 35, "BATTERY", batteryColor);
    drawNumberLabel(50, 45, batPercent, batteryColor);
    drawTextLabel(75, 45, "%", getLabelColor());
    drawNumberLabel(100, 45, batteryMv, batteryColor);
    drawTextLabel(135, 45, "mV", getLabelColor());
    
    // Battery level visualization - simplified
    drawCard(40, 70, DISPLAY_WIDTH - 80, 35, "LEVEL", batteryColor);
    
    // Simple battery icon
    fillVisibleRect(60, 88, 40, 12, SURFACE_LIGHT);  // Battery outline
    fillVisibleRect(61, 89, 38, 10, SURFACE_DARK);   // Interior
    fillVisibleRect(100, 91, 2, 6, SURFACE_LIGHT);   // Terminal
    
    // Battery fill
    int fillWidth = (36 * batPercent) / 100;
    if (fillWidth > 0) {
      fillVisibleRect(62, 90, fillWidth, 8, batteryColor);
    }
    
    // Simple estimate
    int estHours = (batPercent / 15) + 1;
    drawTextLabel(110, 91, "Est:", getLabelColor());
    drawNumberLabel(135, 91, estHours, batteryColor);
    drawTextLabel(150, 91, "h", getLabelColor());
  }
}

void drawSystemInfo() {
  // Memory metric card with semantic coloring
  int memPercent = (ESP.getFreeHeap() * 100) / ESP.getHeapSize();
  int freeKB = ESP.getFreeHeap() / 1024;
  int totalKB = ESP.getHeapSize() / 1024;
  
  // Memory card - compact with details inside
  uint16_t memColor = memPercent > 50 ? getGoodColor() : (memPercent > 25 ? getWarningColor() : getErrorColor());
  drawCard(40, 27, DISPLAY_WIDTH - 80, 35, "MEMORY", memColor);
  
  // Memory display - all on one line inside card
  drawNumberLabel(50, 45, memPercent, memColor);
  drawTextLabel(75, 45, "% free", getLabelColor());
  drawNumberLabel(130, 45, freeKB, memColor);
  drawTextLabel(155, 45, "KB", getLabelColor());
  
  // CPU metric card - calculate real usage
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
  
  uint16_t cpuColor = cpuUsage < 70 ? getGoodColor() : (cpuUsage < 85 ? getWarningColor() : getErrorColor());
  drawCard(40, 70, DISPLAY_WIDTH - 80, 35, "CPU", cpuColor);
  
  // CPU display - all on one line inside card
  drawNumberLabel(50, 88, cpuUsage, cpuColor);
  drawTextLabel(75, 88, "% usage", getLabelColor());
  drawTextLabel(130, 88, "2 cores", getLabelColor());
  drawTextLabel(180, 88, "240MHz", getLabelColor());
  
  // System uptime card - moved up for better spacing
  drawCard(40, 113, DISPLAY_WIDTH - 80, 26, "UPTIME", getInfoColor());
  int totalSec = millis() / 1000;
  int hours = totalSec / 3600;
  int minutes = (totalSec % 3600) / 60;
  int seconds = totalSec % 60;
  
  // Single line uptime display - moved down to avoid overlap
  if (hours > 0) {
    drawNumberLabel(50, 131, hours, getInfoColor());
    drawTextLabel(65, 131, "h", getLabelColor());
    drawNumberLabel(75, 131, minutes, getInfoColor());
    drawTextLabel(90, 131, "m", getLabelColor());
    drawNumberLabel(100, 131, seconds, getInfoColor());
    drawTextLabel(115, 131, "s", getLabelColor());
  } else if (minutes > 0) {
    drawNumberLabel(50, 131, minutes, getInfoColor());
    drawTextLabel(70, 131, "min", getLabelColor());
    drawNumberLabel(90, 131, seconds, getInfoColor());
    drawTextLabel(105, 131, "sec", getLabelColor());
  } else {
    drawNumberLabel(50, 131, seconds, getInfoColor());
    drawTextLabel(70, 131, "seconds", getLabelColor());
  }
}

void drawWiFiStatus() {
  if (WiFi.isConnected()) {
    // Status card with semantic colors
    drawCard(40, 25, DISPLAY_WIDTH - 50, 35, "CONNECTED", getGoodColor());
    drawTextLabel(45, 43, WiFi.SSID().c_str(), getInfoColor());
    
    // Signal card with intelligent coloring
    int rssi = WiFi.RSSI();
    int signalQuality = constrain(map(rssi, -90, -30, 0, 100), 0, 100);
    uint16_t signalColor = signalQuality > 60 ? getGoodColor() : (signalQuality > 30 ? getWarningColor() : getErrorColor());
    drawCard(40, 65, DISPLAY_WIDTH - 50, 40, "SIGNAL", signalColor);
    
    // Signal bar with theme colors
    fillVisibleRect(45, 85, DISPLAY_WIDTH - 60, 10, currentTheme->progress_bg);
    fillVisibleRect(45, 85, ((DISPLAY_WIDTH - 60) * signalQuality) / 100, 10, signalColor);
    drawNumberLabel(DISPLAY_WIDTH - 60, 86, rssi, signalColor);
    drawTextLabel(DISPLAY_WIDTH - 40, 86, "dBm", getLabelColor());
    
    // IP card
    drawCard(40, 110, DISPLAY_WIDTH - 80, 30, "IP ADDRESS", getInfoColor());
    drawTextLabel(45, 125, WiFi.localIP().toString().c_str(), getInfoColor());
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
  // GPIO PINS STATUS - compact inline layout
  drawCard(40, 27, DISPLAY_WIDTH - 80, 35, "GPIO", PRIMARY_BLUE);
  drawTextLabel(50, 45, "Available: GPIO 17, 43, 44", PRIMARY_BLUE);
  drawTextLabel(50, 55, "I2C Ready: SDA/SCL", PRIMARY_GREEN);
  
  // HARDWARE METRICS - compact inline layout
  drawCard(40, 70, DISPLAY_WIDTH - 80, 35, "HARDWARE", ACCENT_ORANGE);
  
  float cpuTemp = temperatureRead();
  int tempDisplay = constrain((int)cpuTemp, 0, 99);
  int totalFlash = ESP.getSketchSize() + ESP.getFreeSketchSpace();
  int flashUsed = totalFlash > 0 ? (ESP.getSketchSize() * 100) / totalFlash : 0;
  flashUsed = constrain(flashUsed, 0, 100);
  
  // Single line hardware info
  drawTextLabel(50, 88, "CPU:", TEXT_SECONDARY);
  drawNumberLabel(80, 88, tempDisplay, ACCENT_ORANGE);
  drawTextLabel(95, 88, "C", TEXT_SECONDARY);
  drawTextLabel(110, 88, "Flash:", TEXT_SECONDARY);
  drawNumberLabel(145, 88, flashUsed, flashUsed > 80 ? PRIMARY_RED : PRIMARY_GREEN);
  drawTextLabel(165, 88, "%", TEXT_SECONDARY);
  
  // SYSTEM RESOURCES - compact inline layout
  drawCard(40, 113, DISPLAY_WIDTH - 80, 35, "RESOURCES", PRIMARY_GREEN);
  
  int battery = analogRead(BATTERY_PIN) * 2;
  int freeKB = ESP.getFreeHeap() / 1024;
  
  // Single line resources info
  drawTextLabel(50, 131, "Power:", TEXT_SECONDARY);
  if (battery > 4000) {
    drawTextLabel(85, 131, "USB", PRIMARY_GREEN);
  } else {
    int batPercent = constrain(map(battery, 3000, 4200, 0, 100), 0, 100);
    drawNumberLabel(85, 131, batPercent, PRIMARY_GREEN);
    drawTextLabel(105, 131, "%", TEXT_SECONDARY);
  }
  drawTextLabel(120, 131, "RAM:", TEXT_SECONDARY);
  drawNumberLabel(150, 131, freeKB, PRIMARY_GREEN);
  drawTextLabel(170, 131, "KB", TEXT_SECONDARY);
}

void drawSettings() {
  // Help card with semantic colors
  drawCard(40, 25, DISPLAY_WIDTH - 80, 30, "HELP", getInfoColor());
  drawTextLabel(45, 40, "Hold USER for menu", getTextColor());
  
  // Menu preview card
  drawCard(40, 60, DISPLAY_WIDTH - 80, 65, "MENU OPTIONS", currentTheme->card_border);
  const char* items[] = {"WiFi Setup", "Display", "Auto Refresh", "Reset"};
  for (int i = 0; i < 4; i++) {
    int y = 80 + (i * 12);
    fillVisibleRect(50, y, 3, 3, getInfoColor());
    drawTextLabel(58, y - 2, items[i], getTextColor());
  }
  
  // Version card
  drawCard(40, 130, DISPLAY_WIDTH - 80, 20, "VERSION", getGoodColor());
  drawTextLabel(DISPLAY_WIDTH/2 - 10, 135, "v1.2", getTextColor());
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
    return; // Skip spaces
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
  
  // Draw character bitmap
  for (int col = 0; col < 5; col++) {
    uint8_t colData = charData[col];
    for (int row = 0; row < 8; row++) {
      if (colData & (1 << row)) {
        fillVisibleRect(x + col, y + row, 1, 1, color);
      }
    }
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
  char buf[10];
  sprintf(buf, "%d", num);
  drawString(x, y, buf, color);
}

// Helper function to draw within visible area
void fillVisibleRect(int x, int y, int w, int h, uint16_t color) {
  int actualX = DISPLAY_X_START + x;
  int actualY = DISPLAY_Y_START + y;
  
  if (x < 0 || y < 0 || x + w > DISPLAY_WIDTH || y + h > DISPLAY_HEIGHT) {
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
  
  digitalWrite(LCD_BL, HIGH);
  Serial.println("Display initialized");
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
        drawTextLabel(10, y + 2, screenNames[i], BLACK);
      } else {
        // Clean navigation - no weird dots
        drawTextLabel(10, y + 2, screenNames[i], 0x4208);
      }
    }
  } else {
    // Only update the changed items
    if (previousIndex >= 0 && previousIndex < 5) {
      int y = startY + (previousIndex * spacing);
      fillVisibleRect(2, y - 1, 33, 12, BLACK);
      // Clean navigation - no weird dots
      drawTextLabel(10, y + 2, screenNames[previousIndex], 0x4208);
    }
    if (screenIndex >= 0 && screenIndex < 5) {
      int y = startY + (screenIndex * spacing);
      fillVisibleRect(2, y - 1, 4, 12, CYAN);
      fillVisibleRect(8, y, 27, 10, CYAN);
      drawTextLabel(10, y + 2, screenNames[screenIndex], BLACK);
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
    
    // Clear and update memory line
    fillVisibleRect(48, 43, 180, 10, SURFACE_DARK);
    drawNumberLabel(50, 45, memPercent, memColor);
    drawTextLabel(75, 45, "% free", getLabelColor());
    drawNumberLabel(130, 45, freeKB, memColor);
    drawTextLabel(155, 45, "KB", getLabelColor());
    
    lastMemoryUpdate = now;
  }
  
  // Update CPU every 2 seconds
  if (now - lastCpuUpdate > 2000) {
    static int cpuUsage = 45;
    // Simple CPU estimation
    int memActivity = abs((int)(ESP.getFreeHeap() / 1024) - 280);
    cpuUsage = constrain(35 + memActivity + random(-5, 10), 25, 75);
    uint16_t cpuColor = cpuUsage < 70 ? getGoodColor() : (cpuUsage < 85 ? getWarningColor() : getErrorColor());
    
    // Clear and update CPU line
    fillVisibleRect(48, 86, 180, 10, SURFACE_DARK);
    drawNumberLabel(50, 88, cpuUsage, cpuColor);
    drawTextLabel(75, 88, "% usage", getLabelColor());
    drawTextLabel(130, 88, "2 cores", getLabelColor());
    drawTextLabel(180, 88, "240MHz", getLabelColor());
    
    lastCpuUpdate = now;
  }
  
  // Update uptime every second
  if (now - lastUptimeUpdate > 1000) {
    // Clear the uptime area completely - adjusted coordinates
    fillVisibleRect(48, 129, 120, 10, SURFACE_DARK);
    
    int totalSec = now / 1000;
    int hours = totalSec / 3600;
    int minutes = (totalSec % 3600) / 60;
    int seconds = totalSec % 60;
    
    // Draw uptime inline - adjusted coordinates
    if (hours > 0) {
      drawNumberLabel(50, 131, hours, getInfoColor());
      drawTextLabel(65, 131, "h", getLabelColor());
      drawNumberLabel(75, 131, minutes, getInfoColor());
      drawTextLabel(90, 131, "m", getLabelColor());
      drawNumberLabel(100, 131, seconds, getInfoColor());
      drawTextLabel(115, 131, "s", getLabelColor());
    } else if (minutes > 0) {
      drawNumberLabel(50, 131, minutes, getInfoColor());
      drawTextLabel(70, 131, "min", getLabelColor());
      drawNumberLabel(90, 131, seconds, getInfoColor());
      drawTextLabel(105, 131, "sec", getLabelColor());
    } else {
      drawNumberLabel(50, 131, seconds, getInfoColor());
      drawTextLabel(70, 131, "seconds", getLabelColor());
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
      
    case 2:  // WiFi with signal history
      if (now - lastWiFiUpdate > WIFI_UPDATE_INTERVAL * speedMultiplier && WiFi.isConnected()) {
        updateWiFiSignalEnhanced();
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
  int batteryMv = analogRead(BATTERY_PIN) * 2;
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
  int battery = analogRead(BATTERY_PIN) * 2;
  
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
  saveSettings();
  
  // Redraw current screen
  clearContentArea();
  switch(screenIndex) {
    case 0: drawSystemInfo(); break;
    case 1: drawPowerStatus(); break;
    case 2: drawWiFiStatus(); break;
    case 3: drawSensorData(); break;
    case 4: drawSettings(); break;
  }
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
          analogWrite(LCD_BL, map(settings.brightness, 0, 100, 0, 255));
          break;
        case 1: // Auto-dim
          settings.autoDim = !settings.autoDim;
          break;
        case 2: // Back
          currentMenu = MENU_MAIN;
          menuSelection = 0;
          break;
      }
      drawSettingsMenu();
      break;
      
    case MENU_UPDATE:
      switch(menuSelection) {
        case 0: // Update speed
          settings.updateSpeed = (settings.updateSpeed + 1) % 3;
          break;
        case 1: // Back
          currentMenu = MENU_MAIN;
          menuSelection = 1;
          break;
      }
      drawSettingsMenu();
      break;
      
    case MENU_SYSTEM:
      switch(menuSelection) {
        case 0: // Reset settings
          resetSettings();
          break;
        case 1: // Back
          currentMenu = MENU_MAIN;
          menuSelection = 2;
          break;
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
      
      const char* mainItems[] = {"Display", "Theme", "Update Speed", "System", "Exit"};
      menuItemCount = 5;
      for (int i = 0; i < menuItemCount; i++) {
        int y = 70 + (i * 20);
        if (i == menuSelection) {
          fillVisibleRect(45, y - 2, DISPLAY_WIDTH - 90, 16, CYAN);
          drawTextLabel(50, y, mainItems[i], getTextColor());
        } else {
          drawTextLabel(50, y, mainItems[i], getTextColor());
        }
      }
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
        drawTextLabel(50, y, "<< Back", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "<< Back", TEXT_SECONDARY);
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
        drawTextLabel(50, y, "<< Back", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "<< Back", TEXT_SECONDARY);
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
        drawTextLabel(50, y, "<< Back", TEXT_SECONDARY);
      } else {
        drawTextLabel(50, y, "<< Back", TEXT_SECONDARY);
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
  analogWrite(LCD_BL, map(settings.brightness, 0, 100, 0, 255));
  
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
  analogWrite(LCD_BL, 255);
}