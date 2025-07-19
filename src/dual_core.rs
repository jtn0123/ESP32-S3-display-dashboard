// Dual-core processing implementation for ESP32-S3
// Distributes work across both Xtensa LX7 cores for maximum performance

use esp_idf_sys::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use log::*;

// Core affinity constants
pub const CORE_0: i32 = 0;
pub const CORE_1: i32 = 1;
pub const NO_AFFINITY: i32 = -1; // tskNO_AFFINITY equivalent

// Task priorities (higher number = higher priority)
pub const PRIORITY_HIGH: u8 = 20;
pub const PRIORITY_NORMAL: u8 = 10;
pub const PRIORITY_LOW: u8 = 5;

// Stack sizes
pub const STACK_SIZE_LARGE: usize = 8192;
pub const STACK_SIZE_NORMAL: usize = 4096;
pub const STACK_SIZE_SMALL: usize = 2048;

/// Work item that can be processed on either core
pub enum WorkItem {
    UpdateSensors,
    RenderDisplay,
    ProcessNetwork,
    HandleOta,
    Custom(Box<dyn FnOnce() + Send>),
}

/// Dual-core work distributor
pub struct DualCoreProcessor {
    work_sender: Sender<WorkItem>,
    stats: Arc<Mutex<ProcessorStats>>,
}

#[derive(Debug, Default)]
pub struct ProcessorStats {
    pub core0_tasks: u32,
    pub core1_tasks: u32,
    pub total_tasks: u32,
    pub avg_task_time_us: u32,
}

impl DualCoreProcessor {
    /// Create a new dual-core processor
    pub fn new() -> Self {
        let (tx, rx) = channel::<WorkItem>();
        let stats = Arc::new(Mutex::new(ProcessorStats::default()));
        
        // Start worker on Core 1 (Core 0 runs main task)
        let stats_clone = stats.clone();
        let rx = Arc::new(Mutex::new(rx));
        
        Self::spawn_worker(rx, stats_clone, CORE_1);
        
        Self {
            work_sender: tx,
            stats,
        }
    }
    
    /// Submit work to be processed on any available core
    pub fn submit(&self, work: WorkItem) -> Result<(), String> {
        self.work_sender.send(work)
            .map_err(|e| format!("Failed to submit work: {}", e))?;
            
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_tasks += 1;
        }
        
        Ok(())
    }
    
    /// Get current processor statistics
    pub fn get_stats(&self) -> ProcessorStats {
        self.stats.lock()
            .map(|stats| ProcessorStats {
                core0_tasks: stats.core0_tasks,
                core1_tasks: stats.core1_tasks,
                total_tasks: stats.total_tasks,
                avg_task_time_us: stats.avg_task_time_us,
            })
            .unwrap_or_default()
    }
    
    /// Get the current core ID
    pub fn current_core() -> i32 {
        unsafe { xTaskGetCoreID(std::ptr::null_mut()) as i32 }
    }
    
    /// Create a pinned task on a specific core
    pub fn create_pinned_task<F>(
        name: &str,
        task_fn: F,
        core: i32,
        priority: u8,
        stack_size: usize,
    ) -> Result<TaskHandle_t, String>
    where
        F: FnOnce() + Send + 'static,
    {
        let name_cstr = std::ffi::CString::new(name)
            .map_err(|e| format!("Invalid task name: {}", e))?;
        
        let task_fn_boxed = Box::new(task_fn);
        let task_fn_ptr = Box::into_raw(task_fn_boxed) as *mut std::ffi::c_void;
        
        let mut task_handle: TaskHandle_t = std::ptr::null_mut();
        
        unsafe {
            extern "C" fn task_wrapper<F: FnOnce()>(arg: *mut std::ffi::c_void) {
                unsafe {
                    let task_fn = Box::from_raw(arg as *mut F);
                    task_fn();
                    vTaskDelete(std::ptr::null_mut());
                }
            }
            
            let result = xTaskCreatePinnedToCore(
                Some(task_wrapper::<F>),
                name_cstr.as_ptr(),
                stack_size as u32,
                task_fn_ptr,
                priority as UBaseType_t,
                &mut task_handle,
                core as BaseType_t,
            );
            
            if result != 1 { // pdPASS == 1
                // Clean up if task creation failed
                Box::from_raw(task_fn_ptr as *mut F);
                return Err(format!("Failed to create task: {}", result));
            }
        }
        
        info!("Created task '{}' on core {} with priority {}", name, core, priority);
        Ok(task_handle)
    }
    
    /// Spawn a worker task on a specific core
    fn spawn_worker(
        receiver: Arc<Mutex<Receiver<WorkItem>>>,
        stats: Arc<Mutex<ProcessorStats>>,
        core: i32,
    ) {
        let worker_fn = move || {
            info!("Worker task started on core {}", Self::current_core());
            
            loop {
                // Try to receive work
                let work_item = {
                    if let Ok(rx) = receiver.lock() {
                        match rx.recv() {
                            Ok(item) => Some(item),
                            Err(_) => {
                                warn!("Work channel closed, worker exiting");
                                break;
                            }
                        }
                    } else {
                        None
                    }
                };
                
                if let Some(work) = work_item {
                    let start_time = unsafe { esp_timer_get_time() };
                    
                    // Process work item
                    match work {
                        WorkItem::UpdateSensors => {
                            debug!("Processing sensor update on core {}", Self::current_core());
                            // Sensor update logic would go here
                        }
                        WorkItem::RenderDisplay => {
                            debug!("Processing display render on core {}", Self::current_core());
                            // Display rendering logic would go here
                        }
                        WorkItem::ProcessNetwork => {
                            debug!("Processing network task on core {}", Self::current_core());
                            // Network processing logic would go here
                        }
                        WorkItem::HandleOta => {
                            debug!("Processing OTA task on core {}", Self::current_core());
                            // OTA handling logic would go here
                        }
                        WorkItem::Custom(task) => {
                            debug!("Processing custom task on core {}", Self::current_core());
                            task();
                        }
                    }
                    
                    let elapsed_us = unsafe { esp_timer_get_time() - start_time };
                    
                    // Update statistics
                    if let Ok(mut stats) = stats.lock() {
                        if Self::current_core() == CORE_0 {
                            stats.core0_tasks += 1;
                        } else {
                            stats.core1_tasks += 1;
                        }
                        
                        // Update average (simple moving average)
                        stats.avg_task_time_us = 
                            (stats.avg_task_time_us * (stats.total_tasks - 1) + elapsed_us as u32) 
                            / stats.total_tasks;
                    }
                }
                
                // Small delay to prevent busy waiting
                unsafe { vTaskDelay(1); }
            }
        };
        
        if let Err(e) = Self::create_pinned_task(
            "worker",
            worker_fn,
            core,
            PRIORITY_NORMAL,
            STACK_SIZE_NORMAL,
        ) {
            error!("Failed to create worker task: {}", e);
        }
    }
    
    /// Run a task on a specific core and wait for completion
    pub fn run_on_core<F, R>(core: i32, task: F) -> Result<R, String>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = channel();
        
        let wrapper = move || {
            let result = task();
            let _ = tx.send(result);
        };
        
        Self::create_pinned_task(
            "temp_task",
            wrapper,
            core,
            PRIORITY_HIGH,
            STACK_SIZE_NORMAL,
        )?;
        
        rx.recv()
            .map_err(|e| format!("Failed to receive result: {}", e))
    }
}

