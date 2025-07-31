// Core 1 background tasks - offload work from main Core 0
// This module implements sensor monitoring, network monitoring, and data processing
// on Core 1 to free up Core 0 for UI rendering and user interaction

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use esp_idf_sys::{xTaskCreatePinnedToCore, TaskHandle_t};
use std::ffi::CString;
use anyhow::Result;

pub mod network_monitor;
pub mod data_processor;

use network_monitor::NetworkMonitor;
use data_processor::DataProcessor;

// SensorUpdate moved here since Core 0 sends sensor data to Core 1
#[derive(Debug, Clone)]
pub struct SensorUpdate {
    pub temperature: f32,
    pub battery_percentage: u8,
    pub battery_voltage: u16,  // mV
    pub is_charging: bool,
    pub is_on_usb: bool,
    pub cpu_usage_core0: u8,
    pub cpu_usage_core1: u8,
}
pub use network_monitor::NetworkUpdate;

// Channels for communication between cores
pub struct Core1Channels {
    pub processed_rx: std::sync::mpsc::Receiver<ProcessedData>,
    pub sensor_tx: std::sync::mpsc::Sender<SensorUpdate>,  // Core 0 sends sensor data to Core 1
}

// Shared state between cores
pub struct Core1Manager {
    network_monitor: Arc<Mutex<NetworkMonitor>>,
    data_processor: Arc<Mutex<DataProcessor>>,
    task_handle: Option<TaskHandle_t>,
}

use data_processor::ProcessedData;

impl Core1Manager {
    pub fn new() -> Result<(Self, Core1Channels)> {
        // Create channels for sensor data FROM Core 0
        let (core0_sensor_tx, core0_sensor_rx) = std::sync::mpsc::channel();
        
        // Create channels for network data
        let (network_tx, network_rx) = std::sync::mpsc::channel();
        
        // Create channels for processed data TO Core 0
        let (processed_tx, processed_rx) = std::sync::mpsc::channel();
        
        // Create task components
        let network_monitor = Arc::new(Mutex::new(NetworkMonitor::new_with_channel(network_tx)));
        let data_processor = Arc::new(Mutex::new(DataProcessor::new_with_channel(
            core0_sensor_rx,  // Will receive sensor data from Core 0
            network_rx,
            processed_tx
        )));

        // Return channels for Core 0 to use
        let channels = Core1Channels {
            processed_rx,
            sensor_tx: core0_sensor_tx,  // Core 0 will use this to send sensor data
        };

        Ok((
            Self {
                network_monitor,
                data_processor,
                task_handle: None,
            },
            channels
        ))
    }

    pub fn start(&mut self) -> Result<()> {
        log::info!("Starting Core 1 background tasks...");
        
        // Clone Arc references for the task
        let network_monitor = self.network_monitor.clone();
        let data_processor = self.data_processor.clone();
        
        // Create the Core 1 task
        let mut handle: TaskHandle_t = std::ptr::null_mut();
        
        unsafe {
            let task_name = CString::new("core1_task")
                .expect("CString creation failed - no null bytes in string");
            
            let ret = xTaskCreatePinnedToCore(
                Some(core1_task_entry),
                task_name.as_ptr(),
                8192,  // Stack size
                Box::into_raw(Box::new((network_monitor, data_processor))) as *mut _,
                10,    // Priority
                &mut handle,
                1,     // Core 1
            );
            
            if ret != 1 {  // pdPASS
                return Err(anyhow::anyhow!("Failed to create Core 1 task"));
            }
        }
        
        self.task_handle = Some(handle);
        log::info!("Core 1 background tasks started successfully");
        
        Ok(())
    }
}

// Task entry point for Core 1
unsafe extern "C" fn core1_task_entry(pv_parameters: *mut std::ffi::c_void) {
    // Recover the task components
    let (network_monitor, data_processor): (
        Arc<Mutex<NetworkMonitor>>,
        Arc<Mutex<DataProcessor>>,
    ) = *Box::from_raw(pv_parameters as *mut _);
    
    // Force a visible log message
    println!("CORE1: Task started on CPU {:?}", esp_idf_hal::cpu::core());
    log::error!("CORE1: Starting background monitoring tasks (using log::error for visibility)");
    log::info!("Core 1: Network interval: 10s, Process interval: 100ms");
    
    // Task intervals
    let network_interval = Duration::from_secs(10);
    let process_interval = Duration::from_millis(100);
    
    let mut last_network = Instant::now();
    let mut last_process = Instant::now();
    let mut loop_counter = 0u32;
    
    loop {
        let now = Instant::now();
        loop_counter += 1;
        
        // Log every 10000 iterations to reduce log spam
        if loop_counter % 10000 == 0 {
            log::info!("CORE1: Heartbeat - iteration {}", loop_counter);
        }
        
        // Network monitoring (10s interval)
        if now.duration_since(last_network) >= network_interval {
            if let Ok(mut monitor) = network_monitor.try_lock() {
                if let Err(e) = monitor.update() {
                    log::warn!("Network monitor error: {}", e);
                }
            }
            last_network = now;
        }
        
        // Data processing (100ms interval) - only process when there's likely new data
        if now.duration_since(last_process) >= process_interval {
            if let Ok(mut processor) = data_processor.try_lock() {
                processor.process();
            }
            last_process = now;
        }
        
        // Calculate next wake time to reduce CPU usage
        let next_network = last_network + network_interval;
        let next_process = last_process + process_interval;
        let next_wake = next_network.min(next_process);
        let sleep_duration = next_wake.saturating_duration_since(now);
        
        // Sleep until next event (up to 100ms max to keep watchdog happy)
        let sleep_ms = sleep_duration.as_millis().min(100) as u32;
        if sleep_ms > 0 {
            esp_idf_hal::delay::FreeRtos::delay_ms(sleep_ms);
        } else {
            // Still yield even if no sleep needed
            esp_idf_hal::delay::FreeRtos::delay_ms(1);
        }
    }
}

impl Drop for Core1Manager {
    fn drop(&mut self) {
        if let Some(handle) = self.task_handle {
            unsafe {
                esp_idf_sys::vTaskDelete(handle);
            }
        }
    }
}