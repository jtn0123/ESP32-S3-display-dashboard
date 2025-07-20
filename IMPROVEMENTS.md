# ESP32-S3 Display Dashboard Performance Improvements

## Overview
This document outlines performance improvement opportunities for the ESP32-S3 Display Dashboard, excluding the known LCD_CAM hardware acceleration issue. Each improvement includes research findings, implementation approach, and verification methods.

## Current Performance Baseline
- **Display**: 10 FPS (GPIO bit-banging limitation)
- **CPU**: 240MHz max, DFS enabled (80-240MHz)
- **Memory**: ~300KB free heap, 2MB PSRAM available but underutilized
- **Power**: No sleep modes, constant full power

## Improvement Areas

### 1. Display Rendering Optimizations
**Goal**: Maximize efficiency within the 10 FPS hardware constraint

#### 1.1 Dirty Rectangle Tracking
- **Current**: Basic implementation exists but not fully utilized
- **Research Findings**: 
  - Dirty rectangles only redraw changed screen portions, saving CPU cycles
  - Common approach: Store rectangles in std::vector, merge overlapping regions
  - For embedded: Use fixed-size array to avoid dynamic allocation
  - Coalesce rectangles when count gets high to reduce draw calls
  - Libraries like LovyanGFX and TFT_eSPI implement this effectively
- **Implementation Approach**:
  1. Track up to 16 dirty rectangles in fixed array
  2. Merge overlapping/adjacent rectangles
  3. If >10 rectangles, merge all into bounding box
  4. Only send changed regions to display
- **Verification**: Measure time spent in flush() before/after

#### 1.2 Frame Buffer with Differential Updates
- **Current**: Frame buffer exists but disabled
- **Research Findings**:
  - PSRAM ideal for frame buffers but needs cache management
  - ESP32-S3: CPU uses cache, DMA bypasses it - causes coherency issues
  - Solution: Use Cache_WriteBack_Addr() after writing to PSRAM
  - LovyanGFX uses "Sprite" class for off-screen buffers
  - Double buffering prevents tearing, enables differential updates
- **Implementation Approach**:
  1. Allocate 2 frame buffers in PSRAM (300x168x2 bytes each = ~100KB)
  2. Compare buffers pixel-by-pixel or in blocks
  3. Send only changed blocks to display
  4. Always flush cache after PSRAM writes
- **Verification**: Logic analyzer to confirm reduced data transfers

#### 1.3 Drawing Operation Batching
- **Current**: Each draw operation immediately writes to display
- **Research Needed**: Optimal batch sizes for ST7789
- **Implementation**: Queue operations, flush in batches
- **Verification**: Profile draw call overhead reduction

### 2. Memory Optimization
**Goal**: Reduce allocations and utilize PSRAM effectively

#### 2.1 PSRAM Frame Buffer
- **Current**: 2MB PSRAM available but unused for display
- **Research Findings**:
  - Use heap_caps_malloc(size, MALLOC_CAP_SPIRAM) for PSRAM allocation
  - In Rust: Available via esp_idf_sys bindings
  - PSRAM access slower than internal RAM but good for large buffers
  - Cache coherency critical: esp_idf_sys::Cache_WriteBack_Addr()
  - Alignment matters: 16-byte alignment improves DMA performance
- **Implementation Approach**:
  1. Use heap_caps_malloc with MALLOC_CAP_SPIRAM flag
  2. Ensure 16-byte alignment for DMA efficiency
  3. Implement cache flush wrapper for safety
  4. Create PSRAM allocator module for reuse
- **Verification**: Monitor PSRAM usage stats

#### 2.2 Pre-allocated Buffers
- **Current**: Repeated allocations in comprehensive_memory_init()
- **Research Needed**: Static vs dynamic allocation trade-offs
- **Implementation**: Create buffer pool for common operations
- **Verification**: Heap fragmentation analysis

### 3. Task Scheduling & CPU Usage
**Goal**: Better utilize dual-core architecture

#### 3.1 Sensor Sampling on Core 1
- **Current**: All tasks on Core 0
- **Research Findings**:
  - ESP32-S3 has two cores: CPU0 (PRO_CPU) and CPU1 (APP_CPU)
  - Use xTaskCreatePinnedToCore() to pin tasks to specific core
  - Core 0 typically handles WiFi/BT, Core 1 for user apps
  - In Rust: Access via unsafe FFI with esp_idf_sys
  - Task affinity improves determinism and load distribution
