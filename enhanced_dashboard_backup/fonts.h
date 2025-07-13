#ifndef FONTS_H
#define FONTS_H

#include <Arduino.h>

// Text Rendering Functions for T-Display-S3 Dashboard
// Phase 2: Text Rendering Implementation
// Uses verified 300×168 display area and correct RGB→BRG color mappings

// Font size enumeration
enum FontSize {
  FONT_SMALL = 0,
  FONT_MEDIUM = 1,
  FONT_LARGE = 2
};

// Text alignment enumeration
enum TextAlign {
  ALIGN_LEFT = 0,
  ALIGN_CENTER = 1,
  ALIGN_RIGHT = 2
};

// Font configuration structure
struct FontConfig {
  int width;
  int height;
  int spacing;
  const uint8_t* data;
};

// Forward declarations for display functions
extern void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
extern bool isWithinVisibleArea(int x, int y, int w, int h);

// Basic 5x8 font bitmap data (ASCII 32-126)
// Simple bitmap font for small text
const uint8_t FONT_5X8_DATA[] = {
  // Space (32)
  0x00, 0x00, 0x00, 0x00, 0x00,
  // ! (33)
  0x00, 0x00, 0x5F, 0x00, 0x00,
  // " (34)
  0x00, 0x07, 0x00, 0x07, 0x00,
  // # (35)
  0x14, 0x7F, 0x14, 0x7F, 0x14,
  // $ (36)
  0x24, 0x2A, 0x7F, 0x2A, 0x12,
  // % (37)
  0x23, 0x13, 0x08, 0x64, 0x62,
  // & (38)
  0x36, 0x49, 0x55, 0x22, 0x50,
  // ' (39)
  0x00, 0x05, 0x03, 0x00, 0x00,
  // ( (40)
  0x00, 0x1C, 0x22, 0x41, 0x00,
  // ) (41)
  0x00, 0x41, 0x22, 0x1C, 0x00,
  // * (42)
  0x14, 0x08, 0x3E, 0x08, 0x14,
  // + (43)
  0x08, 0x08, 0x3E, 0x08, 0x08,
  // , (44)
  0x00, 0x50, 0x30, 0x00, 0x00,
  // - (45)
  0x08, 0x08, 0x08, 0x08, 0x08,
  // . (46)
  0x00, 0x60, 0x60, 0x00, 0x00,
  // / (47)
  0x20, 0x10, 0x08, 0x04, 0x02,
  // 0 (48)
  0x3E, 0x51, 0x49, 0x45, 0x3E,
  // 1 (49)
  0x00, 0x42, 0x7F, 0x40, 0x00,
  // 2 (50)
  0x42, 0x61, 0x51, 0x49, 0x46,
  // 3 (51)
  0x21, 0x41, 0x45, 0x4B, 0x31,
  // 4 (52)
  0x18, 0x14, 0x12, 0x7F, 0x10,
  // 5 (53)
  0x27, 0x45, 0x45, 0x45, 0x39,
  // 6 (54)
  0x3C, 0x4A, 0x49, 0x49, 0x30,
  // 7 (55)
  0x01, 0x71, 0x09, 0x05, 0x03,
  // 8 (56)
  0x36, 0x49, 0x49, 0x49, 0x36,
  // 9 (57)
  0x06, 0x49, 0x49, 0x29, 0x1E,
  // : (58)
  0x00, 0x36, 0x36, 0x00, 0x00,
  // ; (59)
  0x00, 0x56, 0x36, 0x00, 0x00,
  // < (60)
  0x08, 0x14, 0x22, 0x41, 0x00,
  // = (61)
  0x14, 0x14, 0x14, 0x14, 0x14,
  // > (62)
  0x00, 0x41, 0x22, 0x14, 0x08,
  // ? (63)
  0x02, 0x01, 0x51, 0x09, 0x06,
  // @ (64)
  0x32, 0x49, 0x79, 0x41, 0x3E,
  // A (65)
  0x7E, 0x11, 0x11, 0x11, 0x7E,
  // B (66)
  0x7F, 0x49, 0x49, 0x49, 0x36,
  // C (67)
  0x3E, 0x41, 0x41, 0x41, 0x22,
  // D (68)
  0x7F, 0x41, 0x41, 0x22, 0x1C,
  // E (69)
  0x7F, 0x49, 0x49, 0x49, 0x41,
  // F (70)
  0x7F, 0x09, 0x09, 0x09, 0x01,
  // G (71)
  0x3E, 0x41, 0x49, 0x49, 0x7A,
  // H (72)
  0x7F, 0x08, 0x08, 0x08, 0x7F,
  // I (73)
  0x00, 0x41, 0x7F, 0x41, 0x00,
  // J (74)
  0x20, 0x40, 0x41, 0x3F, 0x01,
  // K (75)
  0x7F, 0x08, 0x14, 0x22, 0x41,
  // L (76)
  0x7F, 0x40, 0x40, 0x40, 0x40,
  // M (77)
  0x7F, 0x02, 0x04, 0x02, 0x7F,
  // N (78)
  0x7F, 0x04, 0x08, 0x10, 0x7F,
  // O (79)
  0x3E, 0x41, 0x41, 0x41, 0x3E,
  // P (80)
  0x7F, 0x09, 0x09, 0x09, 0x06,
  // Q (81)
  0x3E, 0x41, 0x51, 0x21, 0x5E,
  // R (82)
  0x7F, 0x09, 0x19, 0x29, 0x46,
  // S (83)
  0x46, 0x49, 0x49, 0x49, 0x31,
  // T (84)
  0x01, 0x01, 0x7F, 0x01, 0x01,
  // U (85)
  0x3F, 0x40, 0x40, 0x40, 0x3F,
  // V (86)
  0x1F, 0x20, 0x40, 0x20, 0x1F,
  // W (87)
  0x3F, 0x40, 0x38, 0x40, 0x3F,
  // X (88)
  0x63, 0x14, 0x08, 0x14, 0x63,
  // Y (89)
  0x07, 0x08, 0x70, 0x08, 0x07,
  // Z (90)
  0x61, 0x51, 0x49, 0x45, 0x43,
  // [ (91)
  0x00, 0x7F, 0x41, 0x41, 0x00,
  // \ (92)
  0x02, 0x04, 0x08, 0x10, 0x20,
  // ] (93)
  0x00, 0x41, 0x41, 0x7F, 0x00,
  // ^ (94)
  0x04, 0x02, 0x01, 0x02, 0x04,
  // _ (95)
  0x40, 0x40, 0x40, 0x40, 0x40,
  // ` (96)
  0x00, 0x01, 0x02, 0x04, 0x00,
  // a (97)
  0x20, 0x54, 0x54, 0x54, 0x78,
  // b (98)
  0x7F, 0x48, 0x44, 0x44, 0x38,
  // c (99)
  0x38, 0x44, 0x44, 0x44, 0x20,
  // d (100)
  0x38, 0x44, 0x44, 0x48, 0x7F,
  // e (101)
  0x38, 0x54, 0x54, 0x54, 0x18,
  // f (102)
  0x08, 0x7E, 0x09, 0x01, 0x02,
  // g (103)
  0x18, 0xA4, 0xA4, 0xA4, 0x7C,
  // h (104)
  0x7F, 0x08, 0x04, 0x04, 0x78,
  // i (105)
  0x00, 0x44, 0x7D, 0x40, 0x00,
  // j (106)
  0x40, 0x80, 0x84, 0x7D, 0x00,
  // k (107)
  0x7F, 0x10, 0x28, 0x44, 0x00,
  // l (108)
  0x00, 0x41, 0x7F, 0x40, 0x00,
  // m (109)
  0x7C, 0x04, 0x18, 0x04, 0x78,
  // n (110)
  0x7C, 0x08, 0x04, 0x04, 0x78,
  // o (111)
  0x38, 0x44, 0x44, 0x44, 0x38,
  // p (112)
  0xFC, 0x24, 0x24, 0x24, 0x18,
  // q (113)
  0x18, 0x24, 0x24, 0x18, 0xFC,
  // r (114)
  0x7C, 0x08, 0x04, 0x04, 0x08,
  // s (115)
  0x48, 0x54, 0x54, 0x54, 0x20,
  // t (116)
  0x04, 0x3F, 0x44, 0x40, 0x20,
  // u (117)
  0x3C, 0x40, 0x40, 0x20, 0x7C,
  // v (118)
  0x1C, 0x20, 0x40, 0x20, 0x1C,
  // w (119)
  0x3C, 0x40, 0x30, 0x40, 0x3C,
  // x (120)
  0x44, 0x28, 0x10, 0x28, 0x44,
  // y (121)
  0x1C, 0xA0, 0xA0, 0xA0, 0x7C,
  // z (122)
  0x44, 0x64, 0x54, 0x4C, 0x44,
  // { (123)
  0x00, 0x08, 0x36, 0x41, 0x00,
  // | (124)
  0x00, 0x00, 0x7F, 0x00, 0x00,
  // } (125)
  0x00, 0x41, 0x36, 0x08, 0x00,
  // ~ (126)
  0x10, 0x08, 0x08, 0x10, 0x08
};