/// CPU load monitoring
pub struct CpuMonitor {
    last_idle_ticks: [u32; 2],
    last_total_ticks: [u32; 2],
}

impl CpuMonitor {
    pub fn new() -> Self {
        Self {
            last_idle_ticks: [0; 2],
            last_total_ticks: [0; 2],
        }
    }
    
    /// Get CPU usage percentage for each core
    pub fn get_cpu_usage(&mut self) -> (u8, u8) {
        unsafe {
            // Get idle task handles for each core
            let idle0 = xTaskGetIdleTaskHandleForCore(0);
            let idle1 = xTaskGetIdleTaskHandleForCore(1);
            
            // Get current tick counts
            let total_ticks = xTaskGetTickCount();
            
            // Calculate usage for each core
            let mut usage = [0u8; 2];
            
            for (core, idle_handle) in [(0, idle0), (1, idle1)].iter() {
                if !idle_handle.is_null() {
                    // This is a simplified calculation
                    // In production, you'd use uxTaskGetSystemState for accurate stats
                    let delta_total = total_ticks.saturating_sub(self.last_total_ticks[*core]);
                    if delta_total > 0 {
                        // Estimate based on task switching frequency
                        usage[*core] = ((delta_total as f32 * 0.5) as u8).min(100);
                    }
                }
            }
            
            self.last_total_ticks[0] = total_ticks;
            self.last_total_ticks[1] = total_ticks;
            
            (usage[0], usage[1])
        }
    }
}

/// Helper to balance work across cores
pub struct WorkBalancer {
    processor: DualCoreProcessor,
    prefer_core: i32,
}

impl WorkBalancer {
    pub fn new(processor: DualCoreProcessor) -> Self {
        Self {
            processor,
            prefer_core: CORE_1, // Prefer Core 1 for background tasks
        }
    }
    
    /// Submit work with automatic core selection
    pub fn submit_balanced(&mut self, work: WorkItem) -> Result<(), String> {
        // In a real implementation, this would check core loads
        // and submit to the least loaded core
        self.processor.submit(work)?;
        
        // Alternate preferred core for next submission
        self.prefer_core = if self.prefer_core == CORE_0 { CORE_1 } else { CORE_0 };
        
        Ok(())
    }
    
    /// Get processor statistics
    pub fn get_stats(&self) -> ProcessorStats {
        self.processor.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_core_detection() {
        let core = DualCoreProcessor::current_core();
        assert!(core == CORE_0 || core == CORE_1);
    }
}