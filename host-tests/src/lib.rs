//! Host-based tests for ESP32-S3 Dashboard
//! These tests run on the development machine, not on the ESP32

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[test]
    fn test_basic() {
        assert_eq!(2 + 2, 4);
    }

    // Sanity: metrics accessor should not panic when called without prior init
    #[test]
    fn metrics_does_not_panic_when_uninitialized() {
        // SAFETY: We are only verifying that accessing metrics does not panic
        let metrics = ESP32_S3_Display_Dashboard::metrics::metrics();
        let guard = metrics.lock().expect("metrics guard usable");
        let _ = guard.cpu_freq_mhz; // touch a field via Deref
        let _clone_ok = Arc::clone(metrics); // Arc should be usable
        drop(guard);
    }

    // Add more host-based tests here
    // For example: configuration validation, data structure tests, etc.
}