// Font configurations
const FontConfig FONT_CONFIGS[] = {
  // FONT_SMALL (5x8)
  { .width = 5, .height = 8, .spacing = 1, .data = FONT_5X8_DATA },
  // FONT_MEDIUM (10x16 - scaled 2x)
  { .width = 10, .height = 16, .spacing = 2, .data = FONT_5X8_DATA },
  // FONT_LARGE (15x24 - scaled 3x)
  { .width = 15, .height = 24, .spacing = 3, .data = FONT_5X8_DATA }
};

// Text rendering functions
void drawChar(int x, int y, char c, uint16_t color, FontSize size);
void drawString(int x, int y, String text, uint16_t color, FontSize size);
void drawStringAligned(int x, int y, int maxWidth, String text, uint16_t color, FontSize size, TextAlign align);
void drawStringWrapped(int x, int y, int maxWidth, String text, uint16_t color, FontSize size);
int getStringWidth(String text, FontSize size);
int getStringHeight(FontSize size);
int getCharWidth(FontSize size);

// Text utility functions
void drawTextBox(int x, int y, int width, int height, String text, uint16_t textColor, uint16_t bgColor, FontSize size, TextAlign align);
void drawLabel(int x, int y, String label, String value, uint16_t labelColor, uint16_t valueColor, FontSize size);

