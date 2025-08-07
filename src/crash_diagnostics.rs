/// Advanced crash diagnostics module
use esp_idf_sys::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use log::{error, warn, info};

/// Enable diagnostic mode
static DIAGNOSTIC_MODE: AtomicBool = AtomicBool::new(true); // Always on for debugging

/// Crash detection
static CRASH_IMMINENT: AtomicBool = AtomicBool::new(false);
static LAST_CHECKPOINT: Mutex<Option<String>> = Mutex::new(None);

/// HTTP request tracking
static ACTIVE_REQUESTS: AtomicU32 = AtomicU32::new(0);
static REQUEST_HISTORY: Mutex<Vec<RequestTrace>> = Mutex::new(Vec::new());

#[derive(Clone)]
struct RequestTrace {
    id: u32,
    path: String,
    start_time: Instant,
    end_time: Option<Instant>,
    status: RequestStatus,
    memory_before: u32,
    memory_after: Option<u32>,
    _stack_before: u32,
}

#[derive(Clone)]
enum RequestStatus {
    Active,
    Completed(u16), // HTTP status code
    Failed(String),
}

/// Set a checkpoint for crash debugging
pub fn checkpoint(location: &str) {
    if DIAGNOSTIC_MODE.load(Ordering::Relaxed) {
        if let Ok(mut last) = LAST_CHECKPOINT.lock() {
            *last = Some(format!("{} @ {:?}", location, Instant::now()));
        }
        
        // Check for critical conditions
        unsafe {
            let heap = heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32);
            let stack = uxTaskGetStackHighWaterMark(std::ptr::null_mut());
            
            if heap < 10_000 || stack < 512 {
                error!("CRITICAL at {}: heap={}, stack={}", location, heap, stack);
                CRASH_IMMINENT.store(true, Ordering::SeqCst);
                dump_diagnostics();
            }
        }
    }
}

/// Track HTTP request lifecycle
pub struct RequestTracker {
    id: u32,
    path: String,
    start: Instant,
    memory_before: u32,
    _stack_before: u32,
}

impl RequestTracker {
    pub fn new(path: &str) -> Self {
        let id = ACTIVE_REQUESTS.fetch_add(1, Ordering::SeqCst);
        
        let (memory_before, _stack_before) = unsafe {
            (
                heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32) as u32,
                uxTaskGetStackHighWaterMark(std::ptr::null_mut()),
            )
        };
        
        if DIAGNOSTIC_MODE.load(Ordering::Relaxed) {
            info!("REQ[{}] START: {} (heap={}, stack={})", 
                id, path, memory_before, _stack_before);
        }
        
        // Store in history
        if let Ok(mut history) = REQUEST_HISTORY.lock() {
            history.push(RequestTrace {
                id,
                path: path.to_string(),
                start_time: Instant::now(),
                end_time: None,
                status: RequestStatus::Active,
                memory_before,
                memory_after: None,
                _stack_before,
            });
            
            // Keep only last 10 requests
            if history.len() > 10 {
                history.remove(0);
            }
        }
        
        Self {
            id,
            path: path.to_string(),
            start: Instant::now(),
            memory_before,
            _stack_before,
        }
    }
    
    pub fn complete(&self, status: u16) {
        let memory_after = unsafe { heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32) as u32 };
        let duration = self.start.elapsed();
        
        ACTIVE_REQUESTS.fetch_sub(1, Ordering::SeqCst);
        
        if DIAGNOSTIC_MODE.load(Ordering::Relaxed) {
            let memory_used = self.memory_before.saturating_sub(memory_after);
            info!("REQ[{}] DONE: {} - {}ms, {} bytes used, status={}", 
                self.id, self.path, duration.as_millis(), memory_used, status);
            
            if memory_used > 50_000 {
                warn!("REQ[{}] High memory usage: {} bytes", self.id, memory_used);
            }
            
            if duration > Duration::from_secs(2) {
                warn!("REQ[{}] Slow request: {}ms", self.id, duration.as_millis());
            }
        }
        
        // Update history
        if let Ok(mut history) = REQUEST_HISTORY.lock() {
            if let Some(trace) = history.iter_mut().find(|t| t.id == self.id) {
                trace.end_time = Some(Instant::now());
                trace.status = RequestStatus::Completed(status);
                trace.memory_after = Some(memory_after);
            }
        }
    }
}