- **Implementation Approach**:
  1. Create sensor task with xTaskCreatePinnedToCore(..., 1)
  2. Use unsafe extern "C" functions for task entry
  3. Move sensor sampling logic to dedicated task
  4. Use FreeRTOS queues for inter-core communication
- **Verification**: Core usage statistics

#### 3.2 Main Loop Rate Optimization
- **Current**: 60 FPS cap but display only does 10 FPS
- **Research Needed**: Optimal update rates for UI responsiveness
- **Implementation**: Adaptive frame rate based on activity
- **Verification**: Power consumption measurements

### 4. Network Performance
**Goal**: Reduce WiFi overhead and improve reliability

#### 4.1 Background Signal Monitoring
- **Current**: Signal strength only checked on connect
- **Research Needed**: ESP-IDF WiFi event callbacks
- **Implementation**: Periodic background RSSI updates
- **Verification**: Network stability metrics

### 5. Power Management
**Goal**: Reduce power consumption during idle

#### 5.1 Dynamic Frequency Scaling Enhancement
- **Current**: Basic DFS 80-240MHz
- **Research Findings**:
  - esp_pm_configure() enables DFS and auto light-sleep
  - Set light_sleep_enable=true in esp_pm_config_t
  - Power management adds 0.2-40μs interrupt latency
  - Use power locks to prevent sleep during critical ops
  - CPU auto-scales based on load when configured
- **Implementation Approach**:
  1. Enable light_sleep_enable in PM config
  2. Use PM locks during display updates
  3. Lower min_freq_mhz to 40MHz for deeper power savings
  4. Monitor wake latency impact on responsiveness
- **Verification**: Current consumption measurements

#### 5.2 Light Sleep During Idle
- **Current**: No sleep modes used
- **Research Findings**:
  - Light sleep: ~0.8mA vs active ~40-80mA
  - Wake latency: <1ms
  - Peripherals clock-gated, context preserved
  - Display will be blank during sleep (peripherals stopped)
  - Auto light-sleep uses FreeRTOS tickless idle
  - CONFIG_FREERTOS_IDLE_TIME_BEFORE_SLEEP sets threshold
- **Implementation Approach**:
  1. Enable CONFIG_FREERTOS_USE_TICKLESS_IDLE
  2. Set idle threshold to 50ms (5 display frames)
  3. Use PM locks to keep awake during user interaction
  4. Wake on button GPIO interrupts
- **Verification**: Power profiler measurements

### 6. Code-Level Optimizations
**Goal**: Reduce CPU cycles in hot paths

#### 6.1 Direct Register Access for GPIO
- **Current**: Using HAL layer for pin operations
- **Research Needed**: Safe direct GPIO register manipulation
- **Implementation**: Replace HAL calls in write_byte()
- **Verification**: Oscilloscope timing measurements

#### 6.2 Aggressive Inlining
- **Current**: Some functions marked inline
- **Research Needed**: Profile-guided optimization
- **Implementation**: Inline all hot-path functions
- **Verification**: Binary size vs performance trade-off

## Key Metrics to Track

### Performance Metrics
- **Core Utilization**: Monitor both Core 0 and Core 1 usage percentages
- **Task Latency**: Measure sensor update delays and UI responsiveness
- **Memory**: Heap fragmentation, allocation counts, free heap trends
- **Power**: Current draw in different states, sleep time percentage
- **Network**: Reconnection count, average RSSI, packet loss

### Success Criteria
- Core 0 CPU usage < 50% (from ~100%)
- Core 1 CPU usage 20-30% (from ~0%)
- UI responsiveness < 50ms for button presses
- No increase in crash rate or instability
- Measurable reduction in power consumption during idle

## Verification Methods

### Performance Metrics
- **Display**: Frame time, draw call count, pixels written
- **Memory**: Heap usage, PSRAM usage, allocation count
- **CPU**: Core utilization, task timing, interrupt latency
- **Power**: Current draw, sleep time percentage

### Test Scenarios
1. **Idle**: Display showing static content
2. **Active**: Continuous UI updates
3. **Network**: Active WiFi transfers
4. **Stress**: All features active simultaneously

## Risk Mitigation
- Keep original code paths with feature flags
- Implement gradual rollout
- Monitor crash reports and stability
- Have rollback plan for each optimization

## Success Criteria
- No reduction in stability
- Measurable performance improvement (>10%)
- Reduced power consumption
- Maintained code readability