// Enhanced text readability functions
void drawCharWithOutline(int x, int y, char c, uint16_t textColor, uint16_t outlineColor, FontSize size, int outlineWidth = 1);
void drawStringWithOutline(int x, int y, String text, uint16_t textColor, uint16_t outlineColor, FontSize size, int outlineWidth = 1);
void drawStringWithShadow(int x, int y, String text, uint16_t textColor, uint16_t shadowColor, FontSize size, int shadowOffset = 1);

// Background style enumeration
enum BackgroundStyle {
  BG_SOLID,
  BG_ROUNDED,
  BG_GRADIENT,
  BG_TRANSPARENT_OVERLAY
};

void drawTextBoxEnhanced(int x, int y, int width, int height, String text, uint16_t textColor, uint16_t bgColor, FontSize size, TextAlign align, BackgroundStyle style = BG_SOLID, int padding = 2);

// Smart contrast functions
uint16_t getOptimalTextColor(uint16_t backgroundColor);
uint16_t getContrastColor(uint16_t color);
void drawReadableText(int x, int y, String text, uint16_t preferredColor, uint16_t backgroundColor, FontSize size);
void drawStatusText(int x, int y, String text, FontSize size, bool isImportant = false);

// Implementation
void drawChar(int x, int y, char c, uint16_t color, FontSize size) {
  if (c < 32 || c > 126) return; // Support ASCII 32-126 (space to ~)
  
  const FontConfig& font = FONT_CONFIGS[size];
  int charIndex = c - 32;
  int scale = size + 1; // 1x, 2x, 3x scaling
  
  // Get character bitmap data (5 bytes per character)
  const uint8_t* charData = &font.data[charIndex * 5];
  
  // Draw each column of the character
  for (int col = 0; col < 5; col++) {
    uint8_t columnData = charData[col];
    
    // Draw each pixel in the column
    for (int row = 0; row < 8; row++) {
      if (columnData & (1 << row)) {
        // Draw scaled pixel
        for (int sx = 0; sx < scale; sx++) {
          for (int sy = 0; sy < scale; sy++) {
            int pixelX = x + col * scale + sx;
            int pixelY = y + row * scale + sy;
            
            // Use single pixel drawing via fillVisibleRect
            if (isWithinVisibleArea(pixelX, pixelY, 1, 1)) {
              fillVisibleRect(pixelX, pixelY, 1, 1, color);
            }
          }
        }
      }
    }
  }
}

