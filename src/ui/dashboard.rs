// Main dashboard controller managing screens and navigation

use crate::display::{Display, Color};
use super::screens::{Screen, SystemScreen, PowerScreen, WiFiScreen, HardwareScreen, SettingsScreen};
use super::theme::Theme;

pub struct Dashboard {
    display: Display,
    screens: [Box<dyn Screen>; 5],
    current_screen: usize,
    theme: Theme,
    theme_index: usize,
    show_menu: bool,
    menu_selection: usize,
}

impl Dashboard {
    pub fn new(display: Display) -> Self {
        // Initialize all screens
        let screens: [Box<dyn Screen>; 5] = [
            Box::new(SystemScreen::new()),
            Box::new(PowerScreen::new()),
            Box::new(WiFiScreen::new()),
            Box::new(HardwareScreen::new()),
            Box::new(SettingsScreen::new()),
        ];
        
        Self {
            display,
            screens,
            current_screen: 0,
            theme: Theme::default(),
            theme_index: 0,
            show_menu: false,
            menu_selection: 0,
        }
    }
    
    pub fn cycle_theme(&mut self) {
        // Get current theme index from settings screen if on that screen
        if self.current_screen == 4 {
            // Settings screen is at index 4
            // This is a simple increment, in a real implementation
            // we'd need to properly track and update the theme index
            self.theme_index = (self.theme_index + 1) % Theme::THEME_COUNT;
            self.theme = Theme::get_theme_by_index(self.theme_index);
        }
    }
    
    pub fn get_current_theme(&self) -> &Theme {
        &self.theme
    }
    
    pub async fn render(&mut self) {
        // Clear screen with theme background
        self.display.clear(self.theme.colors.background);
        
        // Draw header
        self.draw_header();
        
        // Draw current screen or menu
        if self.show_menu {
            self.draw_menu();
        } else {
            self.screens[self.current_screen].draw(&mut self.display, &self.theme);
        }
        
        // Update display
        self.display.flush().await;
    }
    
    pub async fn update(&mut self) {
        // Update current screen
        if !self.show_menu {
            self.screens[self.current_screen].update();
        }
    }
    
    fn draw_header(&mut self) {
        // Draw header bar
        self.display.fill_rect(0, 0, 320, 20, self.theme.colors.primary);
        
        // Draw navigation hints
        self.display.draw_text(5, 6, "<", Color::WHITE);
        self.display.draw_text(305, 6, ">", Color::WHITE);
        
        // Draw screen title centered
        let title = self.screens[self.current_screen].title();
        let text_width = title.len() * 6; // Approximate
        let x = (320 - text_width) / 2;
        self.display.draw_text(x as u16, 6, title, Color::WHITE);
        
        // Draw power indicator
        self.draw_power_indicator();
    }
    
    fn draw_power_indicator(&mut self) {
        // Placeholder for power indicator
        // Will integrate with BatteryMonitor later
        self.display.draw_text(250, 6, "USB", Color::CYAN);
    }
    
    fn draw_menu(&mut self) {
        // Draw menu overlay
        let x = 60;
        let y = 40;
        let w = 200;
        let h = 120;
        
        // Menu background
        self.display.fill_rect(x, y, w, h, self.theme.colors.surface);
        self.display.draw_rect(x, y, w, h, self.theme.colors.border);
        
        // Menu title
        self.display.draw_text(x + 70, y + 10, "MENU", self.theme.colors.text_primary);
        
        // Menu items
        let items = ["Display", "Update", "System", "Back"];
        for (i, item) in items.iter().enumerate() {
            let item_y = y + 35 + (i as u16 * 20);
            let color = if i == self.menu_selection {
                self.theme.colors.accent
            } else {
                self.theme.colors.text_secondary
            };
            self.display.draw_text(x + 20, item_y, item, color);
        }
    }
    
    pub fn next_screen(&mut self) {
        if !self.show_menu {
            self.current_screen = (self.current_screen + 1) % self.screens.len();
        } else {
            self.menu_selection = (self.menu_selection + 1) % 4;
        }
    }
    
    pub fn previous_screen(&mut self) {
        if !self.show_menu {
            self.current_screen = if self.current_screen == 0 {
                self.screens.len() - 1
            } else {
                self.current_screen - 1
            };
        } else {
            self.menu_selection = if self.menu_selection == 0 {
                3
            } else {
                self.menu_selection - 1
            };
        }
    }
    
    pub fn show_menu(&mut self) {
        self.show_menu = true;
        self.menu_selection = 0;
    }
    
    pub fn select(&mut self) {
        if self.show_menu {
            match self.menu_selection {
                0 => {} // Display settings
                1 => {} // Update
                2 => {} // System
                3 => self.show_menu = false, // Back
                _ => {}
            }
        }
    }
}