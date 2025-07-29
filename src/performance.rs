// Performance metrics and FPS tracking
use std::time::{Duration, Instant};

/// Accurate FPS counter that tracks actual display updates
pub struct FpsTracker {
    // Frame timing
    frame_times: Vec<Duration>,
    max_samples: usize,
    last_frame_time: Instant,
    last_rendered_frame_time: Instant,
    
    // Statistics
    current_fps: f32,
    average_fps: f32,
    min_fps: f32,
    max_fps: f32,
    
    // Frame counting
    total_frames: u64,
    skipped_frames: u64,
    
    // Update tracking
    last_update: Instant,
    update_interval: Duration,
}

impl FpsTracker {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_times: Vec::with_capacity(60), // Keep last 60 frames
            max_samples: 60,
            last_frame_time: now,
            last_rendered_frame_time: now,
            current_fps: 0.0,
            average_fps: 0.0,
            min_fps: f32::MAX,
            max_fps: 0.0,
            total_frames: 0,
            skipped_frames: 0,
            last_update: now,
            update_interval: Duration::from_millis(250), // Update stats 4x per second
        }
    }
    
    /// Call at the start of each frame
    pub fn frame_start(&mut self) -> FrameTimer {
        FrameTimer {
            start_time: Instant::now(),
            tracker: self as *mut Self,
        }
    }
    
    /// Call when a frame is skipped (no actual display update)
    pub fn frame_skipped(&mut self) {
        self.skipped_frames += 1;
    }
    
    /// Call when a frame is actually rendered (and flushed to display)
    pub fn frame_rendered(&mut self, _frame_time: Duration) {
        let now = Instant::now();
        
        // Calculate time since last rendered frame
        let time_since_last = now.duration_since(self.last_rendered_frame_time);
        self.last_rendered_frame_time = now;
        
        self.total_frames += 1;
        
        // Only store frame timing if this isn't the first frame
        if self.total_frames > 1 {
            // Store time between frames, not frame render time
            if self.frame_times.len() >= self.max_samples {
                self.frame_times.remove(0);
            }
            self.frame_times.push(time_since_last);
        }
        
        // Update statistics if interval elapsed
        if self.last_update.elapsed() >= self.update_interval {
            self.update_statistics();
            self.last_update = Instant::now();
        }
    }
    
    /// Internal: record frame completion
    fn frame_end(&mut self, frame_time: Duration) {
        self.total_frames += 1;
        
        // Store frame time
        if self.frame_times.len() >= self.max_samples {
            self.frame_times.remove(0);
        }
        self.frame_times.push(frame_time);
        
        // Update statistics if interval elapsed
        if self.last_update.elapsed() >= self.update_interval {
            self.update_statistics();
            self.last_update = Instant::now();
        }
    }
    
    /// Calculate current statistics
    fn update_statistics(&mut self) {
        if self.frame_times.is_empty() {
            return;
        }
        
        // Calculate average frame time
        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time / self.frame_times.len() as u32;
        
        // Calculate FPS from average frame time
        self.average_fps = if avg_frame_time.as_secs_f32() > 0.0 {
            1.0 / avg_frame_time.as_secs_f32()
        } else {
            0.0
        };
        
        // Calculate current FPS from recent frames (last 10)
        let recent_count = self.frame_times.len().min(10);
        if recent_count > 0 {
            let recent_time: Duration = self.frame_times[self.frame_times.len() - recent_count..]
                .iter()
                .sum();
            let recent_avg = recent_time / recent_count as u32;
            self.current_fps = if recent_avg.as_secs_f32() > 0.0 {
                1.0 / recent_avg.as_secs_f32()
            } else {
                0.0
            };
            
            // Debug check for unrealistic FPS
            if self.current_fps > 100.0 {
                log::warn!("[FPS] Unrealistic FPS calculated: {:.1} from avg frame time: {:.3}ms", 
                    self.current_fps, recent_avg.as_secs_f32() * 1000.0);
                // Cap at a reasonable maximum
                self.current_fps = self.current_fps.min(60.0);
            }
        }
        
        // Update min/max
        self.min_fps = self.min_fps.min(self.current_fps);
        self.max_fps = self.max_fps.max(self.current_fps);
    }
    
    /// Get current FPS
    pub fn current_fps(&self) -> f32 {
        // If we have no data yet, return 0.0 to indicate no frames
        if self.frame_times.is_empty() && self.total_frames == 0 {
            return 0.0;
        }
        self.current_fps
    }
    
    /// Get average FPS over the sample window
    pub fn average_fps(&self) -> f32 {
        self.average_fps
    }
    
    /// Get minimum recorded FPS
    pub fn min_fps(&self) -> f32 {
        if self.min_fps == f32::MAX {
            0.0
        } else {
            self.min_fps
        }
    }
    
    /// Get maximum recorded FPS
    pub fn max_fps(&self) -> f32 {
        self.max_fps
    }
    
    /// Get frame statistics
    pub fn stats(&self) -> FpsStats {
        FpsStats {
            current_fps: self.current_fps,
            average_fps: self.average_fps,
            min_fps: self.min_fps(),
            max_fps: self.max_fps,
            total_frames: self.total_frames,
            skipped_frames: self.skipped_frames,
            skip_rate: if self.total_frames > 0 {
                (self.skipped_frames as f32 / self.total_frames as f32) * 100.0
            } else {
                0.0
            },
        }
    }
    
    /// Reset all statistics
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.current_fps = 0.0;
        self.average_fps = 0.0;
        self.min_fps = f32::MAX;
        self.max_fps = 0.0;
        self.total_frames = 0;
        self.skipped_frames = 0;
        self.last_update = Instant::now();
    }
}

