#ifndef GRAPHICS_H
#define GRAPHICS_H

#include <Arduino.h>

// Forward declarations for display functions
extern void setDisplayArea(int x1, int y1, int x2, int y2);
extern void writeCommand(uint8_t cmd);
extern void writeData(uint8_t data);

// Enhanced Graphics Functions for T-Display-S3
// Phase 1: Better Graphics Implementation
// UPDATED: Uses maximum screen real estate and correct boundary checking

// MAXIMUM USABLE DISPLAY AREA - From verified testing
#define MAX_DISPLAY_X_START 10   // Left boundary
#define MAX_DISPLAY_Y_START 36   // Top boundary  
#define MAX_DISPLAY_WIDTH   300  // Maximum width
#define MAX_DISPLAY_HEIGHT  168  // Maximum height

// Basic drawing primitives (enhanced versions)
void drawPixel(int x, int y, uint16_t color);
void drawLine(int x0, int y0, int x1, int y1, uint16_t color);
void drawRect(int x, int y, int w, int h, uint16_t color);
void fillRect(int x, int y, int w, int h, uint16_t color);
void drawCircle(int x0, int y0, int r, uint16_t color);
void fillCircle(int x0, int y0, int r, uint16_t color);

// New enhanced drawing functions
void drawRoundRect(int x, int y, int w, int h, int radius, uint16_t color);
void fillRoundRect(int x, int y, int w, int h, int radius, uint16_t color);

// Gradient functions
void fillGradientH(int x, int y, int w, int h, uint16_t color1, uint16_t color2);
void fillGradientV(int x, int y, int w, int h, uint16_t color1, uint16_t color2);
void fillGradientRadial(int cx, int cy, int radius, uint16_t centerColor, uint16_t edgeColor);

// Visual effects
void drawShadowRect(int x, int y, int w, int h, uint16_t color, uint16_t shadowColor, int shadowOffset);
void drawBorderedRect(int x, int y, int w, int h, uint16_t fillColor, uint16_t borderColor, int borderWidth);

// Progress indicators
void drawProgressBar(int x, int y, int w, int h, int progress, uint16_t bgColor, uint16_t fillColor);
void drawProgressCircle(int cx, int cy, int radius, int progress, uint16_t bgColor, uint16_t fillColor);

// Utility functions
uint16_t interpolateColor(uint16_t color1, uint16_t color2, float ratio);
uint16_t rgb565(uint8_t r, uint8_t g, uint8_t b);
void rgb565ToRgb(uint16_t color, uint8_t &r, uint8_t &g, uint8_t &b);

// Enhanced drawing within verified visible area
void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
bool isWithinVisibleArea(int x, int y, int w, int h);

// Implementation
void drawPixel(int x, int y, uint16_t color) {
  if (x < 0 || x >= 320 || y < 0 || y >= 240) return;
  
  setDisplayArea(x, y, x, y);
  writeCommand(0x2C);
  writeData((color >> 8) & 0xFF);
  writeData(color & 0xFF);
}

void drawLine(int x0, int y0, int x1, int y1, uint16_t color) {
  int dx = abs(x1 - x0);
  int dy = abs(y1 - y0);
  int sx = (x0 < x1) ? 1 : -1;
  int sy = (y0 < y1) ? 1 : -1;
  int err = dx - dy;
  
  while (true) {
    drawPixel(x0, y0, color);
    
    if (x0 == x1 && y0 == y1) break;
    
    int e2 = 2 * err;
    if (e2 > -dy) {
      err -= dy;
      x0 += sx;
    }
    if (e2 < dx) {
      err += dx;
      y0 += sy;
    }
  }
}

void fillRect(int x, int y, int w, int h, uint16_t color) {
  if (x < 0 || y < 0 || x >= 320 || y >= 240 || x + w > 320 || y + h > 240) return;
  
  setDisplayArea(x, y, x + w - 1, y + h - 1);
  writeCommand(0x2C);
  
  for (int i = 0; i < w * h; i++) {
    writeData((color >> 8) & 0xFF);
    writeData(color & 0xFF);
  }
}

