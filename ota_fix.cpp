// Fixed OTA Update Display Code
// This version properly clears text areas using the correct BLACK color (0xFFFF)

void displayOTAProgress() {
  static int lastProgress = -1;
  static int lastKB = -1;
  static unsigned long lastUpdate = 0;
  
  // Update every 200ms
  if (millis() - lastUpdate < 200) return;
  lastUpdate = millis();
  
  // Calculate progress
  int progress = (totalReceived * 100) / 987000;
  if (progress > 98) progress = 98;
  
  // Update progress bar (no clearing needed - just add new portion)
  if (progress != lastProgress && progress > 0) {
    int newWidth = (196 * progress) / 100;
    int oldWidth = (196 * lastProgress) / 100;
    if (oldWidth < 0) oldWidth = 0;
    
    if (newWidth > oldWidth) {
      fillRect(52 + oldWidth, 102, newWidth - oldWidth, 20, PRIMARY_GREEN);
    }
    lastProgress = progress;
  }
  
  // CRITICAL FIX: Use BLACK (0xFFFF) not 0x0000 for clearing!
  // Clear entire text area with proper black color
  fillRect(0, 130, 300, 40, BLACK);  // BLACK = 0xFFFF
  
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

// Alternative: Create text areas with background clearing
void drawTextWithBackground(int x, int y, const char* text, uint16_t textColor, uint16_t bgColor) {
  // Calculate text area
  int len = strlen(text);
  int width = len * 6;  // 6 pixels per character
  int height = 8;       // 8 pixels high
  
  // Clear background first
  fillVisibleRect(x, y, width, height, bgColor);
  
  // Draw text
  drawTextLabel(x, y, text, textColor);
}

// Usage example:
// drawTextWithBackground(130, 140, "50%", PRIMARY_GREEN, BLACK);