/// RAII frame timer that automatically records frame time when dropped
pub struct FrameTimer {
    start_time: Instant,
    tracker: *mut FpsTracker,
}

impl Drop for FrameTimer {
    fn drop(&mut self) {
        let frame_time = self.start_time.elapsed();
        unsafe {
            if let Some(tracker) = self.tracker.as_mut() {
                tracker.frame_end(frame_time);
            }
        }
    }
}

/// FPS statistics
#[derive(Debug, Clone)]
pub struct FpsStats {
    pub current_fps: f32,
    pub average_fps: f32,
    pub min_fps: f32,
    pub max_fps: f32,
    pub total_frames: u64,
    pub skipped_frames: u64,
    pub skip_rate: f32, // Percentage of frames skipped
}

/// Performance metrics beyond just FPS
pub struct PerformanceMetrics {
    pub fps_tracker: FpsTracker,
    
    // Timing breakdown
    pub last_render_time: Duration,
    pub last_flush_time: Duration,
    pub last_sensor_time: Duration,
    pub last_network_time: Duration,
    
    // Memory stats
    pub heap_free: usize,
    pub heap_largest_block: usize,
    pub psram_free: usize,
    
    // Update tracking
    last_memory_update: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            fps_tracker: FpsTracker::new(),
            last_render_time: Duration::ZERO,
            last_flush_time: Duration::ZERO,
            last_sensor_time: Duration::ZERO,
            last_network_time: Duration::ZERO,
            heap_free: 0,
            heap_largest_block: 0,
            psram_free: 0,
            last_memory_update: Instant::now(),
        }
    }
    
    /// Update memory statistics (call sparingly, e.g., once per second)
    pub fn update_memory_stats(&mut self) {
        if self.last_memory_update.elapsed() < Duration::from_secs(1) {
            return;
        }
        
        unsafe {
            self.heap_free = esp_idf_sys::esp_get_free_heap_size() as usize;
            self.heap_largest_block = esp_idf_sys::heap_caps_get_largest_free_block(
                esp_idf_sys::MALLOC_CAP_DEFAULT
            ) as usize;
        }
        
        if crate::psram::PsramAllocator::is_available() {
            self.psram_free = crate::psram::PsramAllocator::get_free_size();
        }
        
        self.last_memory_update = Instant::now();
    }
    
    /// Record render time
    pub fn record_render_time(&mut self, duration: Duration) {
        self.last_render_time = duration;
    }
    
    /// Record flush time
    pub fn record_flush_time(&mut self, duration: Duration) {
        self.last_flush_time = duration;
    }
    
    /// Record sensor update time
    pub fn record_sensor_time(&mut self, duration: Duration) {
        self.last_sensor_time = duration;
    }
    
    /// Record network operation time
    pub fn record_network_time(&mut self, duration: Duration) {
        self.last_network_time = duration;
    }
    
    /// Get a summary of current performance
    pub fn summary(&self) -> String {
        let fps_stats = self.fps_tracker.stats();
        format!(
            "FPS: {:.1} (avg: {:.1}, min: {:.1}, max: {:.1}) | Skip: {:.1}% | Render: {:.1}ms | Flush: {:.1}ms",
            fps_stats.current_fps,
            fps_stats.average_fps,
            fps_stats.min_fps,
            fps_stats.max_fps,
            fps_stats.skip_rate,
            self.last_render_time.as_secs_f32() * 1000.0,
            self.last_flush_time.as_secs_f32() * 1000.0
        )
    }
}