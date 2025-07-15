use anyhow::Result;
use esp_idf_hal::gpio::{PinDriver, Input, Pull, AnyIOPin};
use std::time::{Duration, Instant};

const DEBOUNCE_TIME: Duration = Duration::from_millis(50);
const LONG_PRESS_TIME: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonEvent {
    Button1Press,
    Button1Release,
    Button1Click,
    Button1LongPress,
    Button2Press,
    Button2Release,
    Button2Click,
    Button2LongPress,
}

pub struct ButtonManager {
    button1: PinDriver<'static, AnyIOPin, Input>,
    button2: PinDriver<'static, AnyIOPin, Input>,
    button1_state: ButtonState,
    button2_state: ButtonState,
}

struct ButtonState {
    pressed: bool,
    press_time: Option<Instant>,
    last_change: Instant,
    long_press_fired: bool,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            pressed: false,
            press_time: None,
            last_change: Instant::now(),
            long_press_fired: false,
        }
    }
}

impl ButtonManager {
    pub fn new(
        button1_pin: impl Into<AnyIOPin> + 'static,
        button2_pin: impl Into<AnyIOPin> + 'static,
    ) -> Result<Self> {
        let mut button1 = PinDriver::input(button1_pin.into())?;
        let mut button2 = PinDriver::input(button2_pin.into())?;
        
        // Set pull-up resistors
        button1.set_pull(Pull::Up)?;
        button2.set_pull(Pull::Up)?;

        Ok(Self {
            button1,
            button2,
            button1_state: ButtonState::default(),
            button2_state: ButtonState::default(),
        })
    }

    pub fn poll(&mut self) -> Option<ButtonEvent> {
        // Check button 1
        let button1_pressed = self.button1.is_low(); // Active low
        if let Some(event) = Self::check_button_state(button1_pressed, &mut self.button1_state, 1) {
            return Some(event);
        }

        // Check button 2
        let button2_pressed = self.button2.is_low(); // Active low
        if let Some(event) = Self::check_button_state(button2_pressed, &mut self.button2_state, 2) {
            return Some(event);
        }

        None
    }

    fn check_button_state(
        pressed: bool,
        state: &mut ButtonState,
        button_num: u8,
    ) -> Option<ButtonEvent> {
        let now = Instant::now();

        // Debounce
        if now.duration_since(state.last_change) < DEBOUNCE_TIME {
            return None;
        }

        // State change detection
        if pressed != state.pressed {
            state.last_change = now;
            state.pressed = pressed;

            if pressed {
                // Button pressed
                state.press_time = Some(now);
                state.long_press_fired = false;
                
                return Some(match button_num {
                    1 => ButtonEvent::Button1Press,
                    2 => ButtonEvent::Button2Press,
                    _ => unreachable!(),
                });
            } else {
                // Button released
                let press_duration = state.press_time
                    .map(|t| now.duration_since(t))
                    .unwrap_or(Duration::ZERO);

                state.press_time = None;

                // Generate click event if not a long press
                if press_duration < LONG_PRESS_TIME && !state.long_press_fired {
                    return Some(match button_num {
                        1 => ButtonEvent::Button1Click,
                        2 => ButtonEvent::Button2Click,
                        _ => unreachable!(),
                    });
                }

                return Some(match button_num {
                    1 => ButtonEvent::Button1Release,
                    2 => ButtonEvent::Button2Release,
                    _ => unreachable!(),
                });
            }
        }

        // Check for long press
        if pressed && !state.long_press_fired {
            if let Some(press_time) = state.press_time {
                if now.duration_since(press_time) >= LONG_PRESS_TIME {
                    state.long_press_fired = true;
                    return Some(match button_num {
                        1 => ButtonEvent::Button1LongPress,
                        2 => ButtonEvent::Button2LongPress,
                        _ => unreachable!(),
                    });
                }
            }
        }

        None
    }
}