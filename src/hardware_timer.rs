// Hardware timer implementation for ESP32-S3
// Uses ESP-IDF timer API for precise, interrupt-driven timing

use esp_idf_sys::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type TimerCallback = Box<dyn FnMut() + Send>;

/// Hardware timer wrapper for ESP32-S3
pub struct HardwareTimer {
    timer: esp_timer_handle_t,
    callback: Arc<Mutex<Option<TimerCallback>>>,
}

// Static storage for timer callbacks (ESP-IDF timer API requires static lifetime)
static mut TIMER_CALLBACKS: Option<Vec<Arc<Mutex<Option<TimerCallback>>>>> = None;
static mut TIMER_COUNT: usize = 0;

impl HardwareTimer {
    /// Create a new hardware timer
    pub fn new() -> Result<Self, EspError> {
        unsafe {
            // Initialize callback storage on first use
            if TIMER_CALLBACKS.is_none() {
                TIMER_CALLBACKS = Some(Vec::with_capacity(10));
            }
            
            let callback = Arc::new(Mutex::new(None));
            let timer_index = TIMER_COUNT;
            
            // Store callback reference
            if let Some(ref mut callbacks) = TIMER_CALLBACKS {
                callbacks.push(callback.clone());
            }
            
            TIMER_COUNT += 1;
            
            // Create timer configuration
            let timer_config = esp_timer_create_args_t {
                callback: Some(Self::timer_callback),
                arg: timer_index as *mut _,
                dispatch_method: esp_timer_dispatch_t_ESP_TIMER_TASK,
                name: b"hw_timer\0".as_ptr() as *const _,
                skip_unhandled_events: false,
            };
            
            let mut timer: esp_timer_handle_t = std::ptr::null_mut();
            esp!(esp_timer_create(&timer_config, &mut timer))?;
            
            Ok(Self {
                timer,
                callback,
            })
        }
    }
    
    /// Set the timer callback
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        *self.callback.lock().unwrap() = Some(Box::new(callback));
    }
    
    /// Start the timer with a periodic interval
    pub fn start_periodic(&self, interval: Duration) -> Result<(), EspError> {
        let period_us = interval.as_micros() as u64;
        unsafe {
            esp!(esp_timer_start_periodic(self.timer, period_us))
        }
    }
    
    /// Start the timer for a one-shot execution
    pub fn start_once(&self, delay: Duration) -> Result<(), EspError> {
        let timeout_us = delay.as_micros() as u64;
        unsafe {
            esp!(esp_timer_start_once(self.timer, timeout_us))
        }
    }
    
    /// Stop the timer
    pub fn stop(&self) -> Result<(), EspError> {
        unsafe {
            esp!(esp_timer_stop(self.timer))
        }
    }
    
    /// Check if timer is active
    pub fn is_active(&self) -> bool {
        unsafe {
            esp_timer_is_active(self.timer)
        }
    }
    
    /// Get the next alarm time in microseconds
    pub fn get_next_alarm(&self) -> u64 {
        unsafe {
            esp_timer_get_next_alarm() as u64
        }
    }
    
    /// Timer callback handler
    extern "C" fn timer_callback(arg: *mut std::ffi::c_void) {
        unsafe {
            let timer_index = arg as usize;
            
            if let Some(ref callbacks) = TIMER_CALLBACKS {
                if timer_index < callbacks.len() {
                    if let Ok(mut callback_guard) = callbacks[timer_index].lock() {
                        if let Some(ref mut callback) = *callback_guard {
                            callback();
                        }
                    }
                }
            }
        }
    }
}

impl Drop for HardwareTimer {
    fn drop(&mut self) {
        unsafe {
            let _ = esp_timer_stop(self.timer);
            let _ = esp_timer_delete(self.timer);
        }
    }
}

/// High-precision timer for performance measurements
pub struct PerformanceTimer;

impl PerformanceTimer {
    /// Get current time in microseconds
    pub fn now_us() -> u64 {
        unsafe { esp_timer_get_time() as u64 }
    }
    
    /// Get current time as Duration
    pub fn now() -> Duration {
        Duration::from_micros(Self::now_us())
    }
    
    /// Measure execution time of a closure
    pub fn measure<F, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Self::now_us();
        let result = f();
        let elapsed = Self::now_us() - start;
        (result, Duration::from_micros(elapsed))
    }
}

/// Timer manager for coordinating multiple timers
pub struct TimerManager {
    sensor_timer: HardwareTimer,
    display_timer: HardwareTimer,
    network_timer: HardwareTimer,
    sensor_callback: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>,
    display_callback: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>,
    network_callback: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>,
}

