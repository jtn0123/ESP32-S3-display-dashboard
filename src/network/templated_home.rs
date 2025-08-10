/// Home page handler using template engine
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;
use std::collections::HashMap;

use super::template_engine::TemplateEngine;

// Include templates as bytes at compile time
const HOME_TEMPLATE: &str = include_str!("../templates/home_template.html");
const HEADER_PARTIAL: &str = include_str!("../templates/partials/header.html");
const METRICS_PARTIAL: &str = include_str!("../templates/partials/metrics.html");

// CSS with theme support
const STYLES: &str = r#"<style>
    :root {
        --bg-primary: #ffffff;
        --bg-secondary: #f9fafb;
        --text-primary: #111827;
        --text-secondary: #6b7280;
        --border-color: #e5e7eb;
        --shadow: 0 4px 6px rgba(0, 0, 0, 0.07);
        --accent: #3b82f6;
        --accent-hover: #2563eb;
    }
    
    [data-theme="dark"] {
        --bg-primary: #1f2937;
        --bg-secondary: #0f172a;
        --text-primary: #f3f4f6;
        --text-secondary: #9ca3af;
        --border-color: #374151;
        --shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
        --accent: #60a5fa;
        --accent-hover: #3b82f6;
    }
    
    body { 
        font-family: -apple-system, system-ui, sans-serif; 
        background: var(--bg-secondary); 
        margin: 0; 
        padding: 20px; 
        color: var(--text-primary);
        transition: background-color 0.3s, color 0.3s;
    }
    .container { max-width: 1200px; margin: 0 auto; }
    .header { 
        background: var(--bg-primary); 
        border-radius: 12px; 
        padding: 24px; 
        margin-bottom: 20px; 
        box-shadow: var(--shadow);
        position: relative;
    }
    .header h1 { margin: 0 0 8px 0; font-size: 2rem; color: var(--text-primary); }
    .header p { margin: 0; color: var(--text-secondary); }
    .card { 
        background: var(--bg-primary); 
        border-radius: 12px; 
        padding: 24px; 
        margin-bottom: 20px; 
        box-shadow: var(--shadow);
    }
    .card h2 { margin: 0 0 16px 0; font-size: 1.5rem; color: var(--text-primary); }
    .metric { display: flex; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid var(--border-color); }
    .metric:last-child { border-bottom: none; }
    .metric-label { font-weight: 500; color: var(--text-primary); }
    .metric-value { color: var(--accent); font-family: monospace; }
    .status-healthy { color: #10b981; }
    .status-warning { color: #f59e0b; }
    .status-critical { color: #ef4444; }
    .button { 
        display: inline-block; 
        background: var(--accent); 
        color: white; 
        padding: 10px 20px; 
        border-radius: 8px; 
        text-decoration: none; 
        margin-top: 16px; 
        transition: background-color 0.2s;
    }
    .button:hover { background: var(--accent-hover); }
    .theme-toggle {
        position: absolute;
        top: 24px;
        right: 24px;
        background: var(--accent);
        color: white;
        border: none;
        border-radius: 8px;
        padding: 8px 16px;
        cursor: pointer;
        font-size: 14px;
        transition: background-color 0.2s;
    }
    .theme-toggle:hover { background: var(--accent-hover); }
</style>
<script>
    const theme = localStorage.getItem('theme') || 'light';
    document.documentElement.setAttribute('data-theme', theme);
    
    function toggleTheme() {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';
        document.documentElement.setAttribute('data-theme', newTheme);
        localStorage.setItem('theme', newTheme);
        updateThemeButton(newTheme);
    }
    
    function updateThemeButton(theme) {
        const button = document.getElementById('themeToggle');
        if (button) {
            button.textContent = theme === 'light' ? 'Dark' : 'Light';
        }
    }
    
    window.addEventListener('DOMContentLoaded', function() {
        const theme = document.documentElement.getAttribute('data-theme');
        updateThemeButton(theme);
    });
</script>"#;

/// Handle home page using template engine
pub fn handle_home_templated(req: Request<&mut EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    crate::memory_diagnostics::log_memory_state("Templated home - start");
    
    // Get system info
    let version = crate::version::DISPLAY_VERSION;
    let free_heap = unsafe { esp_idf_sys::esp_get_free_heap_size() };
    let uptime_ms = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as u64 };
    
    // Format uptime
    let uptime_secs = uptime_ms / 1000;
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;
    
    // Get memory stats
    let mem_stats = crate::memory_diagnostics::MemoryStats::current();
    
    // Create template variables
    let mut vars = HashMap::new();
    vars.insert("page_title", String::from("ESP32-S3 Dashboard"));
    vars.insert("title", String::from("ESP32-S3 Dashboard"));
    vars.insert("version", String::from(version));
    vars.insert("uptime", format!("{}h {}m {}s", hours, minutes, seconds));
    vars.insert("free_memory", format!("{} KB", free_heap / 1024));
    vars.insert("dram_info", format!("{} KB (largest: {} KB)", 
        mem_stats.internal_free_kb, 
        mem_stats.internal_largest_kb));
    vars.insert("dram_style", if mem_stats.internal_largest_kb < 4 {
        String::from("style=\"color: #ef4444\"")
    } else {
        String::new()
    });
    vars.insert("psram_info", format!("{} KB", mem_stats.psram_free_kb));
    
    // Create partials map
    let mut partials = HashMap::new();
    partials.insert("styles", STYLES);
    partials.insert("header", HEADER_PARTIAL);
    partials.insert("metrics", METRICS_PARTIAL);
    partials.insert("navbar", include_str!("../templates/partials/navbar.html"));
    
    // Prepare active flags for navbar
    let mut flags = HashMap::new();
    flags.insert("HOME_ACTIVE", "class=\"active\"");
    flags.insert("DASH_ACTIVE", "");
    flags.insert("LOGS_ACTIVE", "");
    flags.insert("FILES_ACTIVE", "");
    flags.insert("OTA_ACTIVE", "");
    flags.insert("DEV_ACTIVE", "");

    // Render the template
    let html = TemplateEngine::render_with_partials_and_flags(HOME_TEMPLATE, &vars, &partials, &flags);
    
    // Send response
    let response_bytes = html.as_bytes();
    let mut response = req.into_response(
        200,
        Some("OK"),
        &[
            ("Content-Type", "text/html; charset=utf-8"),
            ("Content-Length", &response_bytes.len().to_string()),
            ("Connection", "close"),
        ]
    )?;
    
    response.write_all(response_bytes)?;
    
    crate::memory_diagnostics::log_memory_state("Templated home - complete");
    
    Ok(())
}