// Screen modules

mod system;
mod power;
mod wifi;
mod hardware;
mod settings;

pub use system::SystemScreen;
pub use power::PowerScreen;
pub use wifi::WiFiScreen;
pub use hardware::HardwareScreen;
pub use settings::SettingsScreen;

use crate::display::Display;
use crate::ui::theme::Theme;

// Trait that all screens must implement
pub trait Screen {
    fn title(&self) -> &str;
    fn draw(&self, display: &mut Display, theme: &Theme);
    fn update(&mut self);
    fn handle_select(&mut self) {}
}