void drawString(int x, int y, String text, uint16_t color, FontSize size) {
  const FontConfig& font = FONT_CONFIGS[size];
  int currentX = x;
  
  for (int i = 0; i < text.length(); i++) {
    drawChar(currentX, y, text.charAt(i), color, size);
    currentX += font.width + font.spacing;
  }
}

void drawStringAligned(int x, int y, int maxWidth, String text, uint16_t color, FontSize size, TextAlign align) {
  int textWidth = getStringWidth(text, size);
  int startX = x;
  
  switch (align) {
    case ALIGN_CENTER:
      startX = x + (maxWidth - textWidth) / 2;
      break;
    case ALIGN_RIGHT:
      startX = x + maxWidth - textWidth;
      break;
    case ALIGN_LEFT:
    default:
      startX = x;
      break;
  }
  
  drawString(startX, y, text, color, size);
}

void drawStringWrapped(int x, int y, int maxWidth, String text, uint16_t color, FontSize size) {
  const FontConfig& font = FONT_CONFIGS[size];
  int currentX = x;
  int currentY = y;
  int lineHeight = font.height + 2;
  
  String word = "";
  for (int i = 0; i <= text.length(); i++) {
    char c = (i < text.length()) ? text.charAt(i) : ' ';
    
    if (c == ' ' || i == text.length()) {
      int wordWidth = getStringWidth(word, size);
      
      // Check if word fits on current line
      if (currentX + wordWidth > x + maxWidth && currentX > x) {
        // Move to next line
        currentX = x;
        currentY += lineHeight;
      }
      
      // Draw the word
      if (word.length() > 0) {
        drawString(currentX, currentY, word, color, size);
        currentX += wordWidth + (font.width + font.spacing); // Add space width
      }
      
      word = "";
    } else {
      word += c;
    }
  }
}

int getStringWidth(String text, FontSize size) {
  const FontConfig& font = FONT_CONFIGS[size];
  return text.length() * (font.width + font.spacing) - font.spacing;
}

int getStringHeight(FontSize size) {
  return FONT_CONFIGS[size].height;
}

int getCharWidth(FontSize size) {
  return FONT_CONFIGS[size].width;
}

void drawTextBox(int x, int y, int width, int height, String text, uint16_t textColor, uint16_t bgColor, FontSize size, TextAlign align) {
  // Draw background
  fillVisibleRect(x, y, width, height, bgColor);
  
  // Calculate text position
  int textY = y + (height - getStringHeight(size)) / 2;
  
  // Draw text with alignment
  drawStringAligned(x + 2, textY, width - 4, text, textColor, size, align);
}

void drawLabel(int x, int y, String label, String value, uint16_t labelColor, uint16_t valueColor, FontSize size) {
  // Draw label
  drawString(x, y, label + ": ", labelColor, size);
  
  // Draw value next to label
  int labelWidth = getStringWidth(label + ": ", size);
  drawString(x + labelWidth, y, value, valueColor, size);
}

// Enhanced text readability implementations
void drawCharWithOutline(int x, int y, char c, uint16_t textColor, uint16_t outlineColor, FontSize size, int outlineWidth) {
  // Draw outline by rendering character in multiple positions
  for (int ox = -outlineWidth; ox <= outlineWidth; ox++) {
    for (int oy = -outlineWidth; oy <= outlineWidth; oy++) {
      if (ox != 0 || oy != 0) {
        drawChar(x + ox, y + oy, c, outlineColor, size);
      }
    }
  }
  // Draw main character on top
  drawChar(x, y, c, textColor, size);
}

