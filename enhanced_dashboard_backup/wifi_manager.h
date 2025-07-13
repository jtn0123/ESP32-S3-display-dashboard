#ifndef WIFI_MANAGER_H
#define WIFI_MANAGER_H

#include <WiFi.h>
#include <WebServer.h>
#include <DNSServer.h>
#include <Preferences.h>
#include <time.h>
#include <ArduinoOTA.h>

// WiFi Manager for T-Display-S3 Dashboard
// Phase 3B: WiFi Connectivity & Web Interface

// WiFi Configuration
#define WIFI_TIMEOUT_MS 10000        // 10 second connection timeout
#define WIFI_RETRY_DELAY_MS 5000     // 5 second retry delay
#define AP_TIMEOUT_MS 300000         // 5 minute AP mode timeout
#define WIFI_SCAN_INTERVAL_MS 30000  // Scan for networks every 30 seconds

// Access Point Configuration
#define AP_SSID "T-Display-S3-Setup"
#define AP_PASSWORD "dashboard123"
#define AP_CHANNEL 1
#define AP_MAX_CONNECTIONS 4

// Web Server Configuration
#define WEB_SERVER_PORT 80
#define DNS_PORT 53

// NTP Configuration
#define NTP_SERVER "pool.ntp.org"
#define GMT_OFFSET_SEC 0
#define DAYLIGHT_OFFSET_SEC 3600

// WiFi Status
enum WiFiStatus {
  WIFI_DISCONNECTED,
  WIFI_CONNECTING,
  WIFI_CONNECTED,
  WIFI_AP_MODE,
  WIFI_FAILED,
  WIFI_SCANNING
};

// Network Information Structure
struct NetworkInfo {
  String ssid;
  String password;
  String ipAddress;
  String macAddress;
  int rssi;
  WiFiStatus status;
  unsigned long connectedTime;
  unsigned long bytesReceived;
  unsigned long bytesSent;
};

// Web Server and DNS
extern WebServer webServer;
extern DNSServer dnsServer;
extern Preferences preferences;

// Network state
extern NetworkInfo networkInfo;
extern WiFiStatus currentWiFiStatus;
extern unsigned long lastConnectionAttempt;
extern unsigned long lastNetworkScan;
extern int wifiScanResults;

// WiFi Management Functions
void initWiFiManager();
void updateWiFiManager();
bool connectToWiFi(String ssid, String password);
void startAccessPoint();
void stopAccessPoint();
void scanNetworks();
String getWiFiStatusString();
int getSignalQuality();

// Configuration Management
void saveWiFiCredentials(String ssid, String password);
bool loadWiFiCredentials();
void clearWiFiCredentials();

// Web Server Functions
void initWebServer();
void handleWebRequests();
void handleRoot();
void handleWiFiSetup();
void handleStatus();
void handleRestart();
void handleNetworkScan();

// Time Management
void initTimeSync();
String getCurrentTime();
String getUptime();

// OTA Update Functions
void initOTA();
void handleOTA();

// Network Utilities
String getLocalIP();
String getMacAddress();
long getRSSI();
String getNetworkStats();

// Implementation
WebServer webServer(WEB_SERVER_PORT);
DNSServer dnsServer;
Preferences preferences;

NetworkInfo networkInfo;
WiFiStatus currentWiFiStatus = WIFI_DISCONNECTED;
unsigned long lastConnectionAttempt = 0;
unsigned long lastNetworkScan = 0;
int wifiScanResults = 0;

