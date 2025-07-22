/// defmt configuration for ultra-low overhead logging
/// 
/// This module configures defmt for ESP32-S3 with RTT (Real-Time Transfer) backend
/// for minimal performance impact during debugging.

#[cfg(feature = "defmt")]
use defmt_rtt as _; // global logger

#[cfg(feature = "defmt")]
use panic_probe as _; // panic handler

#[cfg(feature = "defmt")]
use defmt::{debug, error, info, trace, warn};

/// Timestamp function for defmt
#[cfg(feature = "defmt")]
defmt::timestamp!("{=u64:us}", {
    // Use ESP32-S3 cycle counter for microsecond timestamps
    unsafe {
        let ccount: u32;
        core::arch::asm!("rsr.ccount {}", out(reg) ccount);
        // Assuming 240MHz CPU clock
        (ccount as u64) / 240
    }
});

/// Example usage of defmt in display debugging
#[cfg(feature = "defmt")]
pub fn defmt_display_command(cmd: u8, data: &[u8]) {
    match data.len() {
        0 => defmt::debug!("ST7789 CMD: {:#04x} (no params)", cmd),
        1..=4 => defmt::debug!("ST7789 CMD: {:#04x} params: {=[u8]:x}", cmd, data),
        _ => defmt::debug!("ST7789 CMD: {:#04x} ({} bytes)", cmd, data.len()),
    }
}

/// Performance measurement with defmt
#[cfg(feature = "defmt")]
pub struct DefmtTimer {
    start: u64,
    name: &'static str,
}

#[cfg(feature = "defmt")]
impl DefmtTimer {
    pub fn start(name: &'static str) -> Self {
        let start = unsafe {
            let ccount: u32;
            core::arch::asm!("rsr.ccount {}", out(reg) ccount);
            ccount as u64
        };
        Self { start, name }
    }
    
    pub fn stop(self) {
        let end = unsafe {
            let ccount: u32;
            core::arch::asm!("rsr.ccount {}", out(reg) ccount);
            ccount as u64
        };
        let duration_us = (end - self.start) / 240;
        defmt::trace!("{} took {}us", self.name, duration_us);
    }
}

/// Panic handler configuration
#[cfg(all(feature = "defmt", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("PANIC: {}", defmt::Display2Format(info));
    
    // Print backtrace if available
    defmt::error!("Backtrace:");
    
    // Reset the system
    unsafe {
        esp_idf_sys::esp_restart();
    }
    
    // Unreachable
    loop {}
}

/// Memory usage tracking with defmt
#[cfg(feature = "defmt")]
pub fn log_memory_usage() {
    unsafe {
        let free_heap = esp_idf_sys::esp_get_free_heap_size();
        let min_free_heap = esp_idf_sys::esp_get_minimum_free_heap_size();
        
        defmt::info!(
            "Heap: free={} min_free={} used={}",
            free_heap,
            min_free_heap,
            min_free_heap - free_heap
        );
    }
}