#ifndef TOUCH_H
#define TOUCH_H

#include <Arduino.h>

// Touch System for T-Display-S3 Dashboard
// Phase 3A: Touch Input Foundation
// Uses ESP32-S3 capacitive touch sensing on GPIO pins

// Touch configuration
#define TOUCH_THRESHOLD 40        // Touch sensitivity threshold
#define TOUCH_DEBOUNCE_MS 50     // Debounce delay in milliseconds
#define LONG_PRESS_MS 1000       // Long press duration
#define SWIPE_MIN_DISTANCE 50    // Minimum swipe distance
#define SWIPE_MAX_TIME 500       // Maximum swipe time in ms

// Touch GPIO pins (ESP32-S3 has 14 touch-capable pins)
#define TOUCH_PIN_1 1    // GPIO1 - Left side touch
#define TOUCH_PIN_2 2    // GPIO2 - Right side touch  
#define TOUCH_PIN_3 3    // GPIO3 - Top touch
#define TOUCH_PIN_4 4    // GPIO4 - Bottom touch (also battery voltage)
#define TOUCH_PIN_5 5    // GPIO5 - Center touch
#define TOUCH_PIN_6 6    // GPIO6 - Available for custom zones

// Touch zones (screen areas mapped to touch pins)
struct TouchZone {
  int x, y, width, height;  // Screen coordinates
  int touchPin;            // Associated GPIO pin
  String name;             // Zone identifier
  bool enabled;            // Zone active state
};

// Touch event types
enum TouchEventType {
  TOUCH_NONE,
  TOUCH_PRESS,
  TOUCH_RELEASE,
  TOUCH_LONG_PRESS,
  SWIPE_LEFT,
  SWIPE_RIGHT,
  SWIPE_UP,
  SWIPE_DOWN
};

// Touch event structure
struct TouchEvent {
  TouchEventType type;
  int zoneIndex;
  String zoneName;
  unsigned long timestamp;
  int x, y;  // Touch coordinates (estimated)
};

// Touch state tracking
struct TouchState {
  bool pressed;
  unsigned long pressTime;
  unsigned long lastReadTime;
  int lastValue;
  bool longPressTriggered;
};

// Touch zones definition - mapped to screen layout
const int MAX_TOUCH_ZONES = 8;
extern TouchZone touchZones[MAX_TOUCH_ZONES];
extern TouchState touchStates[MAX_TOUCH_ZONES];
extern TouchEvent lastTouchEvent;

// Touch system functions
void initTouchSystem();
void updateTouchSystem();
TouchEvent getLastTouchEvent();
bool isTouchDetected(int zoneIndex);
void calibrateTouchThreshold();
void enableTouchZone(int zoneIndex, bool enabled);
void setTouchZone(int zoneIndex, int x, int y, int width, int height, int touchPin, String name);

// Touch feedback functions
void touchFeedback(int zoneIndex);
void visualTouchFeedback(int x, int y);

// Gesture detection
bool detectSwipe(int startZone, int endZone, unsigned long timeMs);
void resetTouchState();

// Touch calibration and debugging
void printTouchValues();
void touchCalibrationMode();

// Implementation
TouchZone touchZones[MAX_TOUCH_ZONES];
TouchState touchStates[MAX_TOUCH_ZONES];
TouchEvent lastTouchEvent = {TOUCH_NONE, -1, "", 0, 0, 0};