impl TimerManager {
    /// Create a new timer manager
    pub fn new() -> Result<Self, EspError> {
        let mut sensor_timer = HardwareTimer::new()?;
        let mut display_timer = HardwareTimer::new()?;
        let mut network_timer = HardwareTimer::new()?;
        
        let sensor_callback: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>> = Arc::new(Mutex::new(None));
        let display_callback: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>> = Arc::new(Mutex::new(None));
        let network_callback: Arc<Mutex<Option<Box<dyn FnMut() + Send>>>> = Arc::new(Mutex::new(None));
        
        // Set up timer callbacks
        let sensor_cb_clone = sensor_callback.clone();
        sensor_timer.set_callback(move || {
            if let Ok(mut cb_guard) = sensor_cb_clone.lock() {
                if let Some(ref mut cb) = *cb_guard {
                    cb();
                }
            }
        });
        
        let display_cb_clone = display_callback.clone();
        display_timer.set_callback(move || {
            if let Ok(mut cb_guard) = display_cb_clone.lock() {
                if let Some(ref mut cb) = *cb_guard {
                    cb();
                }
            }
        });
        
        let network_cb_clone = network_callback.clone();
        network_timer.set_callback(move || {
            if let Ok(mut cb_guard) = network_cb_clone.lock() {
                if let Some(ref mut cb) = *cb_guard {
                    cb();
                }
            }
        });
        
        Ok(Self {
            sensor_timer,
            display_timer,
            network_timer,
            sensor_callback,
            display_callback,
            network_callback,
        })
    }
    
    /// Set sensor update callback and interval
    pub fn set_sensor_callback<F>(&mut self, callback: F, interval: Duration) -> Result<(), EspError>
    where
        F: FnMut() + Send + 'static,
    {
        *self.sensor_callback.lock().unwrap() = Some(Box::new(callback));
        self.sensor_timer.start_periodic(interval)
    }
    
    /// Set display update callback and interval
    pub fn set_display_callback<F>(&mut self, callback: F, interval: Duration) -> Result<(), EspError>
    where
        F: FnMut() + Send + 'static,
    {
        *self.display_callback.lock().unwrap() = Some(Box::new(callback));
        self.display_timer.start_periodic(interval)
    }
    
    /// Set network check callback and interval
    pub fn set_network_callback<F>(&mut self, callback: F, interval: Duration) -> Result<(), EspError>
    where
        F: FnMut() + Send + 'static,
    {
        *self.network_callback.lock().unwrap() = Some(Box::new(callback));
        self.network_timer.start_periodic(interval)
    }
    
    /// Stop all timers
    pub fn stop_all(&self) -> Result<(), EspError> {
        self.sensor_timer.stop()?;
        self.display_timer.stop()?;
        self.network_timer.stop()?;
        Ok(())
    }
}

/// One-shot timer for delayed operations
pub struct OneShotTimer {
    timer: HardwareTimer,
}

impl OneShotTimer {
    /// Create a new one-shot timer
    pub fn new() -> Result<Self, EspError> {
        Ok(Self {
            timer: HardwareTimer::new()?,
        })
    }
    
    /// Schedule a one-shot execution after a delay
    pub fn schedule<F>(&mut self, delay: Duration, callback: F) -> Result<(), EspError>
    where
        F: FnOnce() + Send + 'static,
    {
        let mut callback_opt = Some(callback);
        
        self.timer.set_callback(move || {
            if let Some(cb) = callback_opt.take() {
                cb();
            }
        });
        
        self.timer.start_once(delay)
    }
    
    /// Cancel the scheduled execution
    pub fn cancel(&self) -> Result<(), EspError> {
        self.timer.stop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[test]
    fn test_performance_timer() {
        let (_, duration) = PerformanceTimer::measure(|| {
            std::thread::sleep(Duration::from_millis(10));
        });
        
        assert!(duration >= Duration::from_millis(10));
        assert!(duration < Duration::from_millis(20));
    }
    
    #[test]
    fn test_hardware_timer() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let mut timer = HardwareTimer::new().unwrap();
        timer.set_callback(move || {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
        
        timer.start_periodic(Duration::from_millis(10)).unwrap();
        std::thread::sleep(Duration::from_millis(55));
        timer.stop().unwrap();
        
        let count = counter.load(Ordering::Relaxed);
        assert!(count >= 4 && count <= 6); // Should fire ~5 times
    }
}