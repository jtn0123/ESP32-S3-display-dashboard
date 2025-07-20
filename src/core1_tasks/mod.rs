// Core 1 background tasks - offload work from main Core 0
// This module implements sensor monitoring, network monitoring, and data processing
// on Core 1 to free up Core 0 for UI rendering and user interaction

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use esp_idf_sys::{xTaskCreatePinnedToCore, TaskHandle_t};
use std::ffi::CString;
use anyhow::Result;

pub mod sensor_task;
pub mod network_monitor;
pub mod data_processor;

use sensor_task::SensorTask;
use network_monitor::NetworkMonitor;
use data_processor::DataProcessor;

// Channel for sending data from Core 1 to Core 0
pub use sensor_task::SensorUpdate;
pub use network_monitor::NetworkUpdate;

// Channels for communication between cores
pub struct Core1Channels {
    pub sensor_rx: std::sync::mpsc::Receiver<SensorUpdate>,
    pub network_rx: std::sync::mpsc::Receiver<NetworkUpdate>,
    pub processed_rx: std::sync::mpsc::Receiver<ProcessedData>,
}

// Shared state between cores
pub struct Core1Manager {
    sensor_task: Arc<Mutex<SensorTask>>,
    network_monitor: Arc<Mutex<NetworkMonitor>>,
    data_processor: Arc<Mutex<DataProcessor>>,
    task_handle: Option<TaskHandle_t>,
}

use data_processor::ProcessedData;

impl Core1Manager {
    pub fn new() -> Result<(Self, Core1Channels)> {
        // Create channels for sensor data
        let (sensor_tx, sensor_rx) = std::sync::mpsc::channel();
        
        // Create channels for network data
        let (network_tx, network_rx) = std::sync::mpsc::channel();
        
        // Create channels for processed data
        let (processed_tx, processed_rx) = std::sync::mpsc::channel();
        
        // Create task components with the senders
        let sensor_task = Arc::new(Mutex::new(SensorTask::new_with_channel(sensor_tx)));
        let network_monitor = Arc::new(Mutex::new(NetworkMonitor::new_with_channel(network_tx)));
        let data_processor = Arc::new(Mutex::new(DataProcessor::new_with_channel(
            sensor_rx,
            network_rx,
            processed_tx
        )));

        // Only return the processed data receiver to Core 0
        let channels = Core1Channels {
            sensor_rx: std::sync::mpsc::channel().1,  // Dummy channel, not used
            network_rx: std::sync::mpsc::channel().1,  // Dummy channel, not used
            processed_rx,
        };

        Ok((
            Self {
                sensor_task,
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
        let sensor_task = self.sensor_task.clone();
        let network_monitor = self.network_monitor.clone();
        let data_processor = self.data_processor.clone();
        
        // Create the Core 1 task
        let mut handle: TaskHandle_t = std::ptr::null_mut();
        
        unsafe {
            let task_name = CString::new("core1_task").unwrap();
            
            let ret = xTaskCreatePinnedToCore(
                Some(core1_task_entry),
                task_name.as_ptr(),
                8192,  // Stack size
                Box::into_raw(Box::new((sensor_task, network_monitor, data_processor))) as *mut _,
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
unsafe extern "C" fn core1_task_entry(pvParameters: *mut std::ffi::c_void) {
    // Recover the task components
    let (sensor_task, network_monitor, data_processor): (
        Arc<Mutex<SensorTask>>,
        Arc<Mutex<NetworkMonitor>>,
        Arc<Mutex<DataProcessor>>,
    ) = *Box::from_raw(pvParameters as *mut _);
    
    log::info!("Core 1 task started on CPU {:?}", esp_idf_hal::cpu::core());
    log::info!("Core 1: Starting background monitoring tasks");
    log::info!("Core 1: Sensor interval: 5s, Network interval: 10s, Process interval: 100ms");
    
    // Task intervals
    let sensor_interval = Duration::from_secs(5);
    let network_interval = Duration::from_secs(10);
    let process_interval = Duration::from_millis(100);
    
    let mut last_sensor = Instant::now();
    let mut last_network = Instant::now();
    let mut last_process = Instant::now();
    
    loop {
        let now = Instant::now();
        
        // Sensor monitoring (5s interval)
        if now.duration_since(last_sensor) >= sensor_interval {
            if let Ok(mut task) = sensor_task.try_lock() {
                log::debug!("Core 1: Running sensor update");
                if let Err(e) = task.update() {
                    log::warn!("Core 1: Sensor update error: {}", e);
                } else {
                    log::info!("Core 1: Sensor update completed");
                }
            }
            last_sensor = now;
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
        
        // Data processing (100ms interval)
        if now.duration_since(last_process) >= process_interval {
            if let Ok(mut processor) = data_processor.try_lock() {
                processor.process();
            }
            last_process = now;
        }
        
        // Yield to prevent watchdog
        esp_idf_hal::delay::FreeRtos::delay_ms(10);
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