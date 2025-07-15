// Button input handling with debouncing and event detection

use embassy_time::{Duration, Instant};
use esp_idf_hal::gpio::{AnyPin, Input, PinDriver};

// Button timing constants (matching Arduino implementation)
const DEBOUNCE_TIME: Duration = Duration::from_millis(50);
const LONG_PRESS_TIME: Duration = Duration::from_millis(800);
const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonEvent {
    None,
    Button1Click,
    Button1LongPress,
    Button1DoubleClick,
    Button2Click,
    Button2LongPress,
    Button2DoubleClick,
}

#[derive(Debug)]
struct ButtonState {
    current: bool,
    previous: bool,
    press_time: Option<Instant>,
    release_time: Option<Instant>,
    long_press_triggered: bool,
    click_count: u8,
    last_click_time: Option<Instant>,
}

impl ButtonState {
    fn new() -> Self {
        Self {
            current: true,  // Buttons are pulled up, so HIGH is unpressed
            previous: true,
            press_time: None,
            release_time: None,
            long_press_triggered: false,
            click_count: 0,
            last_click_time: None,
        }
    }
    
    fn update(&mut self, is_pressed: bool) -> ButtonEvent {
        self.previous = self.current;
        self.current = is_pressed;
        
        let now = Instant::now();
        
        // Button just pressed (HIGH to LOW transition)
        if !self.previous && self.current {
            self.press_time = Some(now);
            self.long_press_triggered = false;
            return ButtonEvent::None;
        }
        
        // Button just released (LOW to HIGH transition)
        if self.previous && !self.current {
            self.release_time = Some(now);
            
            if !self.long_press_triggered {
                // Check for double-click
                if let Some(last_click) = self.last_click_time {
                    if now.duration_since(last_click) < DOUBLE_CLICK_TIME {
                        self.click_count = 0;
                        self.last_click_time = None;
                        return ButtonEvent::Button1DoubleClick; // Will be mapped correctly by caller
                    }
                }
                
                // Single click
                self.click_count = 1;
                self.last_click_time = Some(now);
                return ButtonEvent::Button1Click; // Will be mapped correctly by caller
            }
            
            return ButtonEvent::None;
        }
        
        // Button held down - check for long press
        if self.current && !self.long_press_triggered {
            if let Some(press_time) = self.press_time {
                if now.duration_since(press_time) >= LONG_PRESS_TIME {
                    self.long_press_triggered = true;
                    return ButtonEvent::Button1LongPress; // Will be mapped correctly by caller
                }
            }
        }
        
        // Clear click count if timeout exceeded
        if let Some(last_click) = self.last_click_time {
            if now.duration_since(last_click) > DOUBLE_CLICK_TIME {
                self.click_count = 0;
                self.last_click_time = None;
            }
        }
        
        ButtonEvent::None
    }
}

pub struct ButtonManager {
    button1: PinDriver<'static, AnyPin, Input>,
    button2: PinDriver<'static, AnyPin, Input>,
    button1_state: ButtonState,
    button2_state: ButtonState,
    last_poll: Instant,
}

impl ButtonManager {
    pub fn new(
        button1: PinDriver<'static, AnyPin, Input>,
        button2: PinDriver<'static, AnyPin, Input>,
    ) -> Self {
        Self {
            button1,
            button2,
            button1_state: ButtonState::new(),
            button2_state: ButtonState::new(),
            last_poll: Instant::now(),
        }
    }
    
    pub fn poll(&mut self) -> Option<ButtonEvent> {
        let now = Instant::now();
        
        // Debounce check
        if now.duration_since(self.last_poll) < DEBOUNCE_TIME {
            return None;
        }
        
        self.last_poll = now;
        
        // Read button states (LOW = pressed for pull-up buttons)
        let button1_pressed = self.button1.is_low();
        let button2_pressed = self.button2.is_low();
        
        // Update button 1
        let event1 = self.button1_state.update(button1_pressed);
        if event1 != ButtonEvent::None {
            return Some(match event1 {
                ButtonEvent::Button1Click => ButtonEvent::Button1Click,
                ButtonEvent::Button1LongPress => ButtonEvent::Button1LongPress,
                ButtonEvent::Button1DoubleClick => ButtonEvent::Button1DoubleClick,
                _ => event1,
            });
        }
        
        // Update button 2
        let event2 = self.button2_state.update(button2_pressed);
        if event2 != ButtonEvent::None {
            return Some(match event2 {
                ButtonEvent::Button1Click => ButtonEvent::Button2Click,
                ButtonEvent::Button1LongPress => ButtonEvent::Button2LongPress,
                ButtonEvent::Button1DoubleClick => ButtonEvent::Button2DoubleClick,
                _ => event2,
            });
        }
        
        None
    }
    
    pub fn is_button1_pressed(&self) -> bool {
        self.button1.is_low()
    }
    
    pub fn is_button2_pressed(&self) -> bool {
        self.button2.is_low()
    }
}