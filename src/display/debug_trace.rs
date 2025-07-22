/// Display command trace module for debugging ST7789 communication
use esp_idf_sys::*;
use log::{debug, info};
use std::sync::Mutex;
use std::collections::VecDeque;
use lazy_static::lazy_static;
use std::os::raw::c_void;

#[derive(Debug, Clone)]
pub struct DisplayCommand {
    pub cmd: u8,
    pub data: Vec<u8>,
    pub timestamp: std::time::Instant,
    pub cmd_name: String,
}

lazy_static! {
    static ref COMMAND_HISTORY: Mutex<VecDeque<DisplayCommand>> = Mutex::new(VecDeque::with_capacity(100));
    static ref TRACE_ENABLED: Mutex<bool> = Mutex::new(true);
}

/// Map command codes to human-readable names
fn get_command_name(cmd: u8) -> &'static str {
    match cmd {
        0x00 => "NOP",
        0x01 => "SWRESET",
        0x04 => "RDDID",
        0x09 => "RDDST",
        0x0A => "RDDPM",
        0x0B => "RDDMADCTL",
        0x0C => "RDDCOLMOD",
        0x0D => "RDDIM",
        0x0E => "RDDSM",
        0x0F => "RDDSDR",
        0x10 => "SLPIN",
        0x11 => "SLPOUT",
        0x12 => "PTLON",
        0x13 => "NORON",
        0x20 => "INVOFF",
        0x21 => "INVON",
        0x26 => "GAMSET",
        0x28 => "DISPOFF",
        0x29 => "DISPON",
        0x2A => "CASET",
        0x2B => "RASET",
        0x2C => "RAMWR",
        0x2D => "RGBSET",
        0x2E => "RAMRD",
        0x30 => "PTLAR",
        0x33 => "SCRLAR",
        0x34 => "TEOFF",
        0x35 => "TEON",
        0x36 => "MADCTL",
        0x37 => "VSCSAD",
        0x38 => "IDMOFF",
        0x39 => "IDMON",
        0x3A => "COLMOD",
        0x3C => "WRMEMC",
        0x3D => "RDMEMC",
        0x3E => "STE",
        0x44 => "GSCAN",
        0x45 => "WRDISBV",
        0x51 => "WRCTRLD",
        0x53 => "WRCACE",
        0x55 => "WRCABC",
        0x5E => "WRCABCMB",
        0xB0 => "RDABCSDR",
        0xB1 => "RGBCTRL",
        0xB2 => "PORCTRL",
        0xB3 => "FRCTRL1",
        0xB4 => "PARCTRL",
        0xB5 => "GCTRL",
        0xB6 => "DFUNCTR",
        0xB7 => "GATECTRL",
        0xB8 => "GATEON",
        0xB9 => "VCOMS",
        0xBA => "POWSAVE",
        0xBB => "DLPOFFSAVE",
        0xBC => "DISPOFF",
        0xBD => "VRHS",
        0xBE => "VRHV",
        0xBF => "VDVS",
        0xC0 => "LCMCTRL",
        0xC1 => "IDSET",
        0xC2 => "VDVVRHEN",
        0xC3 => "VRHS",
        0xC4 => "VDVS",
        0xC5 => "VCMOFSET",
        0xC6 => "FRCTRL2",
        0xC7 => "CABCCTRL",
        0xC8 => "REGSEL1",
        0xC9 => "REGSEL2",
        0xCA => "REGSEL3",
        0xCB => "PWMFRSEL",
        0xCC => "PWCTRL1",
        0xCD => "VAPVANEN",
        0xCE => "CMD2BK0SEL1",
        0xCF => "CMD2BK0SEL2",
        0xD0 => "PWCTRL2",
        0xD1 => "CMD2BK1SEL1",
        0xD2 => "CMD2BK1SEL2",
        0xD3 => "CMD2BK3SEL1",
        0xD4 => "CMD2BK3SEL2",
        0xD7 => "GATECTRL2",
        0xD8 => "SPI2EN",
        0xD9 => "PVRFEN",
        0xDA => "RDID1",
        0xDB => "RDID2",
        0xDC => "RDID3",
        0xDD => "CMD2BK4SEL1",
        0xDE => "CMD2BK4SEL2",
        0xDF => "CMD2BK4SEL3",
        0xE0 => "PVGAMCTRL",
        0xE1 => "NVGAMCTRL",
        0xE2 => "DGMLUTR",
        0xE3 => "DGMLUTB",
        0xE4 => "GATECTRL3",
        0xE8 => "PWCTRL7",
        0xE9 => "SETEXTC",
        0xEA => "DFUNCTR2",
        0xEB => "SPI2EN2",
        0xEC => "SETINT",
        0xED => "PWCTRL8",
        0xEE => "CABCCTRL7",
        0xEF => "CABCCTRL8",
        0xF0 => "CABCCTRL9",
        0xF1 => "SETGAMMA",
        0xF2 => "SETDISP",
        0xF3 => "SETIMAGE",
        0xF4 => "SETDDBWRCTL",
        0xF5 => "SETDDBRDCTL",
        0xF6 => "SETMIPI",
        0xF7 => "SETSPI",
        0xF8 => "SETPOWER",
        0xFA => "SETVDC",
        0xFB => "SETID",
        0xFC => "SETMIPI2",
        0xFD => "SETCABC",
        0xFE => "SETDSI",
        0xFF => "SETPAGE",
        _ => "UNKNOWN",
    }
}

