# Rust Migration: Display Driver & OTA Solutions Investigation

## Challenge 1: Display Driver (8-bit Parallel ST7789)

### Option 1: Adapt Existing SPI Display Drivers
**Concept:** Modify existing Rust SPI-based ST7789 drivers to support 8-bit parallel mode.

**Implementation Path:**
```rust
// Start with mipidsi or st7789 crate
// Fork and modify for parallel interface
use embedded_hal::digital::v2::OutputPin;

pub struct ParallelST7789<D0, D1, D2, D3, D4, D5, D6, D7, WR, DC, CS> {
    data_pins: (D0, D1, D2, D3, D4, D5, D6, D7),
    wr: WR,
    dc: DC,
    cs: CS,
}

impl<...> ParallelST7789<...> {
    fn write_byte(&mut self, data: u8) {
        // Set all 8 data pins
        self.data_pins.0.set_state((data & 0x01) != 0).ok();
        self.data_pins.1.set_state((data & 0x02) != 0).ok();
        // ... etc
        
        // Toggle WR pin
        self.wr.set_low().ok();
        self.wr.set_high().ok();
    }
}
```

**Existing Crates to Fork:**
- `mipidsi` - Generic MIPI display driver
- `st7789` - Specific ST7789 driver
- `display-interface-parallel-gpio` - Parallel interface trait

**Pros:**
- Leverages tested initialization sequences
- Community support possible
- Could contribute back to ecosystem

**Cons:**
- Still requires significant modification
- Pin manipulation might be slow

**Time Estimate:** 1 week

### Option 2: C-to-Rust Binding Layer (FFI)
**Concept:** Keep your working C display driver, create Rust bindings.

**Implementation Path:**
```c
// display_driver.c - Your existing code
void lcd_init(void);
void lcd_draw_pixel(uint16_t x, uint16_t y, uint16_t color);
void lcd_fill_rect(uint16_t x, uint16_t y, uint16_t w, uint16_t h, uint16_t color);
```

```rust
// display_bindings.rs
#[link(name = "display_driver")]
extern "C" {
    fn lcd_init();
    fn lcd_draw_pixel(x: u16, y: u16, color: u16);
    fn lcd_fill_rect(x: u16, y: u16, w: u16, h: u16, color: u16);
}

// Safe Rust wrapper
pub struct Display;

impl Display {
    pub fn new() -> Self {
        unsafe { lcd_init(); }
        Display
    }
    
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) {
        unsafe { lcd_draw_pixel(x, y, color); }
    }
}
```

**Build Configuration:**
```toml
# build.rs
use cc;

fn main() {
    cc::Build::new()
        .file("src/display_driver.c")
        .compile("display_driver");
}
```

**Pros:**
- Reuse 100% working code
- Fastest path to working system
- Can migrate incrementally

**Cons:**
- Mixed language complexity
- Loses some Rust safety benefits
- Two toolchains needed

**Time Estimate:** 2-3 days

### Option 3: Hardware Abstraction Layer with DMA
**Concept:** Use ESP32-S3's dedicated LCD peripheral (LCD_CAM) with DMA.

**Implementation Path:**
```rust
use esp_idf_hal::lcd_cam::{LcdCam, Config};
use esp_idf_hal::dma::{DmaChannel};

pub struct DmaDisplay {
    lcd_cam: LcdCam,
    dma_channel: DmaChannel,
    framebuffer: &'static mut [u16; 320 * 170],
}

impl DmaDisplay {
    pub fn new() -> Self {
        // Configure LCD_CAM peripheral for 8-bit mode
        let config = Config {
            data_width: DataWidth::Bit8,
            clock_freq: 10_000_000, // 10MHz
            ..Default::default()
        };
        
        // Use DMA for efficient transfers
        // This is MUCH faster than bit-banging
    }
    
    pub fn flush(&mut self) {
        // DMA transfer entire framebuffer
        self.dma_channel.transfer(self.framebuffer).wait();
    }
}
```

**Pros:**
- Hardware accelerated (very fast!)
- Professional solution
- Enables advanced features (tearing sync, etc.)

**Cons:**
- More complex initial setup
- ESP32-S3 specific
- Limited documentation

