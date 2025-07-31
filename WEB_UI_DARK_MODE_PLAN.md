# Dark Mode Web UI Implementation Plan

## 🎯 Priority Features for Phase 1

### 1. **Real-Time Dashboard** (Most Impact)
- Live metrics cards showing:
  - FPS & skip rate
  - CPU usage per core
  - Memory usage with sparkline
  - Temperature & battery
  - WiFi signal strength
- Auto-refresh via polling (WebSocket in Phase 2)

### 2. **Interactive Controls**
- **Brightness Slider**: Real-time adjustment
- **Display Power**: On/Off toggle
- **Performance Mode**: Eco/Normal/Turbo
- **Quick Actions**: Restart device button

### 3. **Modern Dark Theme**
- Pure black background (#0a0a0a)
- Subtle card elevation with shadows
- Blue accent (#3b82f6) for primary actions
- Green/yellow/red status indicators
- Smooth transitions and hover effects

### 4. **Enhanced OTA Page**
- Drag & drop file upload
- Version comparison
- Changelog display
- Rollback option

### 5. **Live Log Viewer**
- Stream telnet logs to web
- Filter by log level
- Search functionality
- Auto-scroll toggle

## 🚀 Implementation Order

### Phase 1: Core Dashboard (1-2 hours)
1. Dark mode CSS framework
2. Metrics dashboard with cards
3. Real-time updates (polling)
4. Interactive controls
5. Mobile responsive layout

### Phase 2: Advanced Features (2-3 hours)
1. WebSocket support
2. Performance charts
3. Log viewer
4. Settings profiles

### Phase 3: Polish (1 hour)
1. Animations & transitions
2. Error handling
3. Offline support
4. PWA manifest

## 🎨 Design Mockup

```
┌─────────────────────────────────────────┐
│ ESP32-S3 Dashboard          [⚙️] [🔄]   │
├─────────────────────────────────────────┤
│ ┌─────────┐ ┌─────────┐ ┌─────────┐   │
│ │   60    │ │  8.3MB  │ │  44°C   │   │
│ │  FPS    │ │  FREE   │ │  TEMP   │   │
│ └─────────┘ └─────────┘ └─────────┘   │
│ ┌─────────┐ ┌─────────┐ ┌─────────┐   │
│ │  0/0%   │ │  100%   │ │  -66    │   │
│ │ CPU 0/1 │ │ BATTERY │ │  RSSI   │   │
│ └─────────┘ └─────────┘ └─────────┘   │
├─────────────────────────────────────────┤
│ Controls                                │
│ Brightness: [═══════──] 70%            │
│ Display: [ON] OFF                      │
│ Mode: ECO [NORMAL] TURBO               │
│ [Restart Device] [Update Firmware]     │
├─────────────────────────────────────────┤
│ Performance (last 60s)                  │
│ [Chart showing FPS over time]          │
├─────────────────────────────────────────┤
│ System Logs                            │
│ [Filter: ALL ▼] [Clear] [Pause]        │
│ ┌─────────────────────────────────┐   │
│ │ [INFO] System running normally   │   │
│ │ [WARN] Temperature above 45°C    │   │
│ └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## 🔧 Technical Approach

### CSS Variables for Dark Theme
```css
:root {
  --bg-main: #0a0a0a;
  --bg-card: #1a1a1a;
  --bg-hover: #2a2a2a;
  --accent: #3b82f6;
  --success: #10b981;
  --warning: #f59e0b;
  --danger: #ef4444;
  --text: #f9fafb;
  --text-dim: #9ca3af;
  --border: #374151;
}
```

### Minimal JavaScript
- Use native fetch() for API calls
- RequestAnimationFrame for smooth updates
- LocalStorage for user preferences
- No heavy frameworks needed

### API Endpoints Needed
- `/api/metrics` - Real-time system metrics
- `/api/control` - Device control commands
- `/api/logs` - Recent log entries
- `/ws` - WebSocket for live updates (Phase 2)

## 📱 Mobile-First Design
- Touch-friendly controls (48px targets)
- Swipeable cards
- Collapsible sections
- Bottom navigation on mobile

## 🎯 Benefits
1. **Professional Look**: Modern dark UI matches IoT aesthetics
2. **Better Monitoring**: See all metrics at a glance
3. **Quick Control**: Adjust settings without navigation
4. **Performance**: Lightweight, fast-loading
5. **Accessibility**: High contrast, clear typography

Ready to start implementation?