void initWiFiManager() {
  Serial.println("=== Initializing WiFi Manager ===");
  
  // Initialize preferences
  preferences.begin("wifi-config", false);
  
  // Set WiFi mode
  WiFi.mode(WIFI_STA);
  WiFi.setHostname("T-Display-S3");
  
  // Initialize network info
  networkInfo = {"", "", "", getMacAddress(), 0, WIFI_DISCONNECTED, 0, 0, 0};
  
  // Try to load saved credentials
  if (loadWiFiCredentials()) {
    Serial.println("Loaded saved WiFi credentials");
    if (connectToWiFi(networkInfo.ssid, networkInfo.password)) {
      Serial.println("Connected using saved credentials");
    } else {
      Serial.println("Failed to connect with saved credentials");
      startAccessPoint();
    }
  } else {
    Serial.println("No saved credentials found");
    startAccessPoint();
  }
  
  // Initialize web server
  initWebServer();
  
  // Initialize OTA
  initOTA();
  
  // Initialize time sync if connected
  if (WiFi.status() == WL_CONNECTED) {
    initTimeSync();
  }
  
  Serial.println("WiFi Manager initialized");
}

void updateWiFiManager() {
  unsigned long now = millis();
  
  // Handle web server requests
  webServer.handleClient();
  dnsServer.processNextRequest();
  
  // Handle OTA updates
  ArduinoOTA.handle();
  
  // Update WiFi status
  WiFiStatus newStatus = currentWiFiStatus;
  
  if (WiFi.status() == WL_CONNECTED) {
    if (currentWiFiStatus != WIFI_CONNECTED) {
      newStatus = WIFI_CONNECTED;
      networkInfo.ipAddress = getLocalIP();
      networkInfo.connectedTime = now;
      Serial.println("WiFi Connected: " + networkInfo.ipAddress);
      
      // Initialize time sync on connection
      initTimeSync();
    }
    
    // Update network stats
    networkInfo.rssi = WiFi.RSSI();
    // Note: ESP32 WiFi library doesn't provide byte counters
    // networkInfo.bytesReceived and bytesSent remain at their initial values
    
  } else {
    if (currentWiFiStatus == WIFI_CONNECTED) {
      newStatus = WIFI_DISCONNECTED;
      Serial.println("WiFi Disconnected");
    }
  }
  
  currentWiFiStatus = newStatus;
  networkInfo.status = currentWiFiStatus;
  
  // Periodic network scan in AP mode
  if (currentWiFiStatus == WIFI_AP_MODE && now - lastNetworkScan > WIFI_SCAN_INTERVAL_MS) {
    scanNetworks();
    lastNetworkScan = now;
  }
}

bool connectToWiFi(String ssid, String password) {
  Serial.print("Connecting to WiFi: ");
  Serial.println(ssid);
  
  currentWiFiStatus = WIFI_CONNECTING;
  networkInfo.ssid = ssid;
  networkInfo.password = password;
  
  WiFi.begin(ssid.c_str(), password.c_str());
  
  unsigned long startTime = millis();
  while (WiFi.status() != WL_CONNECTED && millis() - startTime < WIFI_TIMEOUT_MS) {
    delay(500);
    Serial.print(".");
  }
  Serial.println();
  
  if (WiFi.status() == WL_CONNECTED) {
    currentWiFiStatus = WIFI_CONNECTED;
    networkInfo.ipAddress = WiFi.localIP().toString();
    networkInfo.rssi = WiFi.RSSI();
    
    Serial.println("WiFi Connected!");
    Serial.print("IP Address: ");
    Serial.println(networkInfo.ipAddress);
    Serial.print("Signal Strength: ");
    Serial.print(networkInfo.rssi);
    Serial.println(" dBm");
    
    // Save credentials
    saveWiFiCredentials(ssid, password);
    
    return true;
  } else {
    currentWiFiStatus = WIFI_FAILED;
    Serial.println("WiFi Connection Failed");
    return false;
  }
}