void initTouchSystem() {
  Serial.println("=== Initializing Touch System ===");
  
  // Initialize touch zones for current screen layout
  // Zone 0: Left side navigation
  setTouchZone(0, 0, 0, 100, 168, TOUCH_PIN_1, "nav_left");
  
  // Zone 1: Right side navigation  
  setTouchZone(1, 200, 0, 100, 168, TOUCH_PIN_2, "nav_right");
  
  // Zone 2: Top header area
  setTouchZone(2, 0, 0, 300, 40, TOUCH_PIN_3, "header");
  
  // Zone 3: Bottom status area
  setTouchZone(3, 0, 128, 300, 40, TOUCH_PIN_4, "status");
  
  // Zone 4: Center content area
  setTouchZone(4, 50, 40, 200, 88, TOUCH_PIN_5, "content");
  
  // Zone 5: Settings access (top-right corner)
  setTouchZone(5, 250, 0, 50, 30, TOUCH_PIN_6, "settings");
  
  // Initialize touch states
  for (int i = 0; i < MAX_TOUCH_ZONES; i++) {
    touchStates[i] = {false, 0, 0, 0, false};
  }
  
  Serial.println("Touch zones configured:");
  for (int i = 0; i < MAX_TOUCH_ZONES; i++) {
    if (touchZones[i].enabled) {
      Serial.print("Zone "); Serial.print(i); 
      Serial.print(": "); Serial.print(touchZones[i].name);
      Serial.print(" ("); Serial.print(touchZones[i].x); Serial.print(","); 
      Serial.print(touchZones[i].y); Serial.print(") ");
      Serial.print(touchZones[i].width); Serial.print("x"); Serial.print(touchZones[i].height);
      Serial.print(" -> GPIO"); Serial.println(touchZones[i].touchPin);
    }
  }
  
  Serial.println("Touch system ready!");
}

void updateTouchSystem() {
  unsigned long currentTime = millis();
  
  for (int i = 0; i < MAX_TOUCH_ZONES; i++) {
    if (!touchZones[i].enabled) continue;
    
    // Read touch value
    int touchValue = touchRead(touchZones[i].touchPin);
    
    // Touch detection logic
    bool currentlyTouched = touchValue < TOUCH_THRESHOLD;
    bool wasPressed = touchStates[i].pressed;
    
    // Debounce check
    if (currentTime - touchStates[i].lastReadTime < TOUCH_DEBOUNCE_MS) {
      continue;
    }
    
    touchStates[i].lastReadTime = currentTime;
    touchStates[i].lastValue = touchValue;
    
    // Touch press detection
    if (currentlyTouched && !wasPressed) {
      // New touch detected
      touchStates[i].pressed = true;
      touchStates[i].pressTime = currentTime;
      touchStates[i].longPressTriggered = false;
      
      // Create touch press event
      lastTouchEvent = {
        TOUCH_PRESS,
        i,
        touchZones[i].name,
        currentTime,
        touchZones[i].x + touchZones[i].width/2,
        touchZones[i].y + touchZones[i].height/2
      };
      
      // Visual feedback
      touchFeedback(i);
      
      Serial.print("Touch PRESS: Zone "); Serial.print(i); 
      Serial.print(" ("); Serial.print(touchZones[i].name); Serial.print(") ");
      Serial.print("Value: "); Serial.println(touchValue);
    }
    
    // Touch release detection
    else if (!currentlyTouched && wasPressed) {
      // Touch released
      touchStates[i].pressed = false;
      unsigned long pressDuration = currentTime - touchStates[i].pressTime;
      
      // Create appropriate release event
      TouchEventType eventType = TOUCH_RELEASE;
      if (pressDuration >= LONG_PRESS_MS && !touchStates[i].longPressTriggered) {
        eventType = TOUCH_LONG_PRESS;
      }
      
      lastTouchEvent = {
        eventType,
        i,
        touchZones[i].name,
        currentTime,
        touchZones[i].x + touchZones[i].width/2,
        touchZones[i].y + touchZones[i].height/2
      };
      
      Serial.print("Touch RELEASE: Zone "); Serial.print(i); 
      Serial.print(" Duration: "); Serial.print(pressDuration); Serial.println("ms");
    }
    
    // Long press detection (while still pressed)
    else if (currentlyTouched && wasPressed) {
      unsigned long pressDuration = currentTime - touchStates[i].pressTime;
      
      if (pressDuration >= LONG_PRESS_MS && !touchStates[i].longPressTriggered) {
        touchStates[i].longPressTriggered = true;
        
        lastTouchEvent = {
          TOUCH_LONG_PRESS,
          i,
          touchZones[i].name,
          currentTime,
          touchZones[i].x + touchZones[i].width/2,
          touchZones[i].y + touchZones[i].height/2
        };
        
        Serial.print("Touch LONG PRESS: Zone "); Serial.print(i); 
        Serial.print(" ("); Serial.print(touchZones[i].name); Serial.println(")");
      }
    }
  }
}

