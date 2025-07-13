#ifndef SCREENS_H
#define SCREENS_H

#include <Arduino.h>

// Screen Management System for T-Display-S3 Dashboard
// Phase 3C: Advanced UI & Navigation

// Screen definitions
enum ScreenType {
  SCREEN_DASHBOARD = 0,    // Main dashboard/launcher
  SCREEN_NETWORK = 1,      // Network status and WiFi info
  SCREEN_SYSTEM = 2,       // System monitoring (CPU, memory, etc.)
  SCREEN_SENSORS = 3,      // Sensor data and readings
  SCREEN_SETTINGS = 4,     // Settings and configuration
  SCREEN_ABOUT = 5         // About/info screen
};

#define TOTAL_SCREENS 6

// Screen metadata structure
struct ScreenInfo {
  ScreenType type;
  String name;
  String shortName;
  String description;
  bool enabled;
  unsigned long lastUpdate;
  bool requiresRefresh;
};

// Settings structure
struct DashboardSettings {
  // Display settings
  int brightness;
  bool autoTheme;
  int themeIndex;
  int screenTimeout;
  
  // Navigation settings
  bool swipeEnabled;
  bool autoAdvance;
  int autoAdvanceDelay;
  
  // Network settings
  bool wifiEnabled;
  bool webServerEnabled;
  bool otaEnabled;
  
  // Touch settings
  int touchSensitivity;
  bool touchFeedback;
  bool touchSounds;
  
  // System settings
  bool serialDebug;
  int logLevel;
  bool showFPS;
};

// Forward declarations
extern ScreenInfo screens[TOTAL_SCREENS];
extern DashboardSettings settings;
extern int currentScreenIndex;
extern unsigned long lastScreenUpdate;

// Screen management functions
void initScreenSystem();
void switchToScreen(ScreenType screen);
void switchToScreen(int screenIndex);
void nextScreen();
void previousScreen();
void refreshCurrentScreen();
void updateScreenSystem();

// Settings management
void initSettings();
void saveSettings();
void loadSettings();
void resetSettings();

// Screen drawing functions
void drawCurrentScreen();
void drawDashboardScreen();
void drawNetworkScreen();
void drawSystemScreen();
void drawSensorScreen();
void drawSettingsScreen();
void drawAboutScreen();

// Settings screen helpers
void drawSettingsCategory(int x, int y, String title, int &yOffset);
void drawSettingToggle(int x, int y, String label, bool value, int index, int &yOffset);
void drawSettingSlider(int x, int y, String label, int value, int min, int max, int index, int &yOffset);
void drawSettingOption(int x, int y, String label, String value, int index, int &yOffset);

// Touch handling for settings
void handleSettingsTouch(TouchEvent event);
bool isPointInSettingsItem(int x, int y, int itemIndex);

// Screen transition effects
void fadeTransition(ScreenType fromScreen, ScreenType toScreen);
void slideTransition(ScreenType fromScreen, ScreenType toScreen, bool leftToRight);

// Implementation
ScreenInfo screens[TOTAL_SCREENS];
DashboardSettings settings;
int currentScreenIndex = 0;
unsigned long lastScreenUpdate = 0;

void initScreenSystem() {
  Serial.println("=== Initializing Screen System ===");
  
  // Initialize screen metadata
  screens[SCREEN_DASHBOARD] = {SCREEN_DASHBOARD, "Dashboard", "Home", "Main launcher screen", true, 0, true};
  screens[SCREEN_NETWORK] = {SCREEN_NETWORK, "Network", "WiFi", "Network status and connectivity", true, 0, true};
  screens[SCREEN_SYSTEM] = {SCREEN_SYSTEM, "System", "Sys", "System monitoring and stats", true, 0, true};
  screens[SCREEN_SENSORS] = {SCREEN_SENSORS, "Sensors", "Data", "Sensor readings and data", true, 0, true};
  screens[SCREEN_SETTINGS] = {SCREEN_SETTINGS, "Settings", "Set", "Configuration and preferences", true, 0, true};
  screens[SCREEN_ABOUT] = {SCREEN_ABOUT, "About", "Info", "Device info and credits", true, 0, true};
  
  // Initialize settings
  initSettings();
  
  Serial.println("Screen system initialized with " + String(TOTAL_SCREENS) + " screens");
  for (int i = 0; i < TOTAL_SCREENS; i++) {
    if (screens[i].enabled) {
      Serial.println("  " + String(i) + ": " + screens[i].name + " (" + screens[i].shortName + ")");
    }
  }
}