void drawRect(int x, int y, int w, int h, uint16_t color) {
  drawLine(x, y, x + w - 1, y, color);           // Top
  drawLine(x, y + h - 1, x + w - 1, y + h - 1, color); // Bottom
  drawLine(x, y, x, y + h - 1, color);           // Left
  drawLine(x + w - 1, y, x + w - 1, y + h - 1, color); // Right
}

void drawCircle(int x0, int y0, int r, uint16_t color) {
  int x = r;
  int y = 0;
  int err = 0;
  
  while (x >= y) {
    drawPixel(x0 + x, y0 + y, color);
    drawPixel(x0 + y, y0 + x, color);
    drawPixel(x0 - y, y0 + x, color);
    drawPixel(x0 - x, y0 + y, color);
    drawPixel(x0 - x, y0 - y, color);
    drawPixel(x0 - y, y0 - x, color);
    drawPixel(x0 + y, y0 - x, color);
    drawPixel(x0 + x, y0 - y, color);
    
    if (err <= 0) {
      y += 1;
      err += 2*y + 1;
    }
    if (err > 0) {
      x -= 1;
      err -= 2*x + 1;
    }
  }
}

void fillCircle(int x0, int y0, int r, uint16_t color) {
  for (int y = -r; y <= r; y++) {
    for (int x = -r; x <= r; x++) {
      if (x*x + y*y <= r*r) {
        drawPixel(x0 + x, y0 + y, color);
      }
    }
  }
}

void drawRoundRect(int x, int y, int w, int h, int radius, uint16_t color) {
  if (radius > w/2) radius = w/2;
  if (radius > h/2) radius = h/2;
  
  // Draw the four edges
  drawLine(x + radius, y, x + w - radius - 1, y, color);         // Top
  drawLine(x + radius, y + h - 1, x + w - radius - 1, y + h - 1, color); // Bottom
  drawLine(x, y + radius, x, y + h - radius - 1, color);         // Left
  drawLine(x + w - 1, y + radius, x + w - 1, y + h - radius - 1, color); // Right
  
  // Draw rounded corners using circle quadrants
  int cx, cy;
  
  // Top-left corner
  cx = x + radius;
  cy = y + radius;
  for (int angle = 180; angle <= 270; angle += 5) {
    int px = cx + radius * cos(angle * PI / 180);
    int py = cy + radius * sin(angle * PI / 180);
    drawPixel(px, py, color);
  }
  
  // Top-right corner
  cx = x + w - radius - 1;
  cy = y + radius;
  for (int angle = 270; angle <= 360; angle += 5) {
    int px = cx + radius * cos(angle * PI / 180);
    int py = cy + radius * sin(angle * PI / 180);
    drawPixel(px, py, color);
  }
  
  // Bottom-right corner
  cx = x + w - radius - 1;
  cy = y + h - radius - 1;
  for (int angle = 0; angle <= 90; angle += 5) {
    int px = cx + radius * cos(angle * PI / 180);
    int py = cy + radius * sin(angle * PI / 180);
    drawPixel(px, py, color);
  }
  
  // Bottom-left corner
  cx = x + radius;
  cy = y + h - radius - 1;
  for (int angle = 90; angle <= 180; angle += 5) {
    int px = cx + radius * cos(angle * PI / 180);
    int py = cy + radius * sin(angle * PI / 180);
    drawPixel(px, py, color);
  }
}

void fillRoundRect(int x, int y, int w, int h, int radius, uint16_t color) {
  if (radius > w/2) radius = w/2;
  if (radius > h/2) radius = h/2;
  
  // Fill main rectangle (minus corners)
  fillRect(x + radius, y, w - 2*radius, h, color);
  fillRect(x, y + radius, radius, h - 2*radius, color);
  fillRect(x + w - radius, y + radius, radius, h - 2*radius, color);
  
  // Fill rounded corners
  for (int dy = 0; dy < radius; dy++) {
    for (int dx = 0; dx < radius; dx++) {
      if (dx*dx + dy*dy <= radius*radius) {
        // Top-left
        drawPixel(x + radius - dx - 1, y + radius - dy - 1, color);
        // Top-right
        drawPixel(x + w - radius + dx, y + radius - dy - 1, color);
        // Bottom-left
        drawPixel(x + radius - dx - 1, y + h - radius + dy, color);
        // Bottom-right
        drawPixel(x + w - radius + dx, y + h - radius + dy, color);
      }
    }
  }
}