## Research Summary
All research has been completed for the major optimization areas. Key findings include:
- Dirty rectangle tracking can significantly reduce display updates
- PSRAM requires cache management but is ideal for frame buffers
- Dual-core usage requires unsafe FFI but can offload work effectively
- Light sleep can reduce power by 50x but blanks the display
- Most optimizations are well-supported in ESP-IDF with Rust bindings available

## Implementation Priority (Based on Impact/Risk Analysis)

### High Priority (High Impact, Low Risk)
1. **Dirty Rectangle Tracking** - Can reduce display writes by 50-90%
2. **PSRAM Frame Buffer** - Frees internal RAM, enables advanced features
3. **Pre-allocated Buffers** - Reduces heap fragmentation

### Medium Priority (Medium Impact, Medium Risk)
4. **Sensor Task on Core 1** - Better CPU utilization
5. **Main Loop Rate Optimization** - Power savings
6. **Drawing Operation Batching** - Reduced overhead

### Low Priority (Lower Impact or Higher Risk)
7. **Light Sleep Mode** - High power savings but blanks display
8. **Direct GPIO Register Access** - Minor performance gain, higher risk
9. **Aggressive Inlining** - Marginal gains

## Implementation Strategy

### Phase 1: Dual-Core Architecture (Highest Impact)
The ESP32-S3's second core is completely idle while Core 0 handles everything. This is the biggest optimization opportunity:

1. **Create Core 1 Task Infrastructure**
   - Set up FreeRTOS task pinned to Core 1
   - Implement thread-safe channels for inter-core communication
   - Create event aggregation to reduce cross-core overhead

2. **Move Sensor Monitoring to Core 1**
   - Temperature sensor (5s interval)
   - Battery monitoring (30s interval)
   - CPU stats (2s interval)
   - Implement circular buffers for historical data

3. **Add Network Monitor on Core 1**
   - WiFi RSSI monitoring (10s interval)
   - Connection state management
   - Auto-reconnection logic
   - mDNS service broadcasting

4. **Implement Data Processing Pipeline**
   - Noise filtering (Kalman filters)
   - Moving averages
   - Trend detection
   - Threshold-based alerts

### Phase 2: Memory Optimization
1. **Pre-allocated Buffer Pool**
   - Identify common allocation patterns
   - Create fixed-size buffer pools
   - Implement buffer recycling

### Phase 3: Power Optimization
1. **Dynamic Power Management**
   - Enable light sleep during idle
   - Implement activity-based CPU scaling
   - Add wake-on-button interrupts

## Progress Tracking

### Completed
1. ✅ **Performance Baseline** - Captured ~55 FPS main loop rate via serial monitoring
2. ✅ **Dirty Rectangle Tracking** - Implemented enhanced multi-rectangle manager with automatic merging
   - Created `DirtyRectManager` supporting up to 16 rectangles
   - Automatic merging of adjacent/overlapping regions
   - Integrated into all draw operations
   - Version: v5.09-dirty-rect
3. ✅ **FPS Cap Toggle** - Made 60 FPS cap a compile-time toggle (`ENABLE_FPS_CAP`)
   - Currently disabled for performance benchmarking
   - Can be re-enabled for production builds
4. ✅ **FPS Counter Accuracy** - Fixed dual FPS calculation issue
   - Created `PerformanceMetrics` module with accurate `FpsTracker`
   - Integrated frame skip detection in UI render loop
   - UI `render()` now returns boolean to indicate if frame was rendered
   - Main loop tracks skipped frames separately from rendered frames
   - Version: v5.16-fps-fix
5. ✅ **Telnet Server** - Added remote monitoring capability (port 23)
   - Enables wireless log monitoring without USB connection
   - Scripts: `monitor-telnet.sh` and `monitor-telnet.py` for client access
   - Version: v5.17-telnet
6. ✅ **OTA Updates** - Verified working Over-The-Air update functionality
   - HTTP endpoint at `/ota/update` accepts firmware uploads
   - Partition table supports dual OTA app partitions (ota_0, ota_1)
   - Script `scripts/ota.sh` provides easy device discovery and updates
   - Tested and confirmed working on 2025-07-20
7. ✅ **Dual-Core Architecture** - Implemented Core 1 background tasks
   - Created Core 1 task infrastructure with FreeRTOS pinning
   - Sensor monitoring task runs on Core 1 (temperature, battery, CPU)
   - Network monitoring task tracks WiFi health on Core 1
   - Data processing pipeline with filtering and trend analysis
   - Inter-core communication via channels
   - Version: v5.18-core1

