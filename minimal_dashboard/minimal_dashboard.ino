// Minimal ESP32-S3 T-Display Dashboard Test
// This tests basic display functionality to isolate issues

#include <TFT_eSPI.h>

TFT_eSPI tft = TFT_eSPI();

// Pin definitions for T-Display-S3
#define LCD_POWER_PIN 15
#define LCD_BACKLIGHT 38

void setup() {
  Serial.begin(115200);
  delay(1000);
  Serial.println("\n=== ESP32-S3 Minimal Display Test v1.0 ===");
  
  // Initialize power pins
  pinMode(LCD_POWER_PIN, OUTPUT);
  digitalWrite(LCD_POWER_PIN, HIGH);
  delay(100);
  
  pinMode(LCD_BACKLIGHT, OUTPUT);
  digitalWrite(LCD_BACKLIGHT, HIGH);
  
  // Initialize display
  tft.init();
  tft.setRotation(0);
  tft.fillScreen(TFT_BLACK);
  
  Serial.println("Display initialized");
  
  // Show test pattern
  tft.fillScreen(TFT_RED);
  delay(500);
  tft.fillScreen(TFT_GREEN);
  delay(500);
  tft.fillScreen(TFT_BLUE);
  delay(500);
  
  // Show text
  tft.fillScreen(TFT_BLACK);
  tft.setTextColor(TFT_WHITE);
  tft.setTextSize(2);
  tft.setCursor(10, 10);
  tft.println("ESP32-S3 Display");
  tft.println("Test Success!");
  
  Serial.println("Test pattern complete");
}

void loop() {
  static uint32_t lastUpdate = 0;
  static uint8_t counter = 0;
  
  if (millis() - lastUpdate > 1000) {
    lastUpdate = millis();
    
    // Update counter
    tft.fillRect(10, 60, 200, 30, TFT_BLACK);
    tft.setCursor(10, 60);
    tft.print("Counter: ");
    tft.print(counter++);
    
    Serial.print("Counter: ");
    Serial.println(counter);
  }
}