/// Enable or disable command tracing
pub fn set_trace_enabled(enabled: bool) {
    *TRACE_ENABLED.lock().unwrap() = enabled;
}

/// Get command history
pub fn get_command_history() -> Vec<DisplayCommand> {
    COMMAND_HISTORY.lock().unwrap().iter().cloned().collect()
}

/// Clear command history
pub fn clear_command_history() {
    COMMAND_HISTORY.lock().unwrap().clear();
}

/// Wrapper for esp_lcd_panel_io_tx_param with tracing
pub unsafe fn traced_lcd_panel_io_tx_param(
    io: *mut esp_lcd_panel_io_t,
    lcd_cmd: u32,
    lcd_params: *const c_void,
    param_size: usize,
) -> esp_err_t {
    if *TRACE_ENABLED.lock().unwrap() {
        let cmd = lcd_cmd as u8;
        let cmd_name = get_command_name(cmd);
        
        // Extract parameter data
        let mut data = Vec::new();
        if !lcd_params.is_null() && param_size > 0 {
            let param_slice = std::slice::from_raw_parts(lcd_params as *const u8, param_size);
            data.extend_from_slice(param_slice);
        }
        
        // Log the command
        if data.is_empty() {
            info!("[ST7789] CMD: 0x{:02X} ({}) - no params", cmd, cmd_name);
        } else {
            info!("[ST7789] CMD: 0x{:02X} ({}) - params: {:02X?}", cmd, cmd_name, data);
        }
        
        // Store in history
        let command = DisplayCommand {
            cmd,
            data: data.clone(),
            timestamp: std::time::Instant::now(),
            cmd_name: cmd_name.to_string(),
        };
        
        let mut history = COMMAND_HISTORY.lock().unwrap();
        if history.len() >= 100 {
            history.pop_front();
        }
        history.push_back(command);
    }
    
    // Call the actual function
    esp_lcd_panel_io_tx_param(io, lcd_cmd as i32, lcd_params, param_size)
}

/// Wrapper for esp_lcd_panel_io_tx_color with tracing
pub unsafe fn traced_lcd_panel_io_tx_color(
    io: *mut esp_lcd_panel_io_t,
    lcd_cmd: u32,
    color_data: *const c_void,
    color_size: usize,
) -> esp_err_t {
    // Skip logging RAMWR (0x2C) color data to reduce spam
    // These are pixel writes that happen thousands of times per frame
    if *TRACE_ENABLED.lock().unwrap() && lcd_cmd != 0x2C {
        // debug!("[ST7789] COLOR: 0x{:02X} - {} bytes", lcd_cmd, color_size);
    }
    
    // Call the actual function
    esp_lcd_panel_io_tx_color(io, lcd_cmd as i32, color_data, color_size)
}

/// Print command history summary
pub fn print_command_summary() {
    let history = COMMAND_HISTORY.lock().unwrap();
    info!("=== ST7789 Command History (last {} commands) ===", history.len());
    
    for (i, cmd) in history.iter().enumerate() {
        if cmd.data.is_empty() {
            info!("{:3}: 0x{:02X} ({})", i, cmd.cmd, cmd.cmd_name);
        } else {
            info!("{:3}: 0x{:02X} ({}) - {:02X?}", i, cmd.cmd, cmd.cmd_name, cmd.data);
        }
    }
}

/// Compare two command sequences
pub fn compare_sequences(working: &[DisplayCommand], current: &[DisplayCommand]) {
    info!("=== Command Sequence Comparison ===");
    
    let max_len = working.len().max(current.len());
    
    for i in 0..max_len {
        match (working.get(i), current.get(i)) {
            (Some(w), Some(c)) => {
                if w.cmd != c.cmd || w.data != c.data {
                    info!("DIFF at {}: Working: 0x{:02X} ({}) {:02X?} | Current: 0x{:02X} ({}) {:02X?}",
                        i, w.cmd, w.cmd_name, w.data, c.cmd, c.cmd_name, c.data);
                }
            }
            (Some(w), None) => {
                info!("MISSING at {}: Working has 0x{:02X} ({}), Current has nothing",
                    i, w.cmd, w.cmd_name);
            }
            (None, Some(c)) => {
                info!("EXTRA at {}: Current has 0x{:02X} ({}), Working has nothing",
                    i, c.cmd, c.cmd_name);
            }
            _ => {}
        }
    }
}