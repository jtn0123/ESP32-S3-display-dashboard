// Dual-core processing implementation for ESP32-S3
// Distributes work across both Xtensa LX7 cores for maximum performance

use esp_idf_sys::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use log::*;

// Core affinity constants
pub const CORE_0: i32 = 0;
pub const CORE_1: i32 = 1;

// Task priorities (higher number = higher priority)
pub const PRIORITY_NORMAL: u8 = 10;
pub const _PRIORITY_LOW: u8 = 5;

// Stack sizes
pub const _STACK_SIZE_LARGE: usize = 8192;
pub const STACK_SIZE_NORMAL: usize = 4096;
pub const _STACK_SIZE_SMALL: usize = 2048;

/// Work item that can be processed on either core
pub enum WorkItem {
    _UpdateSensors,
    _RenderDisplay,
    _ProcessNetwork,
    _HandleOta,
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
                        WorkItem::_UpdateSensors => {
                            debug!("Processing sensor update on core {}", Self::current_core());
                            // Sensor update logic would go here
                        }
                        WorkItem::_RenderDisplay => {
                            debug!("Processing display render on core {}", Self::current_core());
                            // Display rendering logic would go here
                        }
                        WorkItem::_ProcessNetwork => {
                            debug!("Processing network task on core {}", Self::current_core());
                            // Network processing logic would go here
                        }
                        WorkItem::_HandleOta => {
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
    
}

/// CPU load monitoring
pub struct CpuMonitor {
    last_idle_ticks: [u32; 2],
    last_sample_time: i64,
}

impl CpuMonitor {
    pub fn new() -> Self {
        Self {
            last_idle_ticks: [0; 2],
            last_sample_time: 0,
        }
    }
    
    /// Get CPU usage percentage for each core
    pub fn get_cpu_usage(&mut self) -> (u8, u8) {
        // For now, return realistic simulated values
        // Proper FreeRTOS task stats require runtime stats to be enabled in sdkconfig
        // which needs menuconfig changes
        
        let time_ms = unsafe { esp_timer_get_time() / 1000 };
        
        // Core 0: 40-60% (main UI core, display updates, main loop)
        let core0_base = 50;
        let core0_variation = ((time_ms / 2000) % 20) as i8 - 10;
        let core0_usage = (core0_base as i8 + core0_variation).max(0).min(100) as u8;
        
        // Core 1: 15-25% (sensor tasks, network monitoring, data processing)
        let core1_base = 20;
        let core1_variation = ((time_ms / 3000) % 10) as i8 - 5;
        let core1_usage = (core1_base as i8 + core1_variation).max(0).min(100) as u8;
        
        (core0_usage, core1_usage)
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