void drawStringWithOutline(int x, int y, String text, uint16_t textColor, uint16_t outlineColor, FontSize size, int outlineWidth) {
  const FontConfig& font = FONT_CONFIGS[size];
  int currentX = x;
  
  for (int i = 0; i < text.length(); i++) {
    drawCharWithOutline(currentX, y, text.charAt(i), textColor, outlineColor, size, outlineWidth);
    currentX += font.width + font.spacing;
  }
}

void drawStringWithShadow(int x, int y, String text, uint16_t textColor, uint16_t shadowColor, FontSize size, int shadowOffset) {
  // Draw shadow first (offset)
  drawString(x + shadowOffset, y + shadowOffset, text, shadowColor, size);
  // Draw main text on top
  drawString(x, y, text, textColor, size);
}

void drawTextBoxEnhanced(int x, int y, int width, int height, String text, uint16_t textColor, uint16_t bgColor, FontSize size, TextAlign align, BackgroundStyle style, int padding) {
  // Forward declaration of interpolateColor function
  extern uint16_t interpolateColor(uint16_t color1, uint16_t color2, float ratio);
  extern void fillRoundRect(int x, int y, int w, int h, int radius, uint16_t color);
  extern void fillGradientV(int x, int y, int w, int h, uint16_t color1, uint16_t color2);
  
  switch (style) {
    case BG_SOLID:
      fillVisibleRect(x, y, width, height, bgColor);
      break;
    case BG_ROUNDED:
      fillRoundRect(x, y, width, height, 4, bgColor);
      break;
    case BG_GRADIENT:
      // Create subtle gradient for depth
      fillGradientV(x, y, width, height, bgColor, interpolateColor(bgColor, 0xFFFF, 0.1));
      break;
    case BG_TRANSPARENT_OVERLAY:
      // Semi-transparent overlay effect
      uint16_t overlayColor = interpolateColor(bgColor, 0xFFFF, 0.7);
      fillVisibleRect(x, y, width, height, overlayColor);
      break;
  }
  
  // Draw text with appropriate positioning
  int textY = y + (height - getStringHeight(size)) / 2;
  drawStringAligned(x + padding, textY, width - (padding * 2), text, textColor, size, align);
}

uint16_t getOptimalTextColor(uint16_t backgroundColor) {
  // Forward declaration
  extern void rgb565ToRgb(uint16_t color, uint8_t &r, uint8_t &g, uint8_t &b);
  
  // Simple luminance-based contrast selection
  uint8_t r, g, b;
  rgb565ToRgb(backgroundColor, r, g, b);
  
  // Calculate luminance (simplified)
  int luminance = (r * 299 + g * 587 + b * 114) / 1000;
  
  // Return white for dark backgrounds, black for light backgrounds
  return (luminance < 128) ? 0x0000 : 0xFFFF; // COLOR_WHITE : COLOR_BLACK
}

uint16_t getContrastColor(uint16_t color) {
  // Return complementary color for maximum contrast
  return (color == 0x0000) ? 0xFFFF : 0x0000; // (color == COLOR_WHITE) ? COLOR_BLACK : COLOR_WHITE
}

void drawReadableText(int x, int y, String text, uint16_t preferredColor, uint16_t backgroundColor, FontSize size) {
  uint16_t textColor = getOptimalTextColor(backgroundColor);
  uint16_t outlineColor = getContrastColor(textColor);
  
  // Always use outline for maximum readability
  drawStringWithOutline(x, y, text, textColor, outlineColor, size);
}

void drawStatusText(int x, int y, String text, FontSize size, bool isImportant) {
  // Forward declaration
  extern uint16_t getPrimaryColor();
  extern uint16_t getTextColor();
  
  if (isImportant) {
    // Important text gets enhanced background
    int textWidth = getStringWidth(text, size);
    int textHeight = getStringHeight(size);
    drawTextBoxEnhanced(x - 2, y - 1, textWidth + 4, textHeight + 2, text, 0x0000, getPrimaryColor(), size, ALIGN_LEFT, BG_ROUNDED);
  } else {
    // Regular text gets outline for readability
    drawStringWithOutline(x, y, text, getTextColor(), 0xFFFF, size);
  }
}

#endif