void initSettings() {
  // Initialize default settings
  settings = {
    // Display settings
    .brightness = 80,
    .autoTheme = true,
    .themeIndex = 0,
    .screenTimeout = 30,
    
    // Navigation settings
    .swipeEnabled = true,
    .autoAdvance = true,
    .autoAdvanceDelay = 6,
    
    // Network settings
    .wifiEnabled = true,
    .webServerEnabled = true,
    .otaEnabled = true,
    
    // Touch settings
    .touchSensitivity = 40,
    .touchFeedback = true,
    .touchSounds = false,
    
    // System settings
    .serialDebug = true,
    .logLevel = 1,
    .showFPS = false
  };
  
  // Try to load saved settings
  loadSettings();
}

void loadSettings() {
  // Load settings from preferences
  extern Preferences preferences;
  
  if (preferences.begin("dashboard", true)) {
    settings.brightness = preferences.getInt("brightness", settings.brightness);
    settings.autoTheme = preferences.getBool("autoTheme", settings.autoTheme);
    settings.themeIndex = preferences.getInt("themeIndex", settings.themeIndex);
    settings.swipeEnabled = preferences.getBool("swipeEnabled", settings.swipeEnabled);
    settings.autoAdvance = preferences.getBool("autoAdvance", settings.autoAdvance);
    settings.autoAdvanceDelay = preferences.getInt("autoAdvanceDelay", settings.autoAdvanceDelay);
    settings.touchSensitivity = preferences.getInt("touchSensitivity", settings.touchSensitivity);
    settings.touchFeedback = preferences.getBool("touchFeedback", settings.touchFeedback);
    
    preferences.end();
    Serial.println("Settings loaded from preferences");
  }
}

void saveSettings() {
  // Save settings to preferences
  extern Preferences preferences;
  
  if (preferences.begin("dashboard", false)) {
    preferences.putInt("brightness", settings.brightness);
    preferences.putBool("autoTheme", settings.autoTheme);
    preferences.putInt("themeIndex", settings.themeIndex);
    preferences.putBool("swipeEnabled", settings.swipeEnabled);
    preferences.putBool("autoAdvance", settings.autoAdvance);
    preferences.putInt("autoAdvanceDelay", settings.autoAdvanceDelay);
    preferences.putInt("touchSensitivity", settings.touchSensitivity);
    preferences.putBool("touchFeedback", settings.touchFeedback);
    
    preferences.end();
    Serial.println("Settings saved to preferences");
  }
}

void switchToScreen(ScreenType screen) {
  switchToScreen((int)screen);
}

void switchToScreen(int screenIndex) {
  if (screenIndex >= 0 && screenIndex < TOTAL_SCREENS && screens[screenIndex].enabled) {
    int oldScreen = currentScreenIndex;
    currentScreenIndex = screenIndex;
    
    Serial.print("Switching to screen: ");
    Serial.print(screenIndex);
    Serial.print(" (");
    Serial.print(screens[screenIndex].name);
    Serial.println(")");
    
    // Mark screen for refresh
    screens[currentScreenIndex].requiresRefresh = true;
    lastScreenUpdate = millis();
    
    // Draw the new screen
    drawCurrentScreen();
  }
}

void nextScreen() {
  int next = currentScreenIndex;
  do {
    next = (next + 1) % TOTAL_SCREENS;
  } while (!screens[next].enabled && next != currentScreenIndex);
  
  switchToScreen(next);
}

void previousScreen() {
  int prev = currentScreenIndex;
  do {
    prev = (prev - 1 + TOTAL_SCREENS) % TOTAL_SCREENS;
  } while (!screens[prev].enabled && prev != currentScreenIndex);
  
  switchToScreen(prev);
}

void refreshCurrentScreen() {
  screens[currentScreenIndex].requiresRefresh = true;
  drawCurrentScreen();
}

