#ifndef ICONS_H
#define ICONS_H

#include <Arduino.h>

// Forward declarations
extern void drawPixel(int x, int y, uint16_t color);
extern void fillRect(int x, int y, int w, int h, uint16_t color);
extern void drawLine(int x0, int y0, int x1, int y1, uint16_t color);

// Icon Library for T-Display-S3 Dashboard
// Phase 1: Icons and Symbols Implementation

// Icon sizes
#define ICON_SMALL 12
#define ICON_MEDIUM 16
#define ICON_LARGE 24

// WiFi signal strength icons
void drawWiFiIcon(int x, int y, int strength, uint16_t color, int size = ICON_MEDIUM);

// Battery status icons
void drawBatteryIcon(int x, int y, int level, uint16_t color, int size = ICON_MEDIUM);

// System status indicators
void drawCheckIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);
void drawCrossIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);
void drawWarningIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);
void drawInfoIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);

// Weather condition icons
void drawSunIcon(int x, int y, uint16_t color, int size = ICON_LARGE);
void drawCloudIcon(int x, int y, uint16_t color, int size = ICON_LARGE);
void drawRainIcon(int x, int y, uint16_t color, int size = ICON_LARGE);

// Navigation and control icons
void drawHomeIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);
void drawSettingsIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);
void drawMenuIcon(int x, int y, uint16_t color, int size = ICON_MEDIUM);
void drawArrowIcon(int x, int y, int direction, uint16_t color, int size = ICON_MEDIUM);

// Implementation

void drawWiFiIcon(int x, int y, int strength, uint16_t color, int size) {
  // WiFi strength: 0-3 bars
  int barWidth = size / 6;
  int barSpacing = size / 8;
  
  for (int i = 0; i < 4; i++) {
    if (i < strength) {
      int barHeight = (i + 1) * size / 4;
      int barX = x + i * (barWidth + barSpacing);
      int barY = y + size - barHeight;
      fillRect(barX, barY, barWidth, barHeight, color);
    }
  }
}

void drawBatteryIcon(int x, int y, int level, uint16_t color, int size) {
  // Battery outline
  int bodyWidth = size * 3 / 4;
  int bodyHeight = size / 2;
  int tipWidth = size / 8;
  int tipHeight = size / 4;
  
  // Battery body outline
  drawLine(x, y, x + bodyWidth, y, color);                    // Top
  drawLine(x, y + bodyHeight, x + bodyWidth, y + bodyHeight, color); // Bottom
  drawLine(x, y, x, y + bodyHeight, color);                   // Left
  drawLine(x + bodyWidth, y, x + bodyWidth, y + bodyHeight, color);   // Right
  
  // Battery tip
  fillRect(x + bodyWidth, y + bodyHeight/4, tipWidth, tipHeight, color);
  
  // Fill level (0-100%)
  int fillWidth = (level * (bodyWidth - 2)) / 100;
  if (fillWidth > 0) {
    fillRect(x + 1, y + 1, fillWidth, bodyHeight - 2, color);
  }
}

void drawCheckIcon(int x, int y, uint16_t color, int size) {
  // Draw checkmark
  int midX = x + size / 3;
  int midY = y + size * 2 / 3;
  
  drawLine(x, y + size / 2, midX, midY, color);
  drawLine(midX, midY, x + size, y, color);
}

void drawCrossIcon(int x, int y, uint16_t color, int size) {
  // Draw X
  drawLine(x, y, x + size, y + size, color);
  drawLine(x + size, y, x, y + size, color);
}

void drawWarningIcon(int x, int y, uint16_t color, int size) {
  // Triangle outline
  int centerX = x + size / 2;
  drawLine(centerX, y, x, y + size, color);         // Left side
  drawLine(centerX, y, x + size, y + size, color);  // Right side
  drawLine(x, y + size, x + size, y + size, color); // Bottom
  
  // Exclamation mark
  int lineHeight = size / 2;
  drawLine(centerX, y + size/4, centerX, y + size/4 + lineHeight, color);
  drawPixel(centerX, y + size - size/6, color);
}

void drawInfoIcon(int x, int y, uint16_t color, int size) {
  // Circle outline (simplified as square for pixel display)
  drawLine(x, y, x + size, y, color);
  drawLine(x, y + size, x + size, y + size, color);
  drawLine(x, y, x, y + size, color);
  drawLine(x + size, y, x + size, y + size, color);
  
  // "i" symbol
  int centerX = x + size / 2;
  drawPixel(centerX, y + size/4, color);              // Dot
  drawLine(centerX, y + size/2, centerX, y + size*3/4, color); // Line
}

