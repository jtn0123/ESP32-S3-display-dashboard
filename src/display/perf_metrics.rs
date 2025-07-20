use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Performance metrics for display operations
pub struct DisplayMetrics {
    // Frame metrics
    frame_count: AtomicU32,
    frame_start: Option<Instant>,
    
    // Pixel metrics
    pixels_written: AtomicUsize,
    
    // Draw operation metrics
    draw_calls: AtomicU32,
    fill_rect_calls: AtomicU32,
    draw_text_calls: AtomicU32,
    clear_calls: AtomicU32,
    
    // Timing metrics (in microseconds)
    total_draw_time_us: AtomicUsize,
    total_flush_time_us: AtomicUsize,
    total_clear_time_us: AtomicUsize,
    
    // Dirty rectangle metrics
    dirty_rect_count: AtomicU32,
    dirty_rect_merges: AtomicU32,
    
    // Report timing
    last_report: Instant,
    report_interval: Duration,
}

impl DisplayMetrics {
    pub fn new() -> Self {
        Self {
            frame_count: AtomicU32::new(0),
            frame_start: None,
            pixels_written: AtomicUsize::new(0),
            draw_calls: AtomicU32::new(0),
            fill_rect_calls: AtomicU32::new(0),
            draw_text_calls: AtomicU32::new(0),
            clear_calls: AtomicU32::new(0),
            total_draw_time_us: AtomicUsize::new(0),
            total_flush_time_us: AtomicUsize::new(0),
            total_clear_time_us: AtomicUsize::new(0),
            dirty_rect_count: AtomicU32::new(0),
            dirty_rect_merges: AtomicU32::new(0),
            last_report: Instant::now(),
            report_interval: Duration::from_secs(1),
        }
    }
    
    /// Start a new frame
    pub fn start_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }
    
    /// End current frame and update metrics
    pub fn end_frame(&mut self) {
        if let Some(_start) = self.frame_start {
            self.frame_count.fetch_add(1, Ordering::Relaxed);
            self.frame_start = None;
        }
        
        // Check if we should report
        if self.last_report.elapsed() >= self.report_interval {
            self.report();
            self.reset();
            self.last_report = Instant::now();
        }
    }
    
    /// Record pixels written
    pub fn add_pixels_written(&self, count: u32) {
        self.pixels_written.fetch_add(count as usize, Ordering::Relaxed);
    }
    
    /// Record a draw call
    pub fn add_draw_call(&self) {
        self.draw_calls.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a fill_rect call
    pub fn add_fill_rect(&self, width: u16, height: u16) {
        self.fill_rect_calls.fetch_add(1, Ordering::Relaxed);
        self.add_pixels_written((width as u32) * (height as u32));
    }
    
    /// Record a draw_text call
    pub fn add_draw_text(&self, char_count: usize) {
        self.draw_text_calls.fetch_add(1, Ordering::Relaxed);
        self.add_draw_call();
        // Approximate pixels for text (5x7 font with scale)
        self.add_pixels_written((char_count * 35) as u32);
    }
    
    /// Record a clear call
    pub fn add_clear(&self, width: u16, height: u16) {
        self.clear_calls.fetch_add(1, Ordering::Relaxed);
        self.add_pixels_written((width as u32) * (height as u32));
    }
    
    /// Record timing for an operation
    pub fn add_timing(&self, operation: &str, duration: Duration) {
        let us = duration.as_micros() as usize;
        match operation {
            "draw" => self.total_draw_time_us.fetch_add(us, Ordering::Relaxed),
            "flush" => self.total_flush_time_us.fetch_add(us, Ordering::Relaxed),
            "clear" => self.total_clear_time_us.fetch_add(us, Ordering::Relaxed),
            _ => 0,
        };
    }
    
    /// Record dirty rectangle activity
    pub fn add_dirty_rect(&self) {
        self.dirty_rect_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record dirty rectangle merge
    pub fn add_dirty_merge(&self) {
        self.dirty_rect_merges.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Generate performance report
    fn report(&self) {
        let frames = self.frame_count.load(Ordering::Relaxed);
        if frames == 0 {
            return;
        }
        
        let pixels = self.pixels_written.load(Ordering::Relaxed);
        let draw_calls = self.draw_calls.load(Ordering::Relaxed);
        let fill_rects = self.fill_rect_calls.load(Ordering::Relaxed);
        let draw_texts = self.draw_text_calls.load(Ordering::Relaxed);
        let clears = self.clear_calls.load(Ordering::Relaxed);
        let dirty_rects = self.dirty_rect_count.load(Ordering::Relaxed);
        let merges = self.dirty_rect_merges.load(Ordering::Relaxed);
        
        let draw_time_ms = self.total_draw_time_us.load(Ordering::Relaxed) as f32 / 1000.0;
        let flush_time_ms = self.total_flush_time_us.load(Ordering::Relaxed) as f32 / 1000.0;
        let clear_time_ms = self.total_clear_time_us.load(Ordering::Relaxed) as f32 / 1000.0;
        let total_time_ms = draw_time_ms + flush_time_ms + clear_time_ms;
        
        let pixels_per_frame = if frames > 0 { pixels / frames as usize } else { 0 };
        let draw_calls_per_frame = if frames > 0 { draw_calls as f32 / frames as f32 } else { 0.0 };
        
        log::info!("[DISPLAY PERF] {} frames | {:.1} pixels/frame | {:.1} draw calls/frame", 
                  frames, pixels_per_frame, draw_calls_per_frame);
        log::info!("[DISPLAY OPS] fill_rect: {} | draw_text: {} | clear: {} | dirty: {} (merged: {})",
                  fill_rects, draw_texts, clears, dirty_rects, merges);
        log::info!("[DISPLAY TIME] total: {:.1}ms | draw: {:.1}ms | flush: {:.1}ms | clear: {:.1}ms",
                  total_time_ms, draw_time_ms, flush_time_ms, clear_time_ms);
        
        // Calculate efficiency metrics
        if pixels > 0 {
            let pixels_per_ms = pixels as f32 / total_time_ms;
            log::info!("[DISPLAY EFF] {:.0} pixels/ms | {:.2}ms per 1K pixels",
                      pixels_per_ms, 1000.0 / pixels_per_ms);
        }
    }
    
    /// Reset all metrics
    fn reset(&self) {
        self.frame_count.store(0, Ordering::Relaxed);
        self.pixels_written.store(0, Ordering::Relaxed);
        self.draw_calls.store(0, Ordering::Relaxed);
        self.fill_rect_calls.store(0, Ordering::Relaxed);
        self.draw_text_calls.store(0, Ordering::Relaxed);
        self.clear_calls.store(0, Ordering::Relaxed);
        self.total_draw_time_us.store(0, Ordering::Relaxed);
        self.total_flush_time_us.store(0, Ordering::Relaxed);
        self.total_clear_time_us.store(0, Ordering::Relaxed);
        self.dirty_rect_count.store(0, Ordering::Relaxed);
        self.dirty_rect_merges.store(0, Ordering::Relaxed);
    }
}

/// Helper macro for timing operations
#[macro_export]
macro_rules! time_operation {
    ($metrics:expr, $op_name:expr, $block:block) => {{
        let start = std::time::Instant::now();
        let result = $block;
        $metrics.add_timing($op_name, start.elapsed());
        result
    }};
}