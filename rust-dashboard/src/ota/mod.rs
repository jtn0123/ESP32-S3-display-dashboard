// OTA (Over-The-Air) update module

pub mod manager;
pub mod web_server;

pub use manager::{OtaManager, OtaStatus, OtaError};

// OTA update flow:
// 1. Check for updates (manual or automatic)
// 2. Download firmware to OTA partition
// 3. Verify integrity
// 4. Set boot partition
// 5. Restart

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateSource {
    WebUpload,      // Direct upload via web interface
    GitHubRelease,  // Download from GitHub releases
    LocalNetwork,   // mDNS discovery + local transfer
}