**Time Estimate:** 1-2 weeks

### Option 4: Hybrid Async Driver
**Concept:** Leverage Rust's async for non-blocking display updates.

```rust
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
async fn display_task(mut display: Display) {
    loop {
        display.update_async().await;
        Timer::after(Duration::from_millis(16)).await; // 60 FPS
    }
}
```

**Pros:**
- Non-blocking UI updates
- Better responsiveness
- Modern Rust patterns

**Time Estimate:** 1 week (after basic driver works)

## Challenge 2: OTA Updates

### Option 1: ESP-IDF OTA API Bindings
**Concept:** Use ESP-IDF's native OTA functionality through Rust bindings.

**Implementation Path:**
```rust
use esp_idf_sys::{esp_ota_begin, esp_ota_write, esp_ota_end, esp_ota_set_boot_partition};
use esp_idf_svc::http::server::{EspHttpServer, Configuration};

pub struct OtaUpdater {
    update_handle: Option<esp_ota_handle_t>,
}

impl OtaUpdater {
    pub fn begin_update(&mut self, size: usize) -> Result<(), EspError> {
        unsafe {
            let partition = esp_ota_get_next_update_partition(std::ptr::null());
            esp!(esp_ota_begin(partition, size as _, &mut self.update_handle))?;
        }
        Ok(())
    }
    
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<(), EspError> {
        unsafe {
            esp!(esp_ota_write(self.update_handle.unwrap(), 
                             data.as_ptr() as _, 
                             data.len() as _))?;
        }
        Ok(())
    }
    
    pub fn complete_update(&mut self) -> Result<(), EspError> {
        unsafe {
            esp!(esp_ota_end(self.update_handle.unwrap()))?;
            esp!(esp_ota_set_boot_partition(partition))?;
            esp_restart();
        }
        Ok(())
    }
}

// Web server for OTA
fn setup_ota_server() -> Result<EspHttpServer, EspError> {
    let mut server = EspHttpServer::new(&Configuration::default())?;
    
    server.fn_handler("/update", Method::Post, |mut req| {
        let mut ota = OtaUpdater::new();
        let content_length = req.content_len().unwrap_or(0) as usize;
        
        ota.begin_update(content_length)?;
        
        let mut buffer = vec![0u8; 4096];
        loop {
            let bytes_read = req.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            ota.write_chunk(&buffer[..bytes_read])?;
        }
        
        ota.complete_update()?;
        Ok(())
    })?;
    
    Ok(server)
}
```

**Pros:**
- Native ESP-IDF OTA (battle-tested)
- Supports rollback on failure
- Encrypted updates possible

**Cons:**
- Requires esp-idf-sys
- More complex than ArduinoOTA

**Time Estimate:** 3-4 days

### Option 2: Custom Pure-Rust OTA Protocol
**Concept:** Implement a custom OTA protocol similar to ArduinoOTA.

**Implementation Path:**
```rust
use embedded_svc::wifi::asynch::Wifi;
use esp_storage::FlashStorage;

pub struct RustOta {
    storage: FlashStorage,
    mdns: MdnsService,
}

impl RustOta {
    pub async fn start(&mut self) {
        // 1. Advertise via mDNS
        self.mdns.add_service("_rustota", "_tcp", 8266);
        
        // 2. Listen for UDP discovery
        let socket = UdpSocket::bind("0.0.0.0:8266").await?;
        
        // 3. Handle OTA protocol
        loop {
            let (data, addr) = socket.recv_from(&mut buffer).await?;
            match parse_ota_command(data) {
                OtaCommand::Start { size, md5 } => {
                    // Begin OTA session
                    self.begin_ota(size, md5).await?;
                }
                OtaCommand::Data { offset, chunk } => {
                    // Write firmware chunk
                    self.write_chunk(offset, chunk).await?;
                }
                OtaCommand::End => {
                    // Verify and reboot
                    self.complete_ota().await?;
                }
            }
        }
    }
    
    async fn write_chunk(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        // Direct partition write
        let partition = Partition::find_first(
            PartitionType::App,
            PartitionSubType::OtaData,
            None
        )?;
        
        partition.write(offset, data).await?;
        Ok(())
    }
}
```