impl Drop for RequestTracker {
    fn drop(&mut self) {
        // Ensure we always decrement counter
        if ACTIVE_REQUESTS.load(Ordering::SeqCst) > 0 {
            ACTIVE_REQUESTS.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Monitor all running tasks (simplified version)
pub fn monitor_tasks() {
    if !DIAGNOSTIC_MODE.load(Ordering::Relaxed) {
        return;
    }
    
    // For now, just log basic task info
    // vTaskList requires CONFIG_FREERTOS_USE_STATS_FORMATTING_FUNCTIONS
    info!("=== TASK STATUS ===");
    unsafe {
        let current_task = xTaskGetCurrentTaskHandle();
        if !current_task.is_null() {
            let stack_remaining = uxTaskGetStackHighWaterMark(current_task);
            info!("Current task stack remaining: {} bytes", stack_remaining);
            
            if stack_remaining < 1000 {
                error!("Current task LOW STACK: {} bytes", stack_remaining);
            }
        }
    }
}

/// Dump all diagnostic information
pub fn dump_diagnostics() {
    error!("=== CRASH DIAGNOSTICS ===");
    
    // Last checkpoint
    if let Ok(checkpoint) = LAST_CHECKPOINT.lock() {
        if let Some(cp) = checkpoint.as_ref() {
            error!("Last checkpoint: {}", cp);
        }
    }
    
    // Memory state
    unsafe {
        let heap_internal = heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32);
        let heap_largest = heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL as u32);
        let heap_min = heap_caps_get_minimum_free_size(MALLOC_CAP_INTERNAL as u32);
        let psram = heap_caps_get_free_size(MALLOC_CAP_SPIRAM as u32);
        
        error!("Memory: internal={}, largest={}, min_ever={}, psram={}", 
            heap_internal, heap_largest, heap_min, psram);
    }
    
    // Active requests
    let active = ACTIVE_REQUESTS.load(Ordering::SeqCst);
    error!("Active HTTP requests: {}", active);
    
    // Request history
    if let Ok(history) = REQUEST_HISTORY.lock() {
        error!("Recent requests:");
        for trace in history.iter() {
            match &trace.status {
                RequestStatus::Active => {
                    error!("  [{}] {} - ACTIVE for {:?}", 
                        trace.id, trace.path, trace.start_time.elapsed());
                },
                RequestStatus::Completed(status) => {
                    let duration = trace.end_time.unwrap_or(Instant::now()) - trace.start_time;
                    let mem_used = trace.memory_before.saturating_sub(
                        trace.memory_after.unwrap_or(trace.memory_before)
                    );
                    error!("  [{}] {} - {} in {:?}, {} bytes", 
                        trace.id, trace.path, status, duration, mem_used);
                },
                RequestStatus::Failed(err) => {
                    error!("  [{}] {} - FAILED: {}", trace.id, trace.path, err);
                },
            }
        }
    }
    
    // Task status
    monitor_tasks();
    
    error!("=== END DIAGNOSTICS ===");
}

/// Initialize crash diagnostics
pub fn init() {
    info!("Crash diagnostics initialized - diagnostic mode ENABLED");
    
    // Set up periodic diagnostics
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(Duration::from_secs(30));
            
            if DIAGNOSTIC_MODE.load(Ordering::Relaxed) {
                info!("=== PERIODIC DIAGNOSTICS ===");
                unsafe {
                    let heap = heap_caps_get_free_size(MALLOC_CAP_INTERNAL as u32);
                    let active_reqs = ACTIVE_REQUESTS.load(Ordering::SeqCst);
                    info!("Heap: {} KB, Active requests: {}", heap / 1024, active_reqs);
                }
            }
        }
    });
}

/// Test endpoint that bypasses most systems
pub fn handle_diagnostic_test(req: esp_idf_svc::http::server::Request<&mut esp_idf_svc::http::server::EspHttpConnection>) 
    -> Result<(), Box<dyn std::error::Error>> {
    use esp_idf_svc::io::Write;
    
    checkpoint("diagnostic_test_start");
    
    let mut response = req.into_response(200, Some("OK"), &[
        ("Content-Type", "text/plain"),
    ])?;
    
    checkpoint("diagnostic_test_response_created");
    
    response.write_all(b"DIAGNOSTIC TEST OK\n")?;
    
    checkpoint("diagnostic_test_complete");
    
    Ok(())
}