void startAccessPoint() {
  Serial.println("Starting Access Point...");
  
  WiFi.mode(WIFI_AP_STA);
  WiFi.softAP(AP_SSID, AP_PASSWORD, AP_CHANNEL, false, AP_MAX_CONNECTIONS);
  
  currentWiFiStatus = WIFI_AP_MODE;
  networkInfo.ipAddress = WiFi.softAPIP().toString();
  
  // Start captive portal DNS
  dnsServer.start(DNS_PORT, "*", WiFi.softAPIP());
  
  Serial.println("Access Point Started");
  Serial.print("AP SSID: ");
  Serial.println(AP_SSID);
  Serial.print("AP IP: ");
  Serial.println(networkInfo.ipAddress);
  Serial.println("Connect to setup WiFi credentials");
}

void stopAccessPoint() {
  WiFi.softAPdisconnect(true);
  dnsServer.stop();
  Serial.println("Access Point Stopped");
}

void scanNetworks() {
  Serial.println("Scanning for networks...");
  currentWiFiStatus = WIFI_SCANNING;
  
  wifiScanResults = WiFi.scanNetworks();
  
  if (wifiScanResults > 0) {
    Serial.print("Found ");
    Serial.print(wifiScanResults);
    Serial.println(" networks:");
    
    for (int i = 0; i < wifiScanResults; i++) {
      Serial.print("  ");
      Serial.print(WiFi.SSID(i));
      Serial.print(" (");
      Serial.print(WiFi.RSSI(i));
      Serial.print(" dBm) ");
      Serial.println(WiFi.encryptionType(i) == WIFI_AUTH_OPEN ? "Open" : "Secured");
    }
  } else {
    Serial.println("No networks found");
  }
}

String getWiFiStatusString() {
  switch (currentWiFiStatus) {
    case WIFI_CONNECTED: return "Connected";
    case WIFI_CONNECTING: return "Connecting";
    case WIFI_AP_MODE: return "AP Mode";
    case WIFI_SCANNING: return "Scanning";
    case WIFI_FAILED: return "Failed";
    default: return "Disconnected";
  }
}

int getSignalQuality() {
  if (currentWiFiStatus != WIFI_CONNECTED) return 0;
  
  int rssi = WiFi.RSSI();
  if (rssi <= -100) return 0;
  if (rssi >= -50) return 100;
  return 2 * (rssi + 100);
}

void saveWiFiCredentials(String ssid, String password) {
  preferences.putString("wifi_ssid", ssid);
  preferences.putString("wifi_pass", password);
  Serial.println("WiFi credentials saved");
}

bool loadWiFiCredentials() {
  String ssid = preferences.getString("wifi_ssid", "");
  String password = preferences.getString("wifi_pass", "");
  
  if (ssid.length() > 0) {
    networkInfo.ssid = ssid;
    networkInfo.password = password;
    return true;
  }
  return false;
}

void clearWiFiCredentials() {
  preferences.remove("wifi_ssid");
  preferences.remove("wifi_pass");
  Serial.println("WiFi credentials cleared");
}

void initWebServer() {
  // Setup web server routes
  webServer.on("/", handleRoot);
  webServer.on("/setup", handleWiFiSetup);
  webServer.on("/status", handleStatus);
  webServer.on("/restart", handleRestart);
  webServer.on("/scan", handleNetworkScan);
  
  webServer.begin();
  Serial.println("Web server started on port 80");
}

