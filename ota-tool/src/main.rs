use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "ota-tool")]
#[command(about = "ESP32-S3 Dashboard OTA Update Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Device IP address (for direct update)
    #[arg(value_name = "IP")]
    ip: Option<String>,

    /// Firmware file to upload
    #[arg(short, long, default_value = "target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard")]
    firmware: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan network for devices
    Scan {
        /// Network subnet to scan (e.g., 192.168.1)
        #[arg(short, long)]
        subnet: Option<String>,
    },
    /// Auto-discover and update all devices
    Auto {
        /// Update devices in parallel
        #[arg(short, long)]
        parallel: bool,
        
        /// Skip confirmation prompt
        #[arg(long)]
        no_confirm: bool,
    },
}

struct Device {
    ip: String,
    port: u16,
    name: String,
    version: String,
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}:{}) v{}", self.name, self.ip, self.port, self.version)
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Scan { subnet }) => {
            scan_devices(subnet.as_deref());
        }
        Some(Commands::Auto { parallel, no_confirm }) => {
            auto_update(&cli.firmware, *parallel, *no_confirm);
        }
        None => {
            if let Some(ip) = cli.ip {
                update_single_device(&ip, &cli.firmware);
            } else {
                println!("{}", "ESP32-S3 Dashboard OTA Tool".bold().blue());
                println!("\nUsage:");
                println!("  {} <IP>              Update specific device", "ota-tool".green());
                println!("  {} scan             Find devices on network", "ota-tool".green());
                println!("  {} auto             Update all devices", "ota-tool".green());
                println!("\nExamples:");
                println!("  ota-tool 192.168.1.100");
                println!("  ota-tool scan");
                println!("  ota-tool auto --parallel");
            }
        }
    }
}

fn scan_devices(subnet: Option<&str>) -> Vec<Device> {
    let subnet = subnet.unwrap_or_else(|| {
        // Try to detect local subnet
        "192.168.1"
    });

    println!("🔍 {} {}.0/24...", "Scanning network".cyan(), subnet);
    
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    
    let mut devices = Vec::new();
    
    // Scan all IPs in parallel using threads
    let handles: Vec<_> = (1..255)
        .map(|i| {
            let ip = format!("{}.{}", subnet, i);
            let client = client.clone();
            
            std::thread::spawn(move || {
                if let Ok(response) = client.get(&format!("http://{}:80/api/system", ip)).send() {
                    if response.status().is_success() {
                        if let Ok(json) = response.json::<serde_json::Value>() {
                            return Some(Device {
                                ip: ip.clone(),
                                port: 80,
                                name: json["hostname"].as_str().unwrap_or("esp32").to_string(),
                                version: json["version"].as_str().unwrap_or("unknown").to_string(),
                            });
                        }
                    }
                }
                None
            })
        })
        .collect();
    
    // Collect results
    for handle in handles {
        if let Ok(Some(device)) = handle.join() {
            println!("  ✓ Found: {}", device.to_string().green());
            devices.push(device);
        }
    }
    
    if devices.is_empty() {
        println!("{}", "❌ No devices found".red());
    } else {
        println!("\n📱 Found {} device(s)", devices.len());
    }
    
    devices
}

fn auto_update(firmware_path: &Path, parallel: bool, no_confirm: bool) {
    let devices = scan_devices(None);
    
    if devices.is_empty() {
        println!("\n{}", "No devices found to update".red());
        return;
    }
    
    if !no_confirm {
        println!("\n{} (y/N): ", "Update all devices?".yellow());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Update cancelled");
            return;
        }
    }
    
    let mut success = 0;
    let total = devices.len();
    
    if parallel {
        // Update in parallel
        let handles: Vec<_> = devices.into_iter()
            .map(|device| {
                let firmware_path = firmware_path.to_path_buf();
                std::thread::spawn(move || {
                    upload_firmware(&device, &firmware_path)
                })
            })
            .collect();
        
        for handle in handles {
            if let Ok(result) = handle.join() {
                if result {
                    success += 1;
                }
            }
        }
    } else {
        // Update sequentially
        for device in devices {
            if upload_firmware(&device, firmware_path) {
                success += 1;
            }
        }
    }
    
    println!("\n✨ {} {}/{} successful", "Update complete:".green(), success, total);
}

fn update_single_device(ip: &str, firmware_path: &Path) {
    let device = Device {
        ip: ip.to_string(),
        port: 80,
        name: format!("esp32-{}", ip.split('.').last().unwrap_or("device")),
        version: "unknown".to_string(),
    };
    
    if !firmware_path.exists() {
        println!("{} {}", "❌ Firmware not found:".red(), firmware_path.display());
        println!("   Run ./compile.sh --release to build firmware");
        return;
    }
    
    if upload_firmware(&device, firmware_path) {
        println!("\n✨ {}", "OTA update completed successfully!".green());
    } else {
        println!("\n{}", "❌ OTA update failed!".red());
        std::process::exit(1);
    }
}

fn upload_firmware(device: &Device, firmware_path: &Path) -> bool {
    let firmware_data = match fs::read(firmware_path) {
        Ok(data) => data,
        Err(e) => {
            println!("❌ Failed to read firmware: {}", e);
            return false;
        }
    };
    
    let file_size = firmware_data.len();
    println!("\n📤 {} {}", "Updating".cyan(), device);
    println!("   Firmware: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1024.0 / 1024.0);
    
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    
    // Create progress bar
    let pb = ProgressBar::new(file_size as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("   {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Upload firmware
    let url = format!("http://{}:{}/ota/update", device.ip, device.port);
    
    match client
        .post(&url)
        .header("Content-Length", file_size.to_string())
        .body(firmware_data)
        .send()
    {
        Ok(response) => {
            pb.finish_and_clear();
            if response.status().is_success() {
                println!("   {} Upload successful! Device will restart.", "✅".green());
                true
            } else {
                println!("   {} Upload failed: HTTP {}", "❌".red(), response.status());
                false
            }
        }
        Err(e) => {
            pb.finish_and_clear();
            println!("   {} Error: {}", "❌".red(), e);
            false
        }
    }
}

// Add this to allow JSON parsing
mod serde_json {
    pub type Value = std::collections::HashMap<String, String>;
}