**Pros:**
- Full control over protocol
- Can match ArduinoOTA behavior
- Pure Rust solution

**Cons:**
- Need to implement entire protocol
- Testing complexity
- Security considerations

**Time Estimate:** 1-2 weeks

### Option 3: GitHub Releases Auto-Updater
**Concept:** Modern OTA using GitHub releases API.

**Implementation Path:**
```rust
use serde::{Deserialize, Serialize};
use esp_idf_svc::http::client::EspHttpConnection;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<Asset>,
}

pub struct GitHubOta {
    repo: String,
    current_version: String,
}

impl GitHubOta {
    pub async fn check_for_updates(&self) -> Result<Option<String>> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        
        let client = HttpClient::new()?;
        let response: GitHubRelease = client.get(&url)
            .send().await?
            .json().await?;
        
        if response.tag_name != self.current_version {
            // Find .bin asset
            let bin_url = response.assets.iter()
                .find(|a| a.name.ends_with(".bin"))
                .map(|a| &a.browser_download_url);
                
            return Ok(bin_url.cloned());
        }
        
        Ok(None)
    }
    
    pub async fn download_and_install(&mut self, url: &str) -> Result<()> {
        // Stream download directly to OTA partition
        let mut ota = OtaUpdater::new();
        
        let mut response = client.get(url).send().await?;
        let size = response.content_length().unwrap_or(0);
        
        ota.begin_update(size)?;
        
        while let Some(chunk) = response.chunk().await? {
            ota.write_chunk(&chunk)?;
        }
        
        ota.complete_update()?;
        Ok(())
    }
}

// Auto-update task
#[embassy_executor::task]
async fn auto_update_task(ota: GitHubOta) {
    loop {
        if let Ok(Some(url)) = ota.check_for_updates().await {
            println!("Update available!");
            // Could show UI prompt here
            ota.download_and_install(&url).await.ok();
        }
        
        Timer::after(Duration::from_secs(3600)).await; // Check hourly
    }
}
```

**Pros:**
- Modern approach
- Automatic updates
- Version management built-in
- Secure (HTTPS)

**Cons:**
- Requires internet
- GitHub dependency
- Rate limits

**Time Estimate:** 1 week

### Option 4: Dual-Mode OTA (Best of Both)
**Concept:** Support both local network OTA and internet updates.

```rust
pub enum OtaMode {
    LocalNetwork(LocalOta),   // ArduinoOTA-like
    Internet(GitHubOta),       // Modern updates
}

impl OtaManager {
    pub async fn update(&mut self) -> Result<()> {
        match self.mode {
            OtaMode::LocalNetwork(ref mut local) => {
                // UDP discovery + local upload
                local.handle_local_ota().await
            }
            OtaMode::Internet(ref mut github) => {
                // Check GitHub releases
                github.check_and_update().await
            }
        }
    }
}
```

**Time Estimate:** 2 weeks (implementing both)

## Recommended Strategy

### For Display Driver:
**Start with Option 2 (FFI)** for immediate results, then migrate to **Option 3 (DMA)** for performance.

```bash
# Quick proof of concept
1. Keep your current display code in C
2. Create Rust bindings
3. Build hybrid app
4. Gradually port to pure Rust
```

### For OTA:
**Implement Option 1 (ESP-IDF OTA)** first, then add **Option 3 (GitHub)** for modern updates.

```bash
# Progressive enhancement
1. Basic ESP-IDF OTA (web upload)
2. Add mDNS discovery
3. Add GitHub auto-updates
4. Full feature parity
```

## Risk Mitigation

1. **Create minimal proof-of-concept first**
   - Display: Show "Hello Rust" on screen
   - OTA: Successfully update once

2. **Maintain parallel development**
   - Keep C++ version stable
   - Develop Rust version alongside

3. **Set go/no-go criteria**
   - Display must achieve 60 FPS
   - OTA must be reliable
   - Binary size < 4MB

## Next Steps

1. **Week 1**: FFI display driver + basic screen
2. **Week 2**: ESP-IDF OTA implementation  
3. **Week 3**: Port UI system
4. **Week 4**: Feature parity testing

Would you like me to create a proof-of-concept for any of these options?