void drawSunIcon(int x, int y, uint16_t color, int size) {
  int centerX = x + size / 2;
  int centerY = y + size / 2;
  int radius = size / 4;
  
  // Sun center (filled circle approximation)
  for (int dy = -radius; dy <= radius; dy++) {
    for (int dx = -radius; dx <= radius; dx++) {
      if (dx*dx + dy*dy <= radius*radius) {
        drawPixel(centerX + dx, centerY + dy, color);
      }
    }
  }
  
  // Sun rays
  int rayLength = size / 6;
  for (int i = 0; i < 8; i++) {
    float angle = i * PI / 4;
    int rayStartX = centerX + (radius + 2) * cos(angle);
    int rayStartY = centerY + (radius + 2) * sin(angle);
    int rayEndX = centerX + (radius + 2 + rayLength) * cos(angle);
    int rayEndY = centerY + (radius + 2 + rayLength) * sin(angle);
    drawLine(rayStartX, rayStartY, rayEndX, rayEndY, color);
  }
}

void drawCloudIcon(int x, int y, uint16_t color, int size) {
  // Simplified cloud shape using circles
  int baseY = y + size * 3 / 4;
  
  // Main cloud body (rectangle base)
  fillRect(x + size/4, baseY - size/3, size/2, size/3, color);
  
  // Cloud bumps (circles approximated as filled areas)
  int bump1X = x + size/4;
  int bump1Y = baseY - size/2;
  fillRect(bump1X - size/8, bump1Y - size/8, size/4, size/4, color);
  
  int bump2X = x + size/2;
  int bump2Y = baseY - size/2;
  fillRect(bump2X - size/8, bump2Y - size/8, size/4, size/4, color);
  
  int bump3X = x + size*3/4;
  int bump3Y = baseY - size/3;
  fillRect(bump3X - size/8, bump3Y - size/8, size/4, size/4, color);
}

void drawRainIcon(int x, int y, uint16_t color, int size) {
  // Draw cloud first (smaller)
  drawCloudIcon(x, y, color, size * 2 / 3);
  
  // Rain drops
  int dropSpacing = size / 6;
  int dropHeight = size / 4;
  for (int i = 0; i < 3; i++) {
    int dropX = x + (i + 1) * dropSpacing;
    int dropY = y + size * 2 / 3;
    drawLine(dropX, dropY, dropX, dropY + dropHeight, color);
  }
}

void drawHomeIcon(int x, int y, uint16_t color, int size) {
  int centerX = x + size / 2;
  
  // House roof
  drawLine(centerX, y, x, y + size / 2, color);         // Left roof
  drawLine(centerX, y, x + size, y + size / 2, color);  // Right roof
  
  // House body
  fillRect(x + size/4, y + size/2, size/2, size/2, color);
  
  // Door
  fillRect(centerX - size/8, y + size*3/4, size/4, size/4, 0x0000); // Black door
}

void drawSettingsIcon(int x, int y, uint16_t color, int size) {
  int centerX = x + size / 2;
  int centerY = y + size / 2;
  
  // Gear teeth (simplified as lines around center)
  for (int i = 0; i < 8; i++) {
    float angle = i * PI / 4;
    int startX = centerX + (size/4) * cos(angle);
    int startY = centerY + (size/4) * sin(angle);
    int endX = centerX + (size/2) * cos(angle);
    int endY = centerY + (size/2) * sin(angle);
    drawLine(startX, startY, endX, endY, color);
  }
  
  // Center hole
  fillRect(centerX - 2, centerY - 2, 4, 4, 0x0000); // Black center
}

void drawMenuIcon(int x, int y, uint16_t color, int size) {
  // Three horizontal lines
  int lineHeight = 2;
  int spacing = size / 4;
  
  fillRect(x, y + spacing, size, lineHeight, color);
  fillRect(x, y + 2*spacing, size, lineHeight, color);
  fillRect(x, y + 3*spacing, size, lineHeight, color);
}

void drawArrowIcon(int x, int y, int direction, uint16_t color, int size) {
  // direction: 0=up, 1=right, 2=down, 3=left
  int centerX = x + size / 2;
  int centerY = y + size / 2;
  
  switch (direction) {
    case 0: // Up arrow
      drawLine(centerX, y, x, y + size/2, color);
      drawLine(centerX, y, x + size, y + size/2, color);
      drawLine(centerX, y, centerX, y + size, color);
      break;
    case 1: // Right arrow
      drawLine(x + size, centerY, x + size/2, y, color);
      drawLine(x + size, centerY, x + size/2, y + size, color);
      drawLine(x + size, centerY, x, centerY, color);
      break;
    case 2: // Down arrow
      drawLine(centerX, y + size, x, y + size/2, color);
      drawLine(centerX, y + size, x + size, y + size/2, color);
      drawLine(centerX, y + size, centerX, y, color);
      break;
    case 3: // Left arrow
      drawLine(x, centerY, x + size/2, y, color);
      drawLine(x, centerY, x + size/2, y + size, color);
      drawLine(x, centerY, x + size, centerY, color);
      break;
  }
}

#endif