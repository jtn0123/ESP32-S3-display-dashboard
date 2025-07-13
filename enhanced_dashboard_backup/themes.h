#ifndef THEMES_H
#define THEMES_H

#include <Arduino.h>

// Color Theme Management for T-Display-S3 Dashboard
// Phase 1: Enhanced Color Schemes Implementation
// UPDATED: Correct RGB→BRG color mappings for T-Display-S3
// DARK THEMES ONLY

// VERIFIED COLOR MAPPINGS - RGB→BRG channel rotation
#define COLOR_RED        0x07FF  // Send YELLOW to get RED
#define COLOR_GREEN      0xF81F  // Send CYAN to get GREEN  
#define COLOR_BLUE       0xF8E0  // Send MAGENTA to get BLUE
#define COLOR_YELLOW     0x001F  // Send GREEN to get YELLOW
#define COLOR_CYAN       0xF800  // Send BLUE to get CYAN
#define COLOR_MAGENTA    0x07E0  // Send RED to get MAGENTA
#define COLOR_WHITE      0x0000  // Confirmed working
#define COLOR_BLACK      0xFFFF  // Confirmed working

// Additional color definitions using correct mapping
#define COLOR_GRAY_LIGHT   0x7BCF  // Light gray
#define COLOR_GRAY_MEDIUM  0x528A  // Medium gray  
#define COLOR_GRAY_DARK    0x2945  // Dark gray
#define COLOR_ORANGE       0x039F  // Orange (corrected)
#define COLOR_PURPLE       0xF81F  // Purple (corrected)

// Theme identifiers - STREAMLINED: Orange & Green on Black
enum ThemeType {
  THEME_ORANGE_PRIMARY = 0,
  THEME_GREEN_PRIMARY = 1
};

// Color scheme structure
struct ColorTheme {
  // Primary colors
  uint16_t primary;
  uint16_t secondary;
  uint16_t accent;
  
  // Background colors
  uint16_t background;
  uint16_t surface;
  uint16_t card;
  
  // Text colors
  uint16_t textPrimary;
  uint16_t textSecondary;
  uint16_t textDisabled;
  
  // Status colors
  uint16_t success;
  uint16_t warning;
  uint16_t error;
  uint16_t info;
  
  // UI element colors
  uint16_t border;
  uint16_t shadow;
  uint16_t highlight;
  uint16_t disabled;
};

// Global theme management
extern ColorTheme currentTheme;
extern ThemeType activeThemeType;

// Theme functions
void setTheme(ThemeType theme);
ColorTheme getTheme(ThemeType theme);
void initializeThemes();
int getThemeCount();
String getThemeName(ThemeType theme);

// Quick color access functions
uint16_t getPrimaryColor();
uint16_t getSecondaryColor();
uint16_t getBackgroundColor();
uint16_t getTextColor();
uint16_t getAccentColor();

// Theme definitions - STREAMLINED: Orange & Green on Black
const ColorTheme THEME_DEFINITIONS[] = {
  // THEME_ORANGE_PRIMARY - Orange-focused theme
  {
    .primary = COLOR_ORANGE,     // Orange primary (corrected)
    .secondary = COLOR_GREEN,    // Green secondary (corrected)
    .accent = COLOR_ORANGE,      // Orange accent (corrected)
    .background = COLOR_BLACK,   // Black background (corrected)
    .surface = COLOR_GRAY_DARK,  // Dark gray surface (corrected)
    .card = COLOR_GRAY_MEDIUM,   // Medium gray cards (corrected)
    .textPrimary = COLOR_WHITE,  // White text (corrected)
    .textSecondary = COLOR_GRAY_LIGHT, // Light gray secondary text (corrected)
    .textDisabled = COLOR_GRAY_MEDIUM,  // Medium gray disabled text (corrected)
    .success = COLOR_GREEN,      // Green for success (corrected)
    .warning = COLOR_ORANGE,     // Orange for warnings (corrected)
    .error = COLOR_ORANGE,       // Orange for errors (corrected)
    .info = COLOR_GREEN,         // Green for info (corrected)
    .border = COLOR_GRAY_MEDIUM, // Medium gray borders (corrected)
    .shadow = COLOR_BLACK,       // Black shadows (corrected)
    .highlight = COLOR_ORANGE,   // Orange highlights (corrected)
    .disabled = COLOR_GRAY_DARK  // Dark gray disabled (corrected)
  },
  
  // THEME_GREEN_PRIMARY - Green-focused theme
  {
    .primary = COLOR_GREEN,      // Green primary (corrected)
    .secondary = COLOR_ORANGE,   // Orange secondary (corrected)
    .accent = COLOR_GREEN,       // Green accent (corrected)
    .background = COLOR_BLACK,   // Black background (corrected)
    .surface = COLOR_GRAY_DARK,  // Dark gray surface (corrected)
    .card = COLOR_GRAY_MEDIUM,   // Medium gray cards (corrected)
    .textPrimary = COLOR_WHITE,  // White text (corrected)
    .textSecondary = COLOR_GRAY_LIGHT, // Light gray secondary text (corrected)
    .textDisabled = COLOR_GRAY_MEDIUM,  // Medium gray disabled text (corrected)
    .success = COLOR_GREEN,      // Green for success (corrected)
    .warning = COLOR_ORANGE,     // Orange for warnings (corrected)
    .error = COLOR_ORANGE,       // Orange for errors (corrected)
    .info = COLOR_GREEN,         // Green for info (corrected)
    .border = COLOR_GRAY_MEDIUM, // Medium gray borders (corrected)
    .shadow = COLOR_BLACK,       // Black shadows (corrected)
    .highlight = COLOR_GREEN,    // Green highlights (corrected)
    .disabled = COLOR_GRAY_DARK  // Dark gray disabled (corrected)
  }
};

// Global theme variables
ColorTheme currentTheme;
ThemeType activeThemeType = THEME_ORANGE_PRIMARY;

// Theme functions implementation
void setTheme(ThemeType theme) {
  if (theme >= 0 && theme < 2) {
    activeThemeType = theme;
    currentTheme = THEME_DEFINITIONS[theme];
  }
}

ColorTheme getTheme(ThemeType theme) {
  if (theme >= 0 && theme < 2) {
    return THEME_DEFINITIONS[theme];
  }
  return THEME_DEFINITIONS[0]; // Default fallback
}

void initializeThemes() {
  setTheme(THEME_ORANGE_PRIMARY);
}

int getThemeCount() {
  return 2; // Streamlined to 2 themes
}

String getThemeName(ThemeType theme) {
  switch (theme) {
    case THEME_ORANGE_PRIMARY: return "Orange Focus";
    case THEME_GREEN_PRIMARY: return "Green Focus";
    default: return "Unknown";
  }
}

// Quick color access functions
uint16_t getPrimaryColor() { return currentTheme.primary; }
uint16_t getSecondaryColor() { return currentTheme.secondary; }
uint16_t getBackgroundColor() { return currentTheme.background; }
uint16_t getTextColor() { return currentTheme.textPrimary; }
uint16_t getAccentColor() { return currentTheme.accent; }

#endif