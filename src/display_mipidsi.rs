use anyhow::Result;

// Re-export colors module
pub mod colors {
    // Standard colors
    pub const BLACK: u16 = 0x0000;
    pub const WHITE: u16 = 0xFFFF;
    pub const YELLOW: u16 = 0xFFE0;
    
    // Theme colors  
    pub const PRIMARY_BLUE: u16 = 0x2589;
    pub const PRIMARY_GREEN: u16 = 0x07E5;
    pub const PRIMARY_PURPLE: u16 = 0x7817;
    pub const PRIMARY_RED: u16 = 0xF800;
    pub const SURFACE_LIGHT: u16 = 0x3186;
    pub const TEXT_PRIMARY: u16 = BLACK;
    pub const TEXT_SECONDARY: u16 = 0xBDF7;
    pub const BORDER_COLOR: u16 = 0x4208;
    pub const ACCENT_ORANGE: u16 = 0xC260;
    pub const SURFACE_DARK: u16 = 0x18E3;
}
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::primitives::{Rectangle, PrimitiveStyle};
use embedded_graphics::text::{Text, Baseline};
use embedded_graphics::mono_font::{ascii::FONT_6X12, MonoTextStyle};
use esp_idf_hal::gpio::{PinDriver, Output, AnyIOPin};
use esp_idf_hal::spi::{SpiDriver, SpiConfig};
use esp_idf_hal::delay::FreeRtos;
use mipidsi::Builder;
use display_interface_spi::SPIInterface;

// Display boundaries for T-Display-S3
const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 170;

// Convert our u16 colors to Rgb565
fn u16_to_rgb565(color: u16) -> Rgb565 {
    Rgb565::new(
        ((color >> 11) & 0x1F) as u8,
        ((color >> 5) & 0x3F) as u8,
        (color & 0x1F) as u8
    )
}

pub struct DisplayManager<'d> {
    display: mipidsi::Display<SPIInterface<SpiDriver<'d>, PinDriver<'d, AnyIOPin, Output>, PinDriver<'d, AnyIOPin, Output>>, mipidsi::models::ST7789, PinDriver<'d, AnyIOPin, Output>>,
    backlight_pin: PinDriver<'d, AnyIOPin, Output>,
    lcd_power_pin: PinDriver<'d, AnyIOPin, Output>,
}

impl<'d> DisplayManager<'d> {
    pub fn new(
        spi: SpiDriver<'d>,
        dc: impl Into<AnyIOPin> + 'd,
        cs: impl Into<AnyIOPin> + 'd,
        rst: impl Into<AnyIOPin> + 'd,
        backlight: impl Into<AnyIOPin> + 'd,
        lcd_power: impl Into<AnyIOPin> + 'd,
    ) -> Result<Self> {
        // Set up power and backlight pins
        let mut lcd_power_pin = PinDriver::output(lcd_power.into())?;
        lcd_power_pin.set_high()?;
        
        let mut backlight_pin = PinDriver::output(backlight.into())?;
        backlight_pin.set_high()?;
        
        // Small delay for power stabilization
        FreeRtos::delay_ms(100);
        
        // Set up SPI interface
        let dc_pin = PinDriver::output(dc.into())?;
        let cs_pin = PinDriver::output(cs.into())?;
        let spi_interface = SPIInterface::new(spi, dc_pin, cs_pin);
        
        // Set up reset pin
        let rst_pin = PinDriver::output(rst.into())?;
        
        // Create display using mipidsi
        let mut delay = FreeRtos;
        let mut display = Builder::st7789(spi_interface)
            .with_display_size(DISPLAY_HEIGHT, DISPLAY_WIDTH) // Note: width/height swapped for landscape
            .with_orientation(mipidsi::Orientation::Landscape)
            .with_invert_colors(mipidsi::ColorInversion::Inverted)
            .init(&mut delay, Some(rst_pin))?;
        
        // Clear the display
        display.clear(Rgb565::BLACK)?;
        
        Ok(Self {
            display,
            backlight_pin,
            lcd_power_pin,
        })
    }
    
    pub fn clear(&mut self, color: u16) -> Result<()> {
        self.display.clear(u16_to_rgb565(color))?;
        Ok(())
    }
    
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) -> Result<()> {
        let style = PrimitiveStyle::with_fill(u16_to_rgb565(color));
        Rectangle::new(
            Point::new(x as i32, y as i32),
            Size::new(w as u32, h as u32)
        )
        .into_styled(style)
        .draw(&mut self.display)?;
        Ok(())
    }
    
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<()> {
        self.display.set_pixel(x as u32, y as u32, u16_to_rgb565(color))?;
        Ok(())
    }
    
    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: u16, _bg_color: Option<u16>, scale: u8) -> Result<()> {
        let style = MonoTextStyle::new(&FONT_6X12, u16_to_rgb565(color));
        
        // Note: Scale is ignored for now, using fixed font
        Text::with_baseline(text, Point::new(x as i32, y as i32), style, Baseline::Top)
            .draw(&mut self.display)?;
        
        Ok(())
    }
    
    pub fn draw_text_centered(&mut self, y: u16, text: &str, color: u16, bg_color: Option<u16>, scale: u8) -> Result<()> {
        // Simple centering - can be improved
        let text_width = text.len() as u16 * 6; // Approximate width
        let x = (DISPLAY_WIDTH - text_width) / 2;
        self.draw_text(x, y, text, color, bg_color, scale)
    }
    
    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> Result<()> {
        use embedded_graphics::primitives::{Line, PrimitiveStyle};
        
        Line::new(Point::new(x0 as i32, y0 as i32), Point::new(x1 as i32, y1 as i32))
            .into_styled(PrimitiveStyle::with_stroke(u16_to_rgb565(color), 1))
            .draw(&mut self.display)?;
        
        Ok(())
    }
    
    pub fn draw_progress_bar(&mut self, x: u16, y: u16, width: u16, height: u16, progress: u8, 
                            bar_color: u16, bg_color: u16, border_color: u16) -> Result<()> {
        // Draw border
        self.fill_rect(x, y, width, height, border_color)?;
        // Draw background
        self.fill_rect(x + 1, y + 1, width - 2, height - 2, bg_color)?;
        // Draw progress
        let progress_width = ((width - 2) as u32 * progress as u32 / 100) as u16;
        if progress_width > 0 {
            self.fill_rect(x + 1, y + 1, progress_width, height - 2, bar_color)?;
        }
        Ok(())
    }
    
    pub fn width(&self) -> u16 {
        DISPLAY_WIDTH
    }
    
    pub fn height(&self) -> u16 {
        DISPLAY_HEIGHT
    }
    
    pub fn update_auto_dim(&mut self) -> Result<()> {
        // Keep backlight on
        self.backlight_pin.set_high()?;
        self.lcd_power_pin.set_high()?;
        Ok(())
    }
    
    pub fn reset_activity_timer(&mut self) {
        // No-op for now
    }
    
    pub fn flush(&mut self) -> Result<()> {
        // mipidsi handles flushing internally
        Ok(())
    }
}