void fillGradientH(int x, int y, int w, int h, uint16_t color1, uint16_t color2) {
  for (int i = 0; i < w; i++) {
    float ratio = (float)i / (w - 1);
    uint16_t color = interpolateColor(color1, color2, ratio);
    fillRect(x + i, y, 1, h, color);
  }
}

void fillGradientV(int x, int y, int w, int h, uint16_t color1, uint16_t color2) {
  for (int i = 0; i < h; i++) {
    float ratio = (float)i / (h - 1);
    uint16_t color = interpolateColor(color1, color2, ratio);
    fillRect(x, y + i, w, 1, color);
  }
}

void drawShadowRect(int x, int y, int w, int h, uint16_t color, uint16_t shadowColor, int shadowOffset) {
  // Draw shadow first (offset)
  fillRect(x + shadowOffset, y + shadowOffset, w, h, shadowColor);
  // Draw main rectangle on top
  fillRect(x, y, w, h, color);
}

void drawBorderedRect(int x, int y, int w, int h, uint16_t fillColor, uint16_t borderColor, int borderWidth) {
  // Fill interior first
  fillRect(x, y, w, h, fillColor);
  
  // Draw border within screen bounds
  for (int i = 0; i < borderWidth; i++) {
    int borderX = max(0, x - i);
    int borderY = max(0, y - i);
    int borderW = min(320 - borderX, w + 2*i - (borderX - (x - i)));
    int borderH = min(240 - borderY, h + 2*i - (borderY - (y - i)));
    
    if (borderW > 0 && borderH > 0) {
      drawRect(borderX, borderY, borderW, borderH, borderColor);
    }
  }
}

void drawProgressBar(int x, int y, int w, int h, int progress, uint16_t bgColor, uint16_t fillColor) {
  // Background
  fillRect(x, y, w, h, bgColor);
  
  // Progress fill (0-100%)
  int fillWidth = (progress * w) / 100;
  if (fillWidth > 0) {
    fillRect(x, y, fillWidth, h, fillColor);
  }
  
  // Border
  drawRect(x, y, w, h, 0xFFFF);
}

uint16_t interpolateColor(uint16_t color1, uint16_t color2, float ratio) {
  if (ratio <= 0) return color1;
  if (ratio >= 1) return color2;
  
  uint8_t r1, g1, b1, r2, g2, b2;
  rgb565ToRgb(color1, r1, g1, b1);
  rgb565ToRgb(color2, r2, g2, b2);
  
  uint8_t r = r1 + (r2 - r1) * ratio;
  uint8_t g = g1 + (g2 - g1) * ratio;
  uint8_t b = b1 + (b2 - b1) * ratio;
  
  return rgb565(r, g, b);
}

uint16_t rgb565(uint8_t r, uint8_t g, uint8_t b) {
  return ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
}

void rgb565ToRgb(uint16_t color, uint8_t &r, uint8_t &g, uint8_t &b) {
  r = (color >> 8) & 0xF8;
  g = (color >> 3) & 0xFC;
  b = (color << 3) & 0xF8;
}

// Enhanced drawing within verified visible area
void fillVisibleRect(int x, int y, int w, int h, uint16_t color) {
  // Convert visible coordinates to actual coordinates
  int actualX = MAX_DISPLAY_X_START + x;
  int actualY = MAX_DISPLAY_Y_START + y;
  
  // Boundary check within visible area
  if (x < 0 || y < 0 || x + w > MAX_DISPLAY_WIDTH || y + h > MAX_DISPLAY_HEIGHT) {
    return; // Out of bounds
  }
  
  fillRect(actualX, actualY, w, h, color);
}

bool isWithinVisibleArea(int x, int y, int w, int h) {
  return (x >= 0 && y >= 0 && 
          x + w <= MAX_DISPLAY_WIDTH && 
          y + h <= MAX_DISPLAY_HEIGHT);
}

#endif