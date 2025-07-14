// main.rs - Example Rust dashboard using FFI display driver

#![no_std]
#![no_main]

extern crate alloc;
use alloc::format;
use alloc::string::String;

use esp_idf_sys as _; // If using esp-idf-sys (bindings need this)
use esp_idf_hal::{
    delay::FreeRtos,
    gpio::*,
    prelude::*,
};

mod display;
use display::{Display, Color};

// Button pins
const BUTTON_1_PIN: i32 = 0;  // Boot button
const BUTTON_2_PIN: i32 = 14; // User button

// Screen states (just like your Arduino version)
#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    System,
    Power,
    WiFi,
    Hardware,
    Settings,
}

struct Dashboard {
    display: Display,
    current_screen: Screen,
    button1: PinDriver<'static, Gpio0, Input>,
    button2: PinDriver<'static, Gpio14, Input>,
}

impl Dashboard {
    fn new(peripherals: Peripherals) -> Result<Self, &'static str> {
        // Initialize display through FFI
        let display = Display::new()?;
        
        // Set up buttons
        let button1 = PinDriver::input(peripherals.pins.gpio0)?;
        let button2 = PinDriver::input(peripherals.pins.gpio14)?;
        
        Ok(Dashboard {
            display,
            current_screen: Screen::System,
            button1,
            button2,
        })
    }
    
    fn draw_header(&mut self) {
        // Blue header bar
        self.display.fill_rect(0, 0, 320, 20, Color::PRIMARY_BLUE);
        
        // Screen name
        let screen_name = match self.current_screen {
            Screen::System => "System",
            Screen::Power => "Power",
            Screen::WiFi => "WiFi",
            Screen::Hardware => "Hardware",
            Screen::Settings => "Settings",
        };
        
        self.display.draw_text_transparent(130, 6, screen_name, Color::WHITE).ok();
    }
    
    fn draw_system_screen(&mut self) {
        // Memory section
        self.display.fill_rect(40, 25, 240, 45, Color(0x1082));
        self.display.fill_rect(40, 25, 240, 1, Color::PRIMARY_GREEN);
        
        // Memory info (you'd get real values in production)
        let free_heap = 150; // KB
        let heap_percent = 75; // %
        
        self.display.draw_text(45, 30, "Memory Usage", Color::TEXT_SECONDARY).ok();
        let mem_text = format!("{}% free - {} KB", heap_percent, free_heap);
        self.display.draw_text(45, 42, &mem_text, Color::PRIMARY_GREEN).ok();
        
        // CPU section
        self.display.fill_rect(40, 75, 240, 45, Color(0x1082));
        self.display.fill_rect(40, 75, 240, 1, Color::PRIMARY_BLUE);
        
        self.display.draw_text(45, 80, "CPU Performance", Color::TEXT_SECONDARY).ok();
        self.display.draw_text(45, 92, "45% usage", Color::PRIMARY_BLUE).ok();
        self.display.draw_text(45, 104, "2 cores @ 240MHz", Color::TEXT_SECONDARY).ok();
        
        // Version info
        self.display.draw_text(45, 145, "Rust Dashboard v0.1", Color::PRIMARY_BLUE).ok();
    }
    
    fn draw_power_screen(&mut self) {
        // Power status
        self.display.fill_rect(40, 25, 240, 45, Color(0x1082));
        self.display.fill_rect(40, 25, 240, 1, Color::YELLOW);
        
        self.display.draw_text(45, 30, "Power Source", Color::TEXT_SECONDARY).ok();
        self.display.draw_text(45, 42, "USB Power", Color::YELLOW).ok();
        
        // Battery icon
        self.display.fill_rect(45, 55, 40, 12, Color(0x3186));
        self.display.fill_rect(46, 56, 38, 10, Color::BLACK);
        self.display.fill_rect(85, 58, 2, 6, Color(0x3186));
        
        // Battery fill (example: 75%)
        self.display.fill_rect(47, 57, 28, 8, Color::GREEN);
        self.display.draw_text(95, 58, "75%", Color::GREEN).ok();
    }
    
    fn update(&mut self) {
        // Clear screen
        self.display.clear(Color::BLACK);
        
        // Draw header
        self.draw_header();
        
        // Draw current screen
        match self.current_screen {
            Screen::System => self.draw_system_screen(),
            Screen::Power => self.draw_power_screen(),
            _ => {
                // Placeholder for other screens
                self.display.draw_text(100, 80, "Coming Soon!", Color::PRIMARY_BLUE).ok();
            }
        }
        
        // Draw navigation hints
        self.display.draw_text(10, 150, "< Prev", Color::TEXT_SECONDARY).ok();
        self.display.draw_text(260, 150, "Next >", Color::TEXT_SECONDARY).ok();
    }
    
    fn handle_input(&mut self) {
        // Check button 1 (previous screen)
        if self.button1.is_low() {
            self.current_screen = match self.current_screen {
                Screen::System => Screen::Settings,
                Screen::Power => Screen::System,
                Screen::WiFi => Screen::Power,
                Screen::Hardware => Screen::WiFi,
                Screen::Settings => Screen::Hardware,
            };
            FreeRtos::delay_ms(200); // Simple debounce
        }
        
        // Check button 2 (next screen)
        if self.button2.is_low() {
            self.current_screen = match self.current_screen {
                Screen::System => Screen::Power,
                Screen::Power => Screen::WiFi,
                Screen::WiFi => Screen::Hardware,
                Screen::Hardware => Screen::Settings,
                Screen::Settings => Screen::System,
            };
            FreeRtos::delay_ms(200); // Simple debounce
        }
    }
}

#[no_mangle]
fn main() -> ! {
    // Initialize ESP-IDF
    esp_idf_sys::link_patches();
    
    // Take peripherals
    let peripherals = Peripherals::take().unwrap();
    
    // Create dashboard
    let mut dashboard = Dashboard::new(peripherals).unwrap();
    
    // Initial draw
    dashboard.update();
    
    // Main loop
    loop {
        dashboard.handle_input();
        dashboard.update();
        FreeRtos::delay_ms(50); // 20 FPS update rate
    }
}

// Required panic handler for no_std
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // In production, you might want to:
    // - Log to serial
    // - Show error on display
    // - Reboot after delay
    loop {}
}

// Memory allocator for no_std
extern crate alloc;
use esp_idf_sys::esp_system_abort;

#[global_allocator]
static ALLOCATOR: esp_idf_sys::EspHeap = esp_idf_sys::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    
    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr(), HEAP_SIZE);
    }
}