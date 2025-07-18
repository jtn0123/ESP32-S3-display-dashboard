use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "ota")]
#[command(about = "ESP32-S3 Dashboard OTA Update Tool", long_about = None)]
struct Cli {
    /// Device IP address
    ip: String,

    /// Firmware file to upload (defaults to release build)
    #[arg(short, long)]
    firmware: Option<PathBuf>,

    /// Port number (default: 80)
    #[arg(short, long, default_value = "80")]
    port: u16,
}

fn main() {
    let cli = Cli::parse();
    
    // Default firmware path
    let firmware_path = cli.firmware.unwrap_or_else(|| {
        PathBuf::from("target/xtensa-esp32s3-espidf/release/esp32-s3-dashboard")
    });

    // Check firmware exists
    if !firmware_path.exists() {
        eprintln!("{} Firmware not found: {}", "❌".red(), firmware_path.display());
        eprintln!("   Run ./compile.sh --release to build firmware");
        std::process::exit(1);
    }

    // Read firmware
    let firmware_data = match fs::read(&firmware_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{} Failed to read firmware: {}", "❌".red(), e);
            std::process::exit(1);
        }
    };

    let file_size = firmware_data.len();
    println!("{} ESP32-S3 Dashboard OTA Update", "🚀".blue());
    println!("{}Device: {}:{}", "   ".dimmed(), cli.ip, cli.port);
    println!("{}Firmware: {} bytes ({:.2} MB)", 
        "   ".dimmed(), 
        file_size, 
        file_size as f64 / 1024.0 / 1024.0
    );

    // Check device is reachable
    print!("{}Checking device...", "   ".dimmed());
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    match client.get(&format!("http://{}:{}/api/system", cli.ip, cli.port)).send() {
        Ok(resp) if resp.status().is_success() => {
            println!("\r{}Device online ✓    ", "   ".dimmed());
        }
        _ => {
            println!("\r{} Device not reachable", "❌".red());
            std::process::exit(1);
        }
    }

    // Create progress bar
    let pb = ProgressBar::new(file_size as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes}")
            .unwrap()
            .progress_chars("#>-")
    );

    // Upload firmware to /ota/update endpoint
    let url = format!("http://{}:{}/ota/update", cli.ip, cli.port);
    pb.set_message("Uploading firmware...");
    
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap();

    match client
        .post(&url)
        .header("Content-Length", file_size.to_string())
        .body(firmware_data)
        .send()
    {
        Ok(response) => {
            pb.finish_and_clear();
            if response.status().is_success() {
                println!("{} Upload successful! Device will restart.", "✅".green());
                println!("\n{} OTA update completed successfully!", "✨".green());
            } else {
                eprintln!("{} Upload failed: HTTP {}", "❌".red(), response.status());
                if response.status() == 404 {
                    eprintln!("   OTA endpoint not available. This may happen if:");
                    eprintln!("   - Device is running from factory partition without OTA support");
                    eprintln!("   - The firmware was not built with OTA enabled");
                }
                std::process::exit(1);
            }
        }
        Err(e) => {
            pb.finish_and_clear();
            eprintln!("{} Error: {}", "❌".red(), e);
            std::process::exit(1);
        }
    }
}