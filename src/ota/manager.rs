// OTA Manager - handles firmware updates using ESP-IDF OTA API

use core::ffi::c_void;
use esp_idf_sys::{
    esp_ota_begin, esp_ota_end, esp_ota_get_next_update_partition,
    esp_ota_handle_t, esp_ota_set_boot_partition, esp_ota_write,
    esp_partition_t,
    esp_partition_find_first, esp_partition_type_t_ESP_PARTITION_TYPE_APP as ESP_PARTITION_TYPE_APP,
    esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_APP_OTA_0 as ESP_PARTITION_SUBTYPE_APP_OTA_0,
};
use std::fmt;
use std::ffi::CStr;
use sha2::{Sha256, Digest};
use esp_idf_hal::delay::FreeRtos;

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

impl fmt::Display for OtaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OtaError::NoUpdatePartition => write!(f, "No update partition available"),
            OtaError::BeginFailed => write!(f, "Failed to begin OTA update"),
            OtaError::WriteFailed => write!(f, "Failed to write OTA data"),
            OtaError::ValidationFailed => write!(f, "OTA validation failed"),
            OtaError::BootPartitionFailed => write!(f, "Failed to set boot partition"),
            OtaError::InvalidSize => write!(f, "Invalid firmware size"),
        }
    }
}

impl std::error::Error for OtaError {}

pub struct OtaManager {
    update_partition: *const esp_partition_t,
    ota_handle: Option<esp_ota_handle_t>,
    expected_size: usize,
    bytes_written: usize,
    status: OtaStatus,
    sha256_hasher: Option<Sha256>,
    expected_sha256: Option<String>,
}

// SAFETY: OtaManager only contains a pointer to the partition structure which is
// statically allocated by ESP-IDF and is safe to share between threads
unsafe impl Send for OtaManager {}
unsafe impl Sync for OtaManager {}

impl OtaManager {
    pub fn new() -> Result<Self, OtaError> {
        log::info!("OTA: Initializing OTA manager...");
        
        // Check current partition
        let running_partition = unsafe { esp_idf_sys::esp_ota_get_running_partition() };
        if !running_partition.is_null() {
            unsafe {
                let partition = &*running_partition;
                let label = CStr::from_ptr(partition.label.as_ptr());
                log::info!("OTA: Currently running from partition: {:?}", label);
            }
        }
        
        // Try to get the next OTA partition normally
        let mut update_partition = unsafe { esp_ota_get_next_update_partition(core::ptr::null()) };
        
        // If that fails (we're on factory), find the first OTA partition manually
        if update_partition.is_null() {
            log::info!("OTA: Running from factory partition, finding first OTA partition...");
            
            // Find first OTA partition (ota_0)
            update_partition = unsafe {
                esp_partition_find_first(
                    ESP_PARTITION_TYPE_APP,
                    ESP_PARTITION_SUBTYPE_APP_OTA_0,
                    core::ptr::null()
                )
            };
            
            if update_partition.is_null() {
                log::error!("OTA: No OTA partition found in partition table");
                return Err(OtaError::NoUpdatePartition);
            }
            
            // Log partition info
            unsafe {
                let partition = &*update_partition;
                let label = CStr::from_ptr(partition.label.as_ptr());
                log::info!("OTA: Found OTA partition: {:?} at offset 0x{:x}, size: {} bytes", 
                    label, partition.address, partition.size);
            }
        } else {
            // Log the found partition
            unsafe {
                let partition = &*update_partition;
                let label = CStr::from_ptr(partition.label.as_ptr());
                log::info!("OTA: Next update partition: {:?} at offset 0x{:x}, size: {} bytes", 
                    label, partition.address, partition.size);
            }
        }
        
        Ok(Self {
            update_partition,
            ota_handle: None,
            expected_size: 0,
            bytes_written: 0,
            status: OtaStatus::Idle,
            sha256_hasher: None,
            expected_sha256: None,
        })
    }
    
    pub fn set_expected_sha256(&mut self, sha256: String) {
        self.expected_sha256 = Some(sha256);
    }
    
