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

## Implementation Plan

### Phase 1: Research (Before Any Code Changes)
1. Research each optimization area online
2. Find ESP32-specific examples and benchmarks
3. Identify potential risks and gotchas
4. Update this document with findings

### Phase 2: Baseline Measurements
1. Create performance benchmark suite
2. Measure current metrics for each area
3. Set improvement targets

### Phase 3: Implementation (One at a Time)
1. Implement one optimization
2. Test thoroughly
3. Measure improvement
4. Document results
5. Only proceed to next if stable

### Phase 4: Integration Testing
1. Test all optimizations together
2. Check for interactions/conflicts
3. Final performance validation

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

## Next Steps - Start with Dirty Rectangle Tracking

### Step 1: Create Performance Benchmark
Before implementing any optimization, we need baseline measurements:
1. Add performance counters to DisplayManager
2. Track metrics per frame:
   - Total pixels written
   - Number of draw calls
   - Time spent in each drawing function
   - Time spent in flush()
3. Log averages every second

### Step 2: Implement Dirty Rectangle Tracking
1. Enhance existing DirtyRect structure
2. Add fixed-size array for tracking multiple rectangles
3. Implement merge algorithm
4. Modify flush() to only update dirty regions
5. Test with various UI scenarios

### Step 3: Measure and Validate
1. Compare metrics before/after implementation
2. Verify display correctness
3. Check for visual artifacts
4. Document performance improvement

Only proceed to PSRAM frame buffer after dirty rectangles are stable and showing measurable improvement.

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

### In Progress
- ⏸️ **Nothing currently in progress**

### Next Up
- 🔄 **Dual-Core Architecture Optimization** - Maximize ESP32-S3 dual-core potential
  
  **Current State Analysis:**
  - Core 0: Overloaded (UI, sensors, network, display)
  - Core 1: Severely underutilized (just waiting for work items)
  - Result: Wasted processing power and potential responsiveness issues
  
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

### Completed (continued)
4. ❌ **PSRAM Frame Buffer** - Implemented but causes severe performance degradation
   - Created `PsramFrameBuffer` with dual 16-byte aligned buffers in PSRAM
   - Implements block-based dirty detection (16x16 blocks)
   - Automatic region merging for efficient updates
   - **Critical Issue**: Reduces performance from 55 FPS to 1.9 FPS (96% slower!)
   - **Root Cause**: Sending 50,400 pixels individually takes ~537ms per frame
   - **Status**: Disabled in v5.15-fb-off to restore normal performance
   - **Lesson Learned**: GPIO bit-banging cannot handle full frame buffer updates
   - **Future**: Requires hardware acceleration (LCD_CAM) or significant optimization

### Pending
- ⏳ Remove Simulated Sensor Data
- ⏳ Display Driver Optimization
- ⏳ Power Management
- ⏳ Compiler Optimizations