TouchEvent getLastTouchEvent() {
  TouchEvent event = lastTouchEvent;
  lastTouchEvent = {TOUCH_NONE, -1, "", 0, 0, 0}; // Clear after reading
  return event;
}

bool isTouchDetected(int zoneIndex) {
  if (zoneIndex < 0 || zoneIndex >= MAX_TOUCH_ZONES || !touchZones[zoneIndex].enabled) {
    return false;
  }
  return touchStates[zoneIndex].pressed;
}

void setTouchZone(int zoneIndex, int x, int y, int width, int height, int touchPin, String name) {
  if (zoneIndex >= 0 && zoneIndex < MAX_TOUCH_ZONES) {
    touchZones[zoneIndex] = {x, y, width, height, touchPin, name, true};
  }
}

void enableTouchZone(int zoneIndex, bool enabled) {
  if (zoneIndex >= 0 && zoneIndex < MAX_TOUCH_ZONES) {
    touchZones[zoneIndex].enabled = enabled;
  }
}

void touchFeedback(int zoneIndex) {
  // Visual feedback - brief highlight of touched zone
  if (zoneIndex >= 0 && zoneIndex < MAX_TOUCH_ZONES) {
    // Forward declaration for graphics functions
    extern void fillVisibleRect(int x, int y, int w, int h, uint16_t color);
    extern uint16_t getPrimaryColor();
    
    TouchZone& zone = touchZones[zoneIndex];
    
    // Brief highlight overlay
    fillVisibleRect(zone.x, zone.y, zone.width, zone.height, getPrimaryColor());
    delay(50);  // Brief visual feedback
  }
}

void visualTouchFeedback(int x, int y) {
  // Create expanding circle effect at touch point
  extern void fillCircle(int x0, int y0, int r, uint16_t color);
  extern uint16_t getPrimaryColor();
  extern uint16_t getBackgroundColor();
  
  for (int r = 1; r <= 8; r++) {
    fillCircle(x, y, r, getPrimaryColor());
    delay(20);
  }
  
  // Clear the effect
  for (int r = 8; r >= 1; r--) {
    fillCircle(x, y, r, getBackgroundColor());
    delay(10);
  }
}

void printTouchValues() {
  Serial.println("=== Touch Values Debug ===");
  for (int i = 0; i < MAX_TOUCH_ZONES; i++) {
    if (touchZones[i].enabled) {
      int value = touchRead(touchZones[i].touchPin);
      Serial.print("Zone "); Serial.print(i); 
      Serial.print(" ("); Serial.print(touchZones[i].name); Serial.print("): ");
      Serial.print(value); 
      Serial.print(" [GPIO"); Serial.print(touchZones[i].touchPin); Serial.print("]");
      if (value < TOUCH_THRESHOLD) Serial.print(" TOUCHED");
      Serial.println();
    }
  }
  Serial.println("========================");
}

void touchCalibrationMode() {
  Serial.println("=== Touch Calibration Mode ===");
  Serial.println("Touch each zone to calibrate thresholds...");
  
  for (int i = 0; i < 20; i++) {  // 20 second calibration period
    printTouchValues();
    delay(1000);
  }
  
  Serial.println("Calibration complete!");
}

void resetTouchState() {
  for (int i = 0; i < MAX_TOUCH_ZONES; i++) {
    touchStates[i] = {false, 0, 0, 0, false};
  }
  lastTouchEvent = {TOUCH_NONE, -1, "", 0, 0, 0};
}

#endif