    pub fn begin_update(&mut self, size: usize) -> Result<(), OtaError> {
        if size == 0 || size > 4 * 1024 * 1024 {
            // Sanity check: firmware should be between 0 and 4MB
            log::error!("OTA: Invalid firmware size: {} bytes", size);
            return Err(OtaError::InvalidSize);
        }
        
        log::info!("OTA: Beginning update with size: {} bytes", size);
        
        // Log partition info
        unsafe {
            let partition = &*self.update_partition;
            let label = CStr::from_ptr(partition.label.as_ptr());
            log::info!("OTA: Target partition: {:?} at 0x{:x}, size: {} bytes", 
                label, partition.address, partition.size);
        }
        
        let mut handle: esp_ota_handle_t = 0;
        
        let result = unsafe {
            esp_ota_begin(
                self.update_partition,
                size as _,
                &mut handle as *mut _,
            )
        };
        
        if result != 0 {
            log::error!("OTA: esp_ota_begin failed with error code: {} (0x{:x})", result, result);
            
            // Log specific error details
            match result {
                -1 => log::error!("OTA: ESP_FAIL - Generic failure"),
                0x101 => log::error!("OTA: ESP_ERR_NO_MEM - Out of memory"),
                0x102 => log::error!("OTA: ESP_ERR_INVALID_ARG - Invalid argument"), 
                0x103 => log::error!("OTA: ESP_ERR_INVALID_STATE - Invalid state"),
                0x104 => log::error!("OTA: ESP_ERR_INVALID_SIZE - Invalid size"),
                0x105 => log::error!("OTA: ESP_ERR_NOT_FOUND - Requested resource not found"),
                0x106 => log::error!("OTA: ESP_ERR_NOT_SUPPORTED - Operation not supported"),
                0x107 => log::error!("OTA: ESP_ERR_TIMEOUT - Operation timed out"),
                0x108 => log::error!("OTA: ESP_ERR_INVALID_RESPONSE - Received invalid response"),
                0x109 => log::error!("OTA: ESP_ERR_INVALID_CRC - CRC or checksum was invalid"),
                0x10A => log::error!("OTA: ESP_ERR_INVALID_VERSION - Version was invalid"),
                0x10B => log::error!("OTA: ESP_ERR_INVALID_MAC - MAC address was invalid"),
                0x10C => log::error!("OTA: ESP_ERR_NOT_FINISHED - Operation has not fully completed"),
                0x1500 => log::error!("OTA: ESP_ERR_OTA_BASE - OTA error base"),
                0x1501 => log::error!("OTA: ESP_ERR_OTA_PARTITION_CONFLICT - Partition conflict"),
                0x1502 => log::error!("OTA: ESP_ERR_OTA_SELECT_INFO_INVALID - OTA data partition invalid"),
                0x1503 => log::error!("OTA: ESP_ERR_OTA_VALIDATE_FAILED - OTA image validate failed"),
                0x1504 => log::error!("OTA: ESP_ERR_OTA_SMALL_SEC_VER - New firmware security version is less than current"),
                0x1505 => log::error!("OTA: ESP_ERR_OTA_ROLLBACK_FAILED - Rollback failed"),
                0x1506 => log::error!("OTA: ESP_ERR_OTA_ROLLBACK_INVALID_STATE - Invalid rollback state"),
                _ => log::error!("OTA: Unknown error code: {} (0x{:x})", result, result),
            }
            
            return Err(OtaError::BeginFailed);
        }
        
        self.ota_handle = Some(handle);
        self.expected_size = size;
        self.bytes_written = 0;
        self.status = OtaStatus::Downloading { progress: 0 };
        self.sha256_hasher = Some(Sha256::new());
        
        Ok(())
    }
    
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<(), OtaError> {
        let handle = self.ota_handle.ok_or(OtaError::WriteFailed)?;
        
        // Update SHA256 hash
        if let Some(ref mut hasher) = self.sha256_hasher {
            hasher.update(data);
        }
        
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
        
        // Verify SHA256 if provided
        if let (Some(hasher), Some(expected)) = (self.sha256_hasher.take(), &self.expected_sha256) {
            let computed = format!("{:x}", hasher.finalize());
            log::info!("OTA: Computed SHA256: {}", computed);
            log::info!("OTA: Expected SHA256: {}", expected);
            
            if computed.to_lowercase() != expected.to_lowercase() {
                log::error!("OTA: SHA256 mismatch! Update rejected.");
                self.status = OtaStatus::Failed;
                // Still need to call esp_ota_end to clean up
                unsafe { esp_ota_end(handle); }
                return Err(OtaError::ValidationFailed);
            }
            
            log::info!("OTA: SHA256 validation passed");
        }
        
        // End the OTA update
        let result = unsafe { esp_ota_end(handle) };
        
        if result != 0 {
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
}

/// Ensure the device is booted into an OTA slot (ota_0/ota_1). If currently
/// running from the factory partition but an OTA partition exists, switch the
/// boot partition to the first OTA slot and reboot. Returns true if a switch
/// was initiated.
pub fn ensure_ota_boot_if_needed() -> bool {
    unsafe {
        let running = esp_idf_sys::esp_ota_get_running_partition();
        if running.is_null() {
            log::warn!("OTA: Running partition is null; skipping self-heal");
            return false;
        }
        let running_label = core::ffi::CStr::from_ptr((*running).label.as_ptr())
            .to_string_lossy()
            .to_string();

        // If already on an OTA slot, nothing to do
        if running_label.starts_with("ota_") {
            return false;
        }

        // Find first OTA partition (ota_0)
        let ota0 = esp_partition_find_first(
            ESP_PARTITION_TYPE_APP,
            ESP_PARTITION_SUBTYPE_APP_OTA_0,
            core::ptr::null(),
        );
        if ota0.is_null() {
            log::warn!("OTA: No OTA partition found; device likely flashed without OTA layout");
            return false;
        }

        // Set boot partition to ota_0 and reboot
        let r = esp_idf_sys::esp_ota_set_boot_partition(ota0);
        if r != 0 {
            log::error!("OTA: Failed to set boot partition to ota_0: {}", r);
            return false;
        }

        log::warn!("OTA: Self-heal: switching boot to ota_0 and restarting (was: {})", running_label);
        // Small delay to flush logs
        FreeRtos::delay_ms(500);
        esp_idf_sys::esp_restart();
        true
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