void updateScreenSystem() {
  unsigned long now = millis();
  
  // Check if current screen needs refresh
  if (screens[currentScreenIndex].requiresRefresh || 
      (now - lastScreenUpdate > 1000)) { // Refresh every second
    
    screens[currentScreenIndex].lastUpdate = now;
    screens[currentScreenIndex].requiresRefresh = false;
    lastScreenUpdate = now;
    
    // Only redraw if something actually changed
    // (This would be expanded with actual change detection)
  }
}

void drawCurrentScreen() {
  // Clear screen
  extern void fillScreen(uint16_t color);
  extern uint16_t getBackgroundColor();
  fillScreen(getBackgroundColor());
  
  // Draw the current screen
  switch ((ScreenType)currentScreenIndex) {
    case SCREEN_DASHBOARD:
      drawDashboardScreen();
      break;
    case SCREEN_NETWORK:
      drawNetworkScreen();
      break;
    case SCREEN_SYSTEM:
      drawSystemScreen();
      break;
    case SCREEN_SENSORS:
      drawSensorScreen();
      break;
    case SCREEN_SETTINGS:
      drawSettingsScreen();
      break;
    case SCREEN_ABOUT:
      drawAboutScreen();
      break;
  }
  
  // Always draw status bar
  extern void drawStatusBarNew(int screen);
  drawStatusBarNew(currentScreenIndex);
}

void drawDashboardScreen() {
  // Call the existing launcher screen function
  extern void drawLauncherScreen();
  drawLauncherScreen();
}

// Network screen is drawn directly by calling drawNetworkScreen() from main file

void drawSystemScreen() {
  // Call the existing system monitoring screen
  extern void drawSystemMonitoringScreen();
  drawSystemMonitoringScreen();
}

void drawSensorScreen() {
  // Forward declaration for sensor functions
  extern void drawSensorScreenDetailed();
  drawSensorScreenDetailed();
}

void drawSettingsScreen() {
  extern void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
  extern void drawString(int x, int y, String text, uint16_t color, FontSize size);
  extern uint16_t getPrimaryColor();
  extern uint16_t getTextColor();
  extern ColorTheme currentTheme;
  
  // Header
  fillVisibleRect(0, 0, 300, 30, getPrimaryColor());
  drawString(105, 8, "Settings", getTextColor(), FONT_MEDIUM);
  
  int yPos = 40;
  
  // Display Settings
  drawString(15, yPos, "Display:", getTextColor(), FONT_SMALL);
  yPos += 15;
  
  // Brightness setting
  String brightnessText = "Brightness: " + String(settings.brightness) + "%";
  drawString(20, yPos, brightnessText, currentTheme.info, FONT_SMALL);
  
  // Brightness bar
  fillVisibleRect(150, yPos, 100, 8, currentTheme.surface);
  int brightnessWidth = settings.brightness;
  if (brightnessWidth > 0) {
    fillVisibleRect(150, yPos, brightnessWidth, 8, currentTheme.warning);
  }
  yPos += 20;
  
  // Auto theme toggle
  String autoThemeText = "Auto Theme: " + String(settings.autoTheme ? "ON" : "OFF");
  uint16_t autoThemeColor = settings.autoTheme ? currentTheme.success : currentTheme.disabled;
  drawString(20, yPos, autoThemeText, autoThemeColor, FONT_SMALL);
  yPos += 20;
  
  // Navigation Settings
  drawString(15, yPos, "Navigation:", getTextColor(), FONT_SMALL);
  yPos += 15;
  
  // Swipe enabled
  String swipeText = "Swipe: " + String(settings.swipeEnabled ? "ON" : "OFF");
  uint16_t swipeColor = settings.swipeEnabled ? currentTheme.success : currentTheme.disabled;
  drawString(20, yPos, swipeText, swipeColor, FONT_SMALL);
  yPos += 15;
  
  // Auto advance
  String autoAdvanceText = "Auto Advance: " + String(settings.autoAdvance ? "ON" : "OFF");
  uint16_t autoAdvanceColor = settings.autoAdvance ? currentTheme.success : currentTheme.disabled;
  drawString(20, yPos, autoAdvanceText, autoAdvanceColor, FONT_SMALL);
  yPos += 20;
  
  // Touch Settings
  drawString(15, yPos, "Touch:", getTextColor(), FONT_SMALL);
  yPos += 15;
  
  // Touch sensitivity
  String sensitivityText = "Sensitivity: " + String(settings.touchSensitivity);
  drawString(20, yPos, sensitivityText, currentTheme.info, FONT_SMALL);
  yPos += 15;
  
  // Touch feedback
  String feedbackText = "Feedback: " + String(settings.touchFeedback ? "ON" : "OFF");
  uint16_t feedbackColor = settings.touchFeedback ? currentTheme.success : currentTheme.disabled;
  drawString(20, yPos, feedbackText, feedbackColor, FONT_SMALL);
  
  // Touch instructions
  drawString(15, 145, "Touch header to save", currentTheme.textSecondary, FONT_SMALL);
}