### In Progress
- ⏸️ **Nothing currently in progress**

### Priority Queue (Ordered by Impact/Feasibility)
1. 📊 **Remove Simulated Sensor Data** - Replace fake data with real monitoring
   - Currently using simulated data for temperature and battery
   - Temperature sensor is partially implemented (internal sensor)
   - Battery monitoring needs ADC API fixes
   - CPU usage monitoring needs proper implementation
  
  **Proposed Architecture:**
  - **Core 0 (PRO_CPU)**: UI & Display Core
    - Main UI render loop
    - Display driver operations
    - Touch/button event handling
    - Immediate user feedback
    - Watchdog management
  
  - **Core 1 (APP_CPU)**: Background Processing Core
    - Continuous sensor monitoring
    - Network status monitoring
    - Data processing & filtering
    - OTA update checking
    - Telemetry collection
    - Data persistence
  
  **Optimization Components:**
  
  1. **Sensor Management Service (Core 1)**
     - Continuous monitoring: Temperature (5s), Battery (30s), RSSI (10s), CPU (2s)
     - Circular buffer for historical data
     - Moving averages and trend detection
     - Channel-based updates to Core 0 only on significant changes
  
  2. **Network Monitor Service (Core 1)**
     - Continuous WiFi signal strength monitoring
     - Connection state management & auto-reconnection
     - Network performance metrics
     - mDNS service broadcasting
  
  3. **Data Processing Pipeline (Core 1)**
     - Noise filtering (Kalman filters)
     - Anomaly detection
     - Trend calculation
     - Alert generation
  
  4. **Display Preparation Service (Core 1)**
     - Graph data point calculation
     - Text layout computation
     - Animation frame preparation
     - Asset management
  
  5. **System Health Monitor (Core 1)**
     - Memory usage tracking
     - Task stack monitoring
     - CPU temperature
     - Flash wear stats
  
  6. **Event Aggregator Pattern**
     - Smart event batching to reduce cross-core communication
     - Batch events every 100ms or on critical events
  
  **Implementation Phases:**
  - Phase 1: Sensor Service (biggest immediate impact)
  - Phase 2: Network Monitor (improves reliability)
  - Phase 3: Data Processing (enhances UX)
  - Phase 4: Display Optimization (future performance)
  
  **Expected Benefits:**
  - Core 0 CPU usage: <50% (from ~100%)
  - Core 1 CPU usage: 20-30% (from ~0%)
  - Improved UI responsiveness
  - Better sensor data quality
  - Enhanced system reliability

### Failed Attempts (Do Not Retry)
1. ❌ **PSRAM Frame Buffer** - Implemented but causes severe performance degradation
   - Created `PsramFrameBuffer` with dual 16-byte aligned buffers in PSRAM
   - Implements block-based dirty detection (16x16 blocks)
   - Automatic region merging for efficient updates
   - **Critical Issue**: Reduces performance from 55 FPS to 1.9 FPS (96% slower!)
   - **Root Cause**: Sending 50,400 pixels individually takes ~537ms per frame
   - **Status**: Disabled in v5.15-fb-off to restore normal performance
   - **Lesson Learned**: GPIO bit-banging cannot handle full frame buffer updates
   - **Future**: Requires hardware acceleration (LCD_CAM) or significant optimization

2. 📊 **Remove Simulated Sensor Data** - Replace fake data with real monitoring
   - Currently using simulated data for temperature and battery
   - Implement proper sensor reading intervals
   - Add data filtering and moving averages
   
3. 🌐 **Network Monitor Service (Core 1)** - Continuous WiFi health monitoring
   - Currently only checks RSSI on connect
   - Move to Core 1 for background monitoring
   - Implement auto-reconnection logic
   - Track network performance metrics
   
4. 💾 **Pre-allocated Buffer Pool** - Reduce heap fragmentation
   - Many repeated allocations in memory init
   - Create reusable buffer pool
   - Monitor heap health metrics
   
5. 🔋 **Power Management** - Reduce idle power consumption
   - Enable light sleep mode (display will blank)
   - Lower minimum CPU frequency to 40MHz
   - Use PM locks during critical operations
   - Potential 50x power reduction when idle

### Low Priority / Future Considerations
- ⏳ Direct GPIO Register Access - Minor performance gain, higher risk
- ⏳ Aggressive Function Inlining - Marginal improvements
- ⏳ Display Driver Micro-optimizations - Already near hardware limits