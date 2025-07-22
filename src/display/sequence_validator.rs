/// Command sequence validator for comparing ST7789 initialization sequences
use super::debug_trace::DisplayCommand;
use log::{info, warn, error};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CommandSequence {
    pub name: String,
    pub commands: Vec<DisplayCommand>,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub differences: Vec<SequenceDifference>,
    pub warnings: Vec<String>,
    pub critical_issues: Vec<String>,
}

#[derive(Debug)]
pub enum SequenceDifference {
    MissingCommand { position: usize, command: DisplayCommand },
    ExtraCommand { position: usize, command: DisplayCommand },
    DifferentParams { position: usize, expected: DisplayCommand, actual: DisplayCommand },
    DifferentOrder { expected_pos: usize, actual_pos: usize, command: DisplayCommand },
}

pub struct SequenceValidator;

impl SequenceValidator {
    /// Known working initialization sequence for T-Display-S3
    pub fn get_reference_sequence() -> CommandSequence {
        use std::time::Instant;
        
        let commands = vec![
            // Software reset
            DisplayCommand {
                cmd: 0x01,
                data: vec![],
                timestamp: Instant::now(),
                cmd_name: "SWRESET".to_string(),
            },
            // Sleep out
            DisplayCommand {
                cmd: 0x11,
                data: vec![],
                timestamp: Instant::now(),
                cmd_name: "SLPOUT".to_string(),
            },
            // Interface pixel format
            DisplayCommand {
                cmd: 0x3A,
                data: vec![0x55], // 16-bit RGB565
                timestamp: Instant::now(),
                cmd_name: "COLMOD".to_string(),
            },
            // Memory access control
            DisplayCommand {
                cmd: 0x36,
                data: vec![0x60], // Landscape mode for T-Display-S3
                timestamp: Instant::now(),
                cmd_name: "MADCTL".to_string(),
            },
            // Display inversion on
            DisplayCommand {
                cmd: 0x21,
                data: vec![],
                timestamp: Instant::now(),
                cmd_name: "INVON".to_string(),
            },
            // Normal display mode
            DisplayCommand {
                cmd: 0x13,
                data: vec![],
                timestamp: Instant::now(),
                cmd_name: "NORON".to_string(),
            },
            // Display on
            DisplayCommand {
                cmd: 0x29,
                data: vec![],
                timestamp: Instant::now(),
                cmd_name: "DISPON".to_string(),
            },
        ];
        
        CommandSequence {
            name: "T-Display-S3 Reference".to_string(),
            commands,
        }
    }
    
    /// Validate a command sequence against a reference
    pub fn validate_sequence(reference: &CommandSequence, actual: &CommandSequence) -> ValidationResult {
        let mut differences = Vec::new();
        let mut warnings = Vec::new();
        let mut critical_issues = Vec::new();
        
        // Build command lookup maps
        let ref_map: HashMap<u8, Vec<usize>> = Self::build_command_map(&reference.commands);
        let act_map: HashMap<u8, Vec<usize>> = Self::build_command_map(&actual.commands);
        
        // Check for critical commands
        Self::check_critical_commands(&reference.commands, &actual.commands, &mut critical_issues);
        
        // Check sequence order and parameters
        let max_len = reference.commands.len().max(actual.commands.len());
        
        for i in 0..max_len {
            match (reference.commands.get(i), actual.commands.get(i)) {
                (Some(ref_cmd), Some(act_cmd)) => {
                    if ref_cmd.cmd != act_cmd.cmd {
                        // Different command at this position
                        differences.push(SequenceDifference::DifferentOrder {
                            expected_pos: i,
                            actual_pos: Self::find_command_position(&actual.commands, ref_cmd.cmd).unwrap_or(usize::MAX),
                            command: ref_cmd.clone(),
                        });
                    } else if ref_cmd.data != act_cmd.data {
                        // Same command, different parameters
                        differences.push(SequenceDifference::DifferentParams {
                            position: i,
                            expected: ref_cmd.clone(),
                            actual: act_cmd.clone(),
                        });
                        
                        // Special handling for critical parameter differences
                        Self::check_critical_params(ref_cmd, act_cmd, &mut warnings, &mut critical_issues);
                    }
                }
                (Some(ref_cmd), None) => {
                    differences.push(SequenceDifference::MissingCommand {
                        position: i,
                        command: ref_cmd.clone(),
                    });
                }
                (None, Some(act_cmd)) => {
                    differences.push(SequenceDifference::ExtraCommand {
                        position: i,
                        command: act_cmd.clone(),
                    });
                }
                _ => {}
            }
        }
        
        // Check timing requirements
        Self::check_timing_requirements(&actual.commands, &mut warnings);
        
        ValidationResult {
            differences,
            warnings,
            critical_issues,
        }
    }
    