void handleRoot() {
  String html = R"(
<!DOCTYPE html>
<html>
<head>
    <title>T-Display S3 Dashboard</title>
    <meta charset='utf-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1'>
    <style>
        body { font-family: Arial; margin: 20px; background: #1a1a1a; color: white; }
        .container { max-width: 600px; margin: 0 auto; }
        .status { background: #333; padding: 15px; border-radius: 5px; margin: 10px 0; }
        button { background: #ff6b35; color: white; border: none; padding: 10px 20px; margin: 5px; border-radius: 3px; cursor: pointer; }
        button:hover { background: #e55a2b; }
        input { padding: 8px; margin: 5px; border: 1px solid #555; background: #222; color: white; border-radius: 3px; }
        .network { background: #2a2a2a; padding: 10px; margin: 5px 0; border-radius: 3px; }
        .signal { float: right; }
    </style>
</head>
<body>
    <div class='container'>
        <h1>🖥️ T-Display S3 Dashboard</h1>
        <div class='status'>
            <h3>Status</h3>
            <p>WiFi: )" + getWiFiStatusString() + R"(</p>
            <p>IP: )" + networkInfo.ipAddress + R"(</p>
            <p>Signal: )" + String(getSignalQuality()) + R"(%</p>
            <p>Uptime: )" + getUptime() + R"(</p>
        </div>
        <div class='status'>
            <h3>WiFi Setup</h3>
            <button onclick="location.href='/setup'">Configure WiFi</button>
            <button onclick="location.href='/scan'">Scan Networks</button>
            <button onclick="location.href='/restart'">Restart Device</button>
        </div>
    </div>
    <script>setTimeout(function(){location.reload();}, 5000);</script>
</body>
</html>
  )";
  
  webServer.send(200, "text/html", html);
}

void handleWiFiSetup() {
  if (webServer.method() == HTTP_POST) {
    String ssid = webServer.arg("ssid");
    String password = webServer.arg("password");
    
    if (ssid.length() > 0) {
      webServer.send(200, "text/html", 
        "<html><body><h1>Connecting...</h1><p>Attempting to connect to: " + ssid + "</p></body></html>");
      
      delay(1000);
      
      if (connectToWiFi(ssid, password)) {
        stopAccessPoint();
      }
      return;
    }
  }
  
  // Generate network list
  String networkList = "";
  for (int i = 0; i < wifiScanResults; i++) {
    int quality = 2 * (WiFi.RSSI(i) + 100);
    if (quality > 100) quality = 100;
    if (quality < 0) quality = 0;
    
    networkList += "<div class='network' onclick='selectNetwork(\"" + WiFi.SSID(i) + "\")'>";
    networkList += WiFi.SSID(i);
    networkList += "<span class='signal'>" + String(quality) + "%</span>";
    networkList += "</div>";
  }
  
  String html = R"(
<!DOCTYPE html>
<html>
<head>
    <title>WiFi Setup</title>
    <meta charset='utf-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1'>
    <style>
        body { font-family: Arial; margin: 20px; background: #1a1a1a; color: white; }
        .container { max-width: 600px; margin: 0 auto; }
        form { background: #333; padding: 20px; border-radius: 5px; }
        input { width: 100%; padding: 10px; margin: 10px 0; border: 1px solid #555; background: #222; color: white; border-radius: 3px; box-sizing: border-box; }
        button { background: #ff6b35; color: white; border: none; padding: 12px 20px; border-radius: 3px; cursor: pointer; width: 100%; }
        .network { background: #2a2a2a; padding: 15px; margin: 5px 0; border-radius: 3px; cursor: pointer; }
        .network:hover { background: #404040; }
        .signal { float: right; }
    </style>
</head>
<body>
    <div class='container'>
        <h1>WiFi Configuration</h1>
        <h3>Available Networks:</h3>
        )" + networkList + R"(
        <form method='POST'>
            <h3>WiFi Credentials:</h3>
            <input type='text' name='ssid' id='ssid' placeholder='WiFi Network Name' required>
            <input type='password' name='password' placeholder='WiFi Password'>
            <button type='submit'>Connect</button>
        </form>
        <br>
        <button onclick="location.href='/'">Back to Status</button>
    </div>
    <script>
        function selectNetwork(ssid) {
            document.getElementById('ssid').value = ssid;
        }
    </script>
</body>
</html>
  )";
  
  webServer.send(200, "text/html", html);
}

void handleStatus() {
  String json = "{";
  json += "\"wifi_status\":\"" + getWiFiStatusString() + "\",";
  json += "\"ip\":\"" + networkInfo.ipAddress + "\",";
  json += "\"rssi\":" + String(networkInfo.rssi) + ",";
  json += "\"signal_quality\":" + String(getSignalQuality()) + ",";
  json += "\"uptime\":\"" + getUptime() + "\",";
  json += "\"free_heap\":" + String(ESP.getFreeHeap()) + ",";
  json += "\"total_heap\":" + String(ESP.getHeapSize()) + ",";
  json += "\"mac\":\"" + getMacAddress() + "\"";
  json += "}";
  
  webServer.send(200, "application/json", json);
}

void handleRestart() {
  webServer.send(200, "text/html", 
    "<html><body><h1>Restarting...</h1><p>Device will restart in 3 seconds</p></body></html>");
  delay(3000);
  ESP.restart();
}

void handleNetworkScan() {
  scanNetworks();
  webServer.sendHeader("Location", "/setup");
  webServer.send(302, "text/plain", "");
}

void initTimeSync() {
  configTime(GMT_OFFSET_SEC, DAYLIGHT_OFFSET_SEC, NTP_SERVER);
  Serial.println("Time sync initialized");
}

String getCurrentTime() {
  struct tm timeinfo;
  if (!getLocalTime(&timeinfo)) {
    return "Time not set";
  }
  char timeString[64];
  strftime(timeString, sizeof(timeString), "%Y-%m-%d %H:%M:%S", &timeinfo);
  return String(timeString);
}

String getUptime() {
  unsigned long uptime = millis();
  unsigned long seconds = uptime / 1000;
  unsigned long minutes = seconds / 60;
  unsigned long hours = minutes / 60;
  unsigned long days = hours / 24;
  
  String result = "";
  if (days > 0) result += String(days) + "d ";
  if (hours % 24 > 0) result += String(hours % 24) + "h ";
  if (minutes % 60 > 0) result += String(minutes % 60) + "m ";
  result += String(seconds % 60) + "s";
  
  return result;
}

void initOTA() {
  ArduinoOTA.setHostname("T-Display-S3");
  ArduinoOTA.setPassword("dashboard123");
  
  ArduinoOTA.onStart([]() {
    String type;
    if (ArduinoOTA.getCommand() == U_FLASH) {
      type = "sketch";
    } else {
      type = "filesystem";
    }
    Serial.println("Start updating " + type);
  });
  
  ArduinoOTA.onEnd([]() {
    Serial.println("\nEnd");
  });
  
  ArduinoOTA.onProgress([](unsigned int progress, unsigned int total) {
    Serial.printf("Progress: %u%%\r", (progress / (total / 100)));
  });
  
  ArduinoOTA.onError([](ota_error_t error) {
    Serial.printf("Error[%u]: ", error);
    if (error == OTA_AUTH_ERROR) {
      Serial.println("Auth Failed");
    } else if (error == OTA_BEGIN_ERROR) {
      Serial.println("Begin Failed");
    } else if (error == OTA_CONNECT_ERROR) {
      Serial.println("Connect Failed");
    } else if (error == OTA_RECEIVE_ERROR) {
      Serial.println("Receive Failed");
    } else if (error == OTA_END_ERROR) {
      Serial.println("End Failed");
    }
  });
  
  ArduinoOTA.begin();
  Serial.println("OTA Ready");
}

String getLocalIP() {
  if (currentWiFiStatus == WIFI_CONNECTED) {
    return WiFi.localIP().toString();
  } else if (currentWiFiStatus == WIFI_AP_MODE) {
    return WiFi.softAPIP().toString();
  }
  return "0.0.0.0";
}

String getMacAddress() {
  return WiFi.macAddress();
}

long getRSSI() {
  return WiFi.RSSI();
}

String getNetworkStats() {
  String stats = "RX: " + String(networkInfo.bytesReceived) + " bytes, ";
  stats += "TX: " + String(networkInfo.bytesSent) + " bytes";
  return stats;
}

#endif