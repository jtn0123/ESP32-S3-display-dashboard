// OTA Manager - handles firmware updates using ESP-IDF OTA API

use core::ffi::c_void;
use esp_idf_sys::{
    esp_ota_begin, esp_ota_end, esp_ota_get_next_update_partition,
    esp_ota_handle_t, esp_ota_set_boot_partition, esp_ota_write,
    esp_partition_t, esp_restart, EspError, ESP_ERR_OTA_VALIDATE_FAILED,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OtaStatus {
    Idle,
    Downloading { progress: u8 },
    Verifying,
    Ready,
    Failed,
}

#[derive(Debug)]
pub enum OtaError {
    NoUpdatePartition,
    BeginFailed,
    WriteFailed,
    ValidationFailed,
    BootPartitionFailed,
    InvalidSize,
}

pub struct OtaManager {
    update_partition: *const esp_partition_t,
    ota_handle: Option<esp_ota_handle_t>,
    expected_size: usize,
    bytes_written: usize,
    status: OtaStatus,
}

impl OtaManager {
    pub fn new() -> Result<Self, OtaError> {
        // Get the next OTA partition
        let update_partition = unsafe { esp_ota_get_next_update_partition(core::ptr::null()) };
        
        if update_partition.is_null() {
            return Err(OtaError::NoUpdatePartition);
        }
        
        Ok(Self {
            update_partition,
            ota_handle: None,
            expected_size: 0,
            bytes_written: 0,
            status: OtaStatus::Idle,
        })
    }
    
    pub fn begin_update(&mut self, size: usize) -> Result<(), OtaError> {
        if size == 0 || size > 4 * 1024 * 1024 {
            // Sanity check: firmware should be between 0 and 4MB
            return Err(OtaError::InvalidSize);
        }
        
        let mut handle: esp_ota_handle_t = core::ptr::null_mut();
        
        let result = unsafe {
            esp_ota_begin(
                self.update_partition,
                size as _,
                &mut handle as *mut _,
            )
        };
        
        if result != 0 {
            return Err(OtaError::BeginFailed);
        }
        
        self.ota_handle = Some(handle);
        self.expected_size = size;
        self.bytes_written = 0;
        self.status = OtaStatus::Downloading { progress: 0 };
        
        Ok(())
    }
    
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<(), OtaError> {
        let handle = self.ota_handle.ok_or(OtaError::WriteFailed)?;
        
        let result = unsafe {
            esp_ota_write(
                handle,
                data.as_ptr() as *const c_void,
                data.len() as _,
            )
        };
        
        if result != 0 {
            self.status = OtaStatus::Failed;
            return Err(OtaError::WriteFailed);
        }
        
        self.bytes_written += data.len();
        
        // Update progress
        if self.expected_size > 0 {
            let progress = ((self.bytes_written * 100) / self.expected_size) as u8;
            self.status = OtaStatus::Downloading { progress };
        }
        
        Ok(())
    }
    
    pub fn finish_update(&mut self) -> Result<(), OtaError> {
        let handle = self.ota_handle.take().ok_or(OtaError::ValidationFailed)?;
        
        self.status = OtaStatus::Verifying;
        
        // End the OTA update
        let result = unsafe { esp_ota_end(handle) };
        
        if result == ESP_ERR_OTA_VALIDATE_FAILED as i32 {
            self.status = OtaStatus::Failed;
            return Err(OtaError::ValidationFailed);
        } else if result != 0 {
            self.status = OtaStatus::Failed;
            return Err(OtaError::ValidationFailed);
        }
        
        // Set the new boot partition
        let result = unsafe { esp_ota_set_boot_partition(self.update_partition) };
        
        if result != 0 {
            self.status = OtaStatus::Failed;
            return Err(OtaError::BootPartitionFailed);
        }
        
        self.status = OtaStatus::Ready;
        Ok(())
    }
    
    pub fn restart(&self) {
        // Give some time for final operations
        esp_hal::delay::Delay::new_default().delay_millis(1000);
        
        // Restart the system
        unsafe { esp_restart(); }
    }
    
    pub fn get_status(&self) -> OtaStatus {
        self.status
    }
    
    pub fn get_progress(&self) -> u8 {
        match self.status {
            OtaStatus::Downloading { progress } => progress,
            OtaStatus::Ready => 100,
            _ => 0,
        }
    }
    
    pub fn cancel(&mut self) {
        if let Some(handle) = self.ota_handle.take() {
            unsafe { esp_ota_end(handle); }
        }
        self.status = OtaStatus::Idle;
        self.bytes_written = 0;
        self.expected_size = 0;
    }
}

impl Drop for OtaManager {
    fn drop(&mut self) {
        // Clean up any ongoing OTA operation
        if let Some(handle) = self.ota_handle.take() {
            unsafe { esp_ota_end(handle); }
        }
    }
}