    /// Print validation results
    pub fn print_validation_results(result: &ValidationResult) {
        info!("=== ST7789 Command Sequence Validation ===");
        
        if result.critical_issues.is_empty() && result.differences.is_empty() {
            info!("✓ Sequence matches reference!");
            return;
        }
        
        // Print critical issues
        if !result.critical_issues.is_empty() {
            error!("CRITICAL ISSUES:");
            for issue in &result.critical_issues {
                error!("  ✗ {}", issue);
            }
        }
        
        // Print differences
        if !result.differences.is_empty() {
            warn!("DIFFERENCES:");
            for diff in &result.differences {
                match diff {
                    SequenceDifference::MissingCommand { position, command } => {
                        warn!("  - Missing at {}: {} (0x{:02X})", position, command.cmd_name, command.cmd);
                    }
                    SequenceDifference::ExtraCommand { position, command } => {
                        warn!("  + Extra at {}: {} (0x{:02X})", position, command.cmd_name, command.cmd);
                    }
                    SequenceDifference::DifferentParams { position, expected, actual } => {
                        warn!("  ≠ Different params at {}: {} expected {:02X?} but got {:02X?}", 
                            position, expected.cmd_name, expected.data, actual.data);
                    }
                    SequenceDifference::DifferentOrder { expected_pos, actual_pos, command } => {
                        warn!("  ↔ Wrong position: {} expected at {} but found at {}", 
                            command.cmd_name, expected_pos, actual_pos);
                    }
                }
            }
        }
        
        // Print warnings
        if !result.warnings.is_empty() {
            warn!("WARNINGS:");
            for warning in &result.warnings {
                warn!("  ⚠ {}", warning);
            }
        }
    }
    
    /// Build a map of command codes to their positions
    fn build_command_map(commands: &[DisplayCommand]) -> HashMap<u8, Vec<usize>> {
        let mut map = HashMap::new();
        for (i, cmd) in commands.iter().enumerate() {
            map.entry(cmd.cmd).or_insert_with(Vec::new).push(i);
        }
        map
    }
    
    /// Find position of a command in the sequence
    fn find_command_position(commands: &[DisplayCommand], cmd_code: u8) -> Option<usize> {
        commands.iter().position(|c| c.cmd == cmd_code)
    }
    
    /// Check for critical commands that must be present
    fn check_critical_commands(reference: &[DisplayCommand], actual: &[DisplayCommand], issues: &mut Vec<String>) {
        let critical_cmds = [
            (0x01, "SWRESET - Software reset"),
            (0x11, "SLPOUT - Sleep out"),
            (0x3A, "COLMOD - Pixel format"),
            (0x36, "MADCTL - Display orientation"),
            (0x29, "DISPON - Display on"),
        ];
        
        for (cmd_code, desc) in &critical_cmds {
            if !actual.iter().any(|c| c.cmd == *cmd_code) {
                issues.push(format!("Missing critical command: {}", desc));
            }
        }
    }
    
    /// Check critical parameter values
    fn check_critical_params(expected: &DisplayCommand, actual: &DisplayCommand, 
                           warnings: &mut Vec<String>, issues: &mut Vec<String>) {
        match expected.cmd {
            0x36 => { // MADCTL
                if !expected.data.is_empty() && !actual.data.is_empty() {
                    let exp_val = expected.data[0];
                    let act_val = actual.data[0];
                    
                    // Check RGB/BGR bit
                    if (exp_val & 0x08) != (act_val & 0x08) {
                        issues.push("MADCTL: RGB/BGR order mismatch!".to_string());
                    }
                    
                    // Check rotation bits
                    if (exp_val & 0xE0) != (act_val & 0xE0) {
                        warnings.push("MADCTL: Different rotation settings".to_string());
                    }
                }
            }
            0x3A => { // COLMOD
                if !expected.data.is_empty() && !actual.data.is_empty() {
                    if expected.data[0] != actual.data[0] {
                        issues.push(format!("COLMOD: Pixel format mismatch! Expected 0x{:02X}, got 0x{:02X}", 
                            expected.data[0], actual.data[0]));
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Check timing requirements between commands
    fn check_timing_requirements(commands: &[DisplayCommand], warnings: &mut Vec<String>) {
        // Find SWRESET and check if SLPOUT follows with enough delay
        if let Some(reset_pos) = commands.iter().position(|c| c.cmd == 0x01) {
            if let Some(slpout_pos) = commands.iter().position(|c| c.cmd == 0x11) {
                if slpout_pos == reset_pos + 1 {
                    warnings.push("SLPOUT immediately after SWRESET - needs 120ms delay".to_string());
                }
            }
        }
        
        // Check for DISPON timing
        if let Some(slpout_pos) = commands.iter().position(|c| c.cmd == 0x11) {
            if let Some(dispon_pos) = commands.iter().position(|c| c.cmd == 0x29) {
                if dispon_pos == slpout_pos + 1 {
                    warnings.push("DISPON immediately after SLPOUT - needs 120ms delay".to_string());
                }
            }
        }
    }
    
    /// Generate fix suggestions based on validation results
    pub fn suggest_fixes(result: &ValidationResult) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Suggest fixes for critical issues
        for issue in &result.critical_issues {
            if issue.contains("RGB/BGR") {
                suggestions.push("Fix MADCTL RGB/BGR bit: Change bit 3 in MADCTL parameter".to_string());
            }
            if issue.contains("Pixel format") {
                suggestions.push("Fix COLMOD: Use 0x55 for RGB565 16-bit format".to_string());
            }
            if issue.contains("Missing critical command") {
                suggestions.push(format!("Add missing command: {}", issue));
            }
        }
        
        // Suggest timing fixes
        for warning in &result.warnings {
            if warning.contains("needs 120ms delay") {
                suggestions.push("Add delay after command: use Ets::delay_ms(120)".to_string());
            }
        }
        
        suggestions
    }
}