void drawAboutScreen() {
  extern void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
  extern void drawString(int x, int y, String text, uint16_t color, FontSize size);
  extern uint16_t getPrimaryColor();
  extern uint16_t getTextColor();
  extern ColorTheme currentTheme;
  extern String getUptime();
  
  // Header
  fillVisibleRect(0, 0, 300, 30, getPrimaryColor());
  drawString(115, 8, "About", getTextColor(), FONT_MEDIUM);
  
  int yPos = 40;
  
  // Device info
  drawString(15, yPos, "T-Display S3 Dashboard", getTextColor(), FONT_SMALL);
  yPos += 15;
  drawString(15, yPos, "Version: 3.0 Phase 3C", currentTheme.info, FONT_SMALL);
  yPos += 15;
  drawString(15, yPos, "Build: " + String(__DATE__), currentTheme.textSecondary, FONT_SMALL);
  yPos += 20;
  
  // Hardware info
  drawString(15, yPos, "Hardware:", getTextColor(), FONT_SMALL);
  yPos += 15;
  drawString(15, yPos, "ESP32-S3 @ 240MHz", currentTheme.info, FONT_SMALL);
  yPos += 15;
  drawString(15, yPos, "8MB PSRAM + 16MB Flash", currentTheme.info, FONT_SMALL);
  yPos += 15;
  drawString(15, yPos, "1.9\" ST7789V Display", currentTheme.info, FONT_SMALL);
  yPos += 20;
  
  // Runtime info
  drawString(15, yPos, "Runtime:", getTextColor(), FONT_SMALL);
  yPos += 15;
  drawString(15, yPos, "Uptime: " + getUptime(), currentTheme.success, FONT_SMALL);
  yPos += 15;
  
  int freeHeap = ESP.getFreeHeap();
  int totalHeap = ESP.getHeapSize();
  int usedHeap = totalHeap - freeHeap;
  int heapPercent = (usedHeap * 100) / totalHeap;
  
  drawString(15, yPos, "Memory: " + String(heapPercent) + "% used", currentTheme.warning, FONT_SMALL);
  
  // Features indicator
  drawString(15, 135, "WiFi + Touch + Web + OTA", currentTheme.success, FONT_SMALL);
  
  // Test instructions
  drawString(15, 150, "Long press content for test", currentTheme.textSecondary, FONT_SMALL);
}

// New status bar that works with the screen system
void drawStatusBarNew(int screen) {
  extern void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
  extern void drawString(int x, int y, String text, uint16_t color, FontSize size);
  extern ColorTheme currentTheme;
  extern int themeCounter;
  
  // Simple status bar at bottom
  fillVisibleRect(0, 155, 300, 13, currentTheme.surface);
  
  // Touch zone indicators - left and right navigation
  drawString(5, 158, "◀", currentTheme.textSecondary, FONT_SMALL);   // Left nav hint
  drawString(285, 158, "▶", currentTheme.textSecondary, FONT_SMALL); // Right nav hint
  
  // Screen indicator dots - 6 screens
  extern uint16_t getPrimaryColor();
  for (int i = 0; i < TOTAL_SCREENS; i++) {
    if (screens[i].enabled) {
      uint16_t dotColor = (i == screen) ? getPrimaryColor() : currentTheme.disabled;
      fillVisibleRect(120 + i * 6, 160, 3, 3, dotColor);
    }
  }
  
  // Touch hint for theme toggle + theme indicator
  String themeName = (themeCounter == 0) ? "Orange" : "Green";
  drawString(180, 158, "↑" + themeName, getTextColor(), FONT_SMALL);
}

#endif