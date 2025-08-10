#![allow(dead_code)]
use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::io::Write;

/// Optimized streaming dashboard that sends the response in chunks
/// to avoid memory exhaustion on the ESP32
pub fn handle_dashboard_streaming(req: Request<&mut EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    // Send response headers first
    let headers = [
        ("Content-Type", "text/html; charset=utf-8"),
        ("Cache-Control", "no-cache"),
    ];
    
    let mut response = req.into_response(200, Some("OK"), &headers)?;
    
    // Stream the dashboard in smaller chunks
    
    // Part 1: HTML head and minimal critical CSS
    response.write_all(br#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ESP32-S3 Dashboard</title>
    <style>
        /* Critical CSS only - rest loads async */
        body{margin:0;background:#0a0a0a;color:#f9fafb;font-family:system-ui}
        .loading{display:flex;align-items:center;justify-content:center;height:100vh}
        .spinner{border:3px solid #374151;border-top-color:#3b82f6;border-radius:50%;width:40px;height:40px;animation:spin 1s linear infinite}
        @keyframes spin{to{transform:rotate(360deg)}}
    </style>
</head>
<body>
    <div class="loading" id="loader">
        <div class="spinner"></div>
    </div>
    <div id="app" style="display:none">
"#)?;
    
    // Part 2: Main dashboard structure
    response.write_all(br#"
        <div class="header">
            <h1>ESP32-S3 Dashboard</h1>
            <div class="status" id="status">Connecting...</div>
        </div>
        
        <div class="container">
            <div class="grid">
"#)?;
    
    // Part 3: System Info Card
    response.write_all(br#"
                <div class="card">
                    <h2>System Information</h2>
                    <div class="info-grid">
                        <div class="info-item">
                            <span class="label">Uptime:</span>
                            <span class="value" id="uptime">--:--:--</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Free Heap:</span>
                            <span class="value" id="heap">-- KB</span>
                        </div>
                        <div class="info-item">
                            <span class="label">CPU Usage:</span>
                            <span class="value" id="cpu">--%</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Temperature:</span>
                            <span class="value" id="temp">--C</span>
                        </div>
                    </div>
                </div>
"#)?;
    
    // Part 4: Network Card
    response.write_all(br#"
                <div class="card">
                    <h2>Network</h2>
                    <div class="info-grid">
                        <div class="info-item">
                            <span class="label">SSID:</span>
                            <span class="value" id="ssid">--</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Signal:</span>
                            <span class="value" id="rssi">-- dBm</span>
                        </div>
                        <div class="info-item">
                            <span class="label">IP Address:</span>
                            <span class="value" id="ip">--</span>
                        </div>
                    </div>
                </div>
"#)?;
    
    // Part 5: Performance Card (simplified)
    response.write_all(br#"
                <div class="card">
                    <h2>Performance</h2>
                    <div class="info-grid">
                        <div class="info-item">
                            <span class="label">Display FPS:</span>
                            <span class="value" id="fps">-- fps</span>
                        </div>
                        <div class="info-item">
                            <span class="label">Render Time:</span>
                            <span class="value" id="render">-- ms</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="metric-card">
            <h3>Display & Power Controls</h3>
            <div class="controls">
                <div class="cpu-core">
                    <div class="core-label">Brightness <span id="dc_brightness_val" style="float:right">--</span></div>
                    <input id="dc_brightness" type="range" min="0" max="255" value="128" />
                </div>
                <div class="cpu-core" style="display:flex; align-items:center; gap:.5rem">
                    <input type="checkbox" id="dc_display" />
                    <label for="dc_display">Display Power</label>
                </div>
                <div class="cpu-core" style="display:flex; align-items:center; gap:.5rem">
                    <input type="checkbox" id="dc_autodim" />
                    <label for="dc_autodim">Auto-dim when idle</label>
                </div>
                <div class="cpu-core">
                    <div class="core-label">Performance Mode</div>
                    <select id="dc_mode">
                        <option value="eco">Eco</option>
                        <option value="normal" selected>Normal</option>
                        <option value="turbo">Turbo</option>
                    </select>
                </div>
                <div class="cpu-core">
                    <div class="core-label">Dim Timeout <span id="dc_dim_val" style="float:right">--</span></div>
                    <input id="dc_dim_timeout" type="range" min="10" max="300" step="10" value="30" />
                </div>
                <div class="cpu-core">
                    <div class="core-label">Sleep Timeout <span id="dc_sleep_val" style="float:right">--</span></div>
                    <input id="dc_sleep_timeout" type="range" min="60" max="1800" step="60" value="300" />
                </div>
                <div class="cpu-core" style="display:flex; gap:.5rem; flex-wrap:wrap">
                    <button id="dc_save_power" class="theme-toggle">Save Power Settings</button>
                    <button id="dc_restart" class="theme-toggle">Restart</button>
                </div>
            </div>
        </div>

        <div class="metric-card">
            <h3>Battery</h3>
            <div class="network-info">
                <div class="network-item">
                    <span class="network-label">Level</span>
                    <span class="network-value" id="battery_percent">--%</span>
                </div>
                <div class="network-item">
                    <span class="network-label">Status</span>
                    <span class="network-value" id="battery_status">--</span>
                </div>
            </div>
        </div>
    </div>
"#)?;
    
    // Part 6: Minimal JavaScript
    response.write_all(br#"
    <script>
    // Minimal dashboard functionality
    let updateInterval;
    
    async function updateDashboard() {
        try {
            const [systemRes, metricsRes] = await Promise.all([
                fetch('/api/system'),
                fetch('/api/metrics')
            ]);
            
            if (!systemRes.ok || !metricsRes.ok) {
                document.getElementById('status').textContent = 'Error';
                return;
            }
            
            const system = await systemRes.json();
            const metrics = await metricsRes.json();
            
            // Update system info
            document.getElementById('uptime').textContent = formatUptime(system.uptime_ms);
            document.getElementById('heap').textContent = Math.round(system.free_heap / 1024) + ' KB';
            document.getElementById('ssid').textContent = system.ssid;
            
            // Update metrics
            document.getElementById('cpu').textContent = metrics.cpu_usage.toFixed(1) + '%';
            document.getElementById('temp').textContent = metrics.temperature.toFixed(1) + '\u00B0C';
            document.getElementById('rssi').textContent = metrics.wifi_rssi + ' dBm';
            document.getElementById('fps').textContent = metrics.fps_actual.toFixed(1) + ' fps';
            document.getElementById('render').textContent = metrics.render_time_ms + ' ms';
            
            document.getElementById('status').textContent = 'Connected';
            document.getElementById('status').style.color = '#10b981';
            
        } catch (error) {
            console.error('Update failed:', error);
            document.getElementById('status').textContent = 'Connection Error';
            document.getElementById('status').style.color = '#ef4444';
        }
    }
    
    function formatUptime(ms) {
        const seconds = Math.floor(ms / 1000);
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;
        return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }
    
    // Start updates when page loads
    window.addEventListener('load', () => {
        // Hide loader, show app
        document.getElementById('loader').style.display = 'none';
        document.getElementById('app').style.display = 'block';
        
        // Load full CSS asynchronously
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = '/dashboard.css';
        document.head.appendChild(link);
        
        // Start updates
        updateDashboard();
        updateInterval = setInterval(updateDashboard, 2000);
    });
    
    // Cleanup on page unload
    window.addEventListener('beforeunload', () => {
        if (updateInterval) clearInterval(updateInterval);
    });
    </script>
"#)?;
    
    // Part 7: Inline critical styles (minimal)
    response.write_all(br#"
    <style>
        .header{background:#1a1a1a;padding:1rem;border-bottom:1px solid #374151;display:flex;justify-content:space-between;align-items:center}
        .container{padding:1rem;max-width:1200px;margin:0 auto}
        .grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:1rem}
        .card{background:#1a1a1a;border:1px solid #374151;border-radius:8px;padding:1.5rem}
        .card h2{margin-bottom:1rem;color:#3b82f6}
        .info-grid{display:grid;gap:0.75rem}
        .info-item{display:flex;justify-content:space-between}
        .label{color:#9ca3af}
        .value{font-weight:600}
        #status{font-size:0.875rem;color:#10b981}
    </style>
</body>
</html>"#)?;
    
    Ok(())
}

/// Serve the full CSS file separately (cached)
pub fn handle_dashboard_css(req: Request<&mut EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    // This would contain the full dashboard CSS
    // For now, return a minimal version
    let css = r#"
/* Full dashboard styles */
:root {
    --bg-main: #0a0a0a;
    --bg-card: #1a1a1a;
    --bg-hover: #2a2a2a;
    --accent: #3b82f6;
    --success: #10b981;
    --warning: #f59e0b;
    --danger: #ef4444;
    --text: #f9fafb;
    --text-dim: #9ca3af;
    --border: #374151;
}

/* Additional styles would go here */
"#;
    
    let headers = [
        ("Content-Type", "text/css"),
        ("Cache-Control", "public, max-age=3600"),
    ];
    
    let mut response = req.into_response(200, Some("OK"), &headers)?;
    response.write_all(css.as_bytes())?;
    
    Ok(())
}

/// Handle the enhanced dashboard with SSE support and modern UI
pub fn handle_dashboard_enhanced(req: Request<&mut EspHttpConnection>) -> Result<(), Box<dyn std::error::Error>> {
    // Send response headers first
    let headers = [
        ("Content-Type", "text/html; charset=utf-8"),
        ("Cache-Control", "no-cache"),
    ];
    
    let mut response = req.into_response(200, Some("OK"), &headers)?;
    
    // Stream the enhanced dashboard in chunks to avoid memory issues
    
    // Part 1: DOCTYPE and head with CSS
    response.write_all(br#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ESP32-S3 Dashboard</title>
    <style>
        /* Theme System */
        :root[data-theme="light"] {
            --bg-main: #ffffff;
            --bg-card: #f9fafb;
            --bg-hover: #f3f4f6;
            --accent: #3b82f6;
            --success: #10b981;
            --warning: #f59e0b;
            --danger: #ef4444;
            --text: #111827;
            --text-dim: #6b7280;
            --border: #e5e7eb;
        }
        :root[data-theme="dark"] {
            --bg-main: #0a0a0a;
            --bg-card: #1a1a1a;
            --bg-hover: #2a2a2a;
            --accent: #3b82f6;
            --success: #10b981;
            --warning: #f59e0b;
            --danger: #ef4444;
            --text: #f9fafb;
            --text-dim: #9ca3af;
            --border: #374151;
        }
        /* Default dark */
        :root {
            --bg-main: #0a0a0a;
            --bg-card: #1a1a1a;
            --text: #f9fafb;
            --text-dim: #9ca3af;
            --border: #374151;
        }
        body {
            margin: 0;
            background: var(--bg-main);
            color: var(--text);
            font-family: system-ui, -apple-system, sans-serif;
            line-height: 1.5;
        }
    </style>
</head>
<body>
"#)?;
    
    // Part 2: Navigation bar with theme toggle
    response.write_all(br#"
    <nav class="navbar">
        <div class="nav-brand">ESP32-S3 Dashboard</div>
        <div class="nav-links">
            <!-- Global Navbar: Control merged into Dashboard -->
            <a href="/">Home</a>
            <a href="/dashboard" class="active">Dashboard</a>
            <a href="/logs">Logs</a>
            <a href="/files">Files</a>
            <a href="/ota">Update</a>
            <a href="/dev">Dev Tools</a>
        </div>
        <div style="display:flex; gap:.5rem; align-items:center">
            <button class="theme-toggle" id="themeToggle" title="Toggle theme">
                <span class="theme-icon">&#x1F319;</span>
            </button>
            <button class="theme-toggle" id="restartBtn" title="Restart device">Restart</button>
        </div>
    </nav>
"#)?;
    
    // Part 3: Quick stats bar
    response.write_all(br#"
    <div class="quick-stats">
        <div class="stat">
            <span class="stat-label">Uptime</span>
            <span class="stat-value" id="uptime">--:--:--</span>
        </div>
        <div class="stat">
            <span class="stat-label">WiFi</span>
            <span class="stat-value" id="wifi-status">--</span>
        </div>
        <div class="stat">
            <span class="stat-label">Power</span>
            <span class="stat-value" id="power-mode">Normal</span>
        </div>
        <div class="stat">
            <span class="stat-label">Version</span>
            <span class="stat-value" id="fw-version">--</span>
        </div>
        <div class="stat">
            <span class="stat-label">IP</span>
            <span class="stat-value" id="ip-address">--</span>
        </div>
    </div>
"#)?;
    
    // Part 4: Main dashboard container
    response.write_all(br#"
    <div class="dashboard-container">
        <div class="metrics-grid">
"#)?;
    
    // Part 5: CPU card
    response.write_all(br#"
            <div class="metric-card">
                <h3>CPU Usage</h3>
                <div class="cpu-cores">
                    <div class="cpu-core">
                        <div class="core-label">Core 0</div>
                        <div class="progress-bar">
                            <div class="progress-fill" id="cpu0-bar" style="width: 0%"></div>
                        </div>
                        <span class="core-value" id="cpu0-usage">0%</span>
                    </div>
                    <div class="cpu-core">
                        <div class="core-label">Core 1</div>
                        <div class="progress-bar">
                            <div class="progress-fill" id="cpu1-bar" style="width: 0%"></div>
                        </div>
                        <span class="core-value" id="cpu1-usage">0%</span>
                    </div>
                </div>
                <div class="metric-footer">
                    <span id="cpu-freq">-- MHz</span>
                    <span id="cpu-temp">--&deg;C</span>
                </div>
            </div>
"#)?;
    
    // Part 6: Memory card
    response.write_all(br#"
            <div class="metric-card">
                <h3>Memory</h3>
                <div class="memory-bars">
                    <div class="memory-item">
                        <div class="memory-header">
                            <span>Heap</span>
                            <span id="heap-free">-- KB</span>
                        </div>
                        <div class="progress-bar">
                            <div class="progress-fill success" id="heap-bar" style="width: 50%"></div>
                        </div>
                    </div>
                    <div class="memory-item">
                        <div class="memory-header">
                            <span>PSRAM</span>
                            <span id="psram-free">-- KB</span>
                        </div>
                        <div class="progress-bar">
                            <div class="progress-fill success" id="psram-bar" style="width: 50%"></div>
                        </div>
                    </div>
                </div>
                <div class="metric-footer">
                    <span>Fragmentation: <span id="heap-frag">0%</span></span>
                </div>
            </div>
"#)?;
    
    // Part 7: Performance card (simplified)
    response.write_all(br#"
            <div class="metric-card">
                <h3>Performance</h3>
                <div class="perf-stats">
                    <div class="perf-item">
                        <span class="perf-label">Display FPS</span>
                        <span class="perf-value" id="fps">-- fps</span>
                    </div>
                    <div class="perf-item">
                        <span class="perf-label">Frame Skip</span>
                        <span class="perf-value" id="skip-rate">--%</span>
                    </div>
                    <div class="perf-item">
                        <span class="perf-label">Render Time</span>
                        <span class="perf-value" id="render-time">-- ms</span>
                    </div>
                </div>
            </div>
"#)?;
    
    // Part 8: Network card
    response.write_all(br#"
            <div class="metric-card">
                <h3>Network</h3>
                <div class="network-info">
                    <div class="network-item">
                        <span class="network-label">SSID</span>
                        <span class="network-value" id="ssid">--</span>
                    </div>
                    <div class="network-item">
                        <span class="network-label">Signal</span>
                        <span class="network-value" id="rssi">-- dBm</span>
                    </div>
                    <div class="network-item">
                        <span class="network-label">SSE Status</span>
                        <span class="network-value" id="sse-status">Disconnected</span>
                    </div>
                </div>
            </div>
            <div class="metric-card">
                <h3>System Health</h3>
                <div class="perf-stats">
                    <div class="perf-item">
                        <span class="perf-label">Uptime</span>
                        <span class="perf-value" id="uptime">--:--:--</span>
                    </div>
                    <div class="perf-item">
                        <span class="perf-label">Reset Reason</span>
                        <span class="perf-value" id="reset-reason">--</span>
                    </div>
                    <div class="perf-item">
                        <span class="perf-label">HTTPD Stack LWM</span>
                        <span class="perf-value" id="httpd-lwm">-- B</span>
                    </div>
                </div>
            </div>
            <div class="metric-card">
                <h3>Display & Power Controls</h3>
                <div class="controls">
                    <div class="cpu-core">
                        <div class="core-label">Brightness <span id=\"dc_brightness_val\" style=\"float:right\">--</span></div>
                        <input id=\"dc_brightness\" type=\"range\" min=\"0\" max=\"255\" value=\"128\" />
                    </div>
                    <div class="cpu-core" style="display:flex; align-items:center; gap:.5rem">
                        <input type="checkbox" id="dc_display" />
                        <label for="dc_display">Display Power</label>
                    </div>
                    <div class="cpu-core" style="display:flex; align-items:center; gap:.5rem">
                        <input type="checkbox" id="dc_autodim" />
                        <label for="dc_autodim">Auto-dim when idle</label>
                    </div>
                    <div class="cpu-core">
                        <div class="core-label">Performance Mode</div>
                        <select id="dc_mode">
                            <option value="eco">Eco</option>
                            <option value="normal" selected>Normal</option>
                            <option value="turbo">Turbo</option>
                        </select>
                    </div>
                    <div class="cpu-core">
                        <div class="core-label">Dim Timeout <span id="dc_dim_val" style="float:right">--</span></div>
                        <input id="dc_dim_timeout" type="range" min="10" max="300" step="10" value="30" />
                    </div>
                    <div class="cpu-core">
                        <div class="core-label">Sleep Timeout <span id="dc_sleep_val" style="float:right">--</span></div>
                        <input id="dc_sleep_timeout" type="range" min="60" max="1800" step="60" value="300" />
                    </div>
                    <div class="cpu-core" style="display:flex; gap:.5rem; flex-wrap:wrap">
                        <button id="dc_save_power" class="theme-toggle">Save Power Settings</button>
                        <button id="dc_restart" class="theme-toggle">Restart</button>
                    </div>
                </div>
            </div>
            <div class="metric-card">
                <h3>Battery</h3>
                <div class="network-info">
                    <div class="network-item">
                        <span class="network-label">Level</span>
                        <span class="network-value" id="battery_percent">--%</span>
                    </div>
                    <div class="network-item">
                        <span class="network-label">Status</span>
                        <span class="network-value" id="battery_status">--</span>
                    </div>
                </div>
            </div>
        </div>
    </div>
"#)?;
    
    // Part 9: Basic styles
    response.write_all(br#"
    <style>
        .navbar {
            background: var(--bg-card);
            border-bottom: 1px solid var(--border);
            padding: 1rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .nav-brand {
            font-size: 1.25rem;
            font-weight: 600;
        }
        .nav-links {
            display: flex;
            gap: 1rem;
            align-items: center;
            flex: 1;
            justify-content: center;
        }
        .nav-links a {
            color: var(--text-dim);
            text-decoration: none;
            padding: 0.5rem 1rem;
            border-radius: 0.375rem;
            transition: all 0.2s;
        }
        .nav-links a:hover {
            color: var(--text);
            background: var(--bg-hover);
        }
        .nav-links a.active {
            color: var(--accent);
            background: var(--bg-hover);
        }
        .theme-toggle {
            background: transparent;
            border: 1px solid var(--border);
            color: var(--text);
            padding: 0.5rem;
            border-radius: 0.375rem;
            cursor: pointer;
        }
        .quick-stats {
            background: var(--bg-card);
            border-bottom: 1px solid var(--border);
            padding: 0.75rem 1rem;
            display: flex;
            gap: 2rem;
            overflow-x: auto;
        }
        .stat {
            display: flex;
            gap: 0.5rem;
            white-space: nowrap;
        }
        .stat-label {
            color: var(--text-dim);
        }
        .stat-value {
            font-weight: 600;
        }
        .dashboard-container {
            padding: 1rem;
            max-width: 1200px;
            margin: 0 auto;
        }
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 1rem;
        }
        .metric-card {
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: 0.5rem;
            padding: 1.5rem;
        }
        .metric-card h3 {
            margin: 0 0 1rem;
            font-size: 1.125rem;
        }
        .progress-bar {
            background: var(--bg-hover);
            height: 8px;
            border-radius: 4px;
            overflow: hidden;
            margin: 0.5rem 0;
        }
        .progress-fill {
            background: var(--accent);
            height: 100%;
            transition: width 0.3s ease;
        }
        .progress-fill.success {
            background: var(--success);
        }
        .cpu-core {
            margin-bottom: 1rem;
        }
        .core-label, .memory-header {
            display: flex;
            justify-content: space-between;
            margin-bottom: 0.25rem;
        }
        .metric-footer {
            margin-top: 1rem;
            padding-top: 1rem;
            border-top: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            font-size: 0.875rem;
            color: var(--text-dim);
        }
        .control-embed { border: 1px solid var(--border); border-radius: 8px; overflow: hidden; background: var(--bg-card); }
        .control-embed iframe { width: 100%; border: 0; min-height: 900px; background: var(--bg-card); }
        @media (max-width: 768px) {
            .nav-links {
                display: none;
            }
            .navbar {
                flex-wrap: wrap;
            }
        }
    </style>
"#)?;
    
    // Part 10: JavaScript for updates
    response.write_all(br#"
    <script>
        // Theme toggle
        const themeToggle = document.getElementById('themeToggle');
        const root = document.documentElement;
        
        // Load saved theme
        const savedTheme = localStorage.getItem('theme') || 'dark';
        root.setAttribute('data-theme', savedTheme);
        themeToggle.textContent = savedTheme === 'dark' ? 'Dark' : 'Light';
        
        themeToggle.addEventListener('click', () => {
            const currentTheme = root.getAttribute('data-theme');
            const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
            root.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);
            themeToggle.textContent = newTheme === 'dark' ? 'Dark' : 'Light';
        });
        
        // Restart button
        const restartBtn = document.getElementById('restartBtn');
        restartBtn.addEventListener('click', async () => {
            if (!confirm('Restart device now?')) return;
            try {
                const resp = await fetch('/api/restart', {
                    method: 'POST',
                    headers: { 'X-Restart-Token': 'esp32-restart' }
                });
                if (resp.ok) {
                    restartBtn.textContent = 'Restarting...';
                    setTimeout(() => { window.location.reload(); }, 4000);
                } else {
                    alert('Restart failed: ' + resp.status);
                }
            } catch (e) {
                alert('Restart error');
            }
        });
        
        // SSE connection
        let eventSource;
        
        function connectSSE() {
            console.log('Connecting to SSE...');
            eventSource = new EventSource('/api/events'); // keep metrics on /api/events
            
            eventSource.onopen = () => {
                console.log('SSE connected');
                document.getElementById('sse-status').textContent = 'Connected';
                document.getElementById('sse-status').style.color = 'var(--success)';
            };
            
            eventSource.onerror = (e) => {
                console.error('SSE error:', e);
                document.getElementById('sse-status').textContent = 'Disconnected';
                document.getElementById('sse-status').style.color = 'var(--danger)';
                // Reconnect after 5 seconds
                setTimeout(connectSSE, 5000);
            };
            
            eventSource.onmessage = (event) => {
                console.log('SSE data received');
                try {
                    const data = JSON.parse(event.data);
                    updateDashboard(data);
                } catch (e) {
                    console.error('Failed to parse SSE data:', e);
                }
            };
        }
        
        function updateDashboard(data) {
            // Update uptime
            if (data.uptime_ms) {
                document.getElementById('uptime').textContent = formatUptime(data.uptime_ms);
            }
            
            // Update CPU
            if (data.cpu0_usage !== undefined) {
                const cpu0 = Math.round(data.cpu0_usage);
                document.getElementById('cpu0-usage').textContent = cpu0 + '%';
                document.getElementById('cpu0-bar').style.width = cpu0 + '%';
            }
            if (data.cpu1_usage !== undefined) {
                const cpu1 = Math.round(data.cpu1_usage);
                document.getElementById('cpu1-usage').textContent = cpu1 + '%';
                document.getElementById('cpu1-bar').style.width = cpu1 + '%';
            }
            if (data.cpu_freq_mhz) {
                document.getElementById('cpu-freq').textContent = data.cpu_freq_mhz + ' MHz';
            }
            if (data.temperature !== undefined) {
                document.getElementById('cpu-temp').textContent = data.temperature.toFixed(1) + String.fromCharCode(176) + 'C';
            }
            
            // Update Memory
            if (data.heap_free_kb !== undefined) {
                document.getElementById('heap-free').textContent = data.heap_free_kb + ' KB';
                const heapPct = Math.round((data.heap_free_kb / 320) * 100); // Assume 320KB total
                document.getElementById('heap-bar').style.width = heapPct + '%';
            }
            if (data.psram_free_kb !== undefined) {
                document.getElementById('psram-free').textContent = data.psram_free_kb + ' KB';
                const psramPct = Math.round((data.psram_free_kb / 8192) * 100); // 8MB PSRAM
                document.getElementById('psram-bar').style.width = psramPct + '%';
            }
            if (data.heap_fragmentation !== undefined) {
                document.getElementById('heap-frag').textContent = data.heap_fragmentation + '%';
            }
            
            // Update Performance
            if (data.fps_actual !== undefined) {
                document.getElementById('fps').textContent = data.fps_actual.toFixed(1) + ' fps';
            }
            if (data.skip_rate !== undefined) {
                document.getElementById('skip-rate').textContent = data.skip_rate.toFixed(0) + '%';
            }
            if (data.render_time_ms !== undefined) {
                const el = document.getElementById('render-time');
                if (el) el.textContent = data.render_time_ms + ' ms';
            }
            
            // Update Network
            if (data.wifi_connected !== undefined) {
                document.getElementById('wifi-status').textContent = data.wifi_connected ? 'Connected' : 'Disconnected';
                document.getElementById('wifi-status').style.color = data.wifi_connected ? 'var(--success)' : 'var(--danger)';
            }
            if (data.wifi_ssid) {
                document.getElementById('ssid').textContent = data.wifi_ssid;
            }
            if (data.wifi_rssi !== undefined) {
                document.getElementById('rssi').textContent = data.wifi_rssi + ' dBm';
            }
            if (data.ip_address) {
                document.getElementById('ip-address').textContent = data.ip_address;
            }
            if (data.version) {
                const v = document.getElementById('fw-version');
                if (v) v.textContent = data.version;
            }

            // Update Health
            if (data.uptime_ms) {
                document.getElementById('uptime').textContent = formatUptime(data.uptime_ms);
            }
            if (data.reset_reason) {
                document.getElementById('reset-reason').textContent = data.reset_reason;
            }
            if (data.httpd_stack_low_water) {
                document.getElementById('httpd-lwm').textContent = data.httpd_stack_low_water + ' B';
            }
        }
        
        // Controls wiring
        (function(){
            const dcB = document.getElementById('dc_brightness');
            const dcBVal = document.getElementById('dc_brightness_val');
            const dcAD = document.getElementById('dc_autodim');
            const dcMode = document.getElementById('dc_mode');
            const dcRestart = document.getElementById('dc_restart');
            const dcDisplay = document.getElementById('dc_display');
            const dimSlider = document.getElementById('dc_dim_timeout');
            const dimVal = document.getElementById('dc_dim_val');
            const sleepSlider = document.getElementById('dc_sleep_timeout');
            const sleepVal = document.getElementById('dc_sleep_val');
            const savePower = document.getElementById('dc_save_power');

            if (dcB) dcB.addEventListener('input', ()=>{
                dcBVal.textContent = dcB.value;
                fetch('/api/control', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({brightness: Number(dcB.value)})});
            });
            if (dcAD) dcAD.addEventListener('change', ()=>{
                fetch('/api/config', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({auto_dim: !!dcAD.checked})});
            });
            if (dcMode) dcMode.addEventListener('change', ()=>{
                fetch('/api/control', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({mode: dcMode.value})});
            });
            if (dcRestart) dcRestart.addEventListener('click', async ()=>{
                if (!confirm('Restart device now?')) return;
                try{
                    const r = await fetch('/api/restart', {method:'POST', headers:{'X-Restart-Token':'esp32-restart'}});
                    if (r.ok){ dcRestart.textContent='Restarting...'; setTimeout(()=>location.reload(), 4000);} else { alert('Restart failed: '+r.status); }
                }catch(e){ alert('Restart error'); }
            });
            if (dcDisplay) dcDisplay.addEventListener('change', ()=>{
                fetch('/api/control', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({display: !!dcDisplay.checked})});
            });
            if (dimSlider) dimSlider.addEventListener('input', ()=>{ dimVal.textContent = dimSlider.value + 's'; });
            if (sleepSlider) sleepSlider.addEventListener('input', ()=>{ sleepVal.textContent = Math.floor(sleepSlider.value/60) + 'm'; });
            if (savePower) savePower.addEventListener('click', async ()=>{
                const body = {
                    dim_timeout: Number(dimSlider.value),
                    sleep_timeout: Number(sleepSlider.value),
                    auto_dim: !!dcAD.checked
                };
                try { await fetch('/api/config', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify(body)}); } catch(e){}
            });

            // Initialize controls from /api/config
            (async function(){
                try{
                    const r = await fetch('/api/config');
                    const j = await r.json();
                    if (j){
                        if (typeof j.brightness==='number' && dcB){ dcB.value = Math.max(0, Math.min(255, j.brightness)); dcBVal.textContent = dcB.value; }
                        if (typeof j.auto_dim==='boolean' && dcAD){ dcAD.checked = j.auto_dim; }
                        else if (typeof j.auto_brightness==='boolean' && dcAD){ dcAD.checked = j.auto_brightness; }
                        const dim = (j.dim_timeout ?? j.dim_timeout_secs ?? 30);
                        const sleep = (j.sleep_timeout ?? j.sleep_timeout_secs ?? 300);
                        if (dimSlider){ dimSlider.value = dim; dimVal.textContent = dim + 's'; }
                        if (sleepSlider){ sleepSlider.value = sleep; sleepVal.textContent = Math.floor(sleep/60) + 'm'; }
                    }
                }catch(e){}
            })();
        })();
        
        function formatUptime(ms) {
            const seconds = Math.floor(ms / 1000);
            const hours = Math.floor(seconds / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);
            const secs = seconds % 60;
            return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        }
        
        // Initial update via REST API
        async function initialUpdate() {
            console.log('Fetching initial data...');
            try {
                const [system, metrics] = await Promise.all([
                    fetch('/api/system').then(r => r.json()),
                    fetch('/api/metrics').then(r => r.json())
                ]);
                
                console.log('Initial data received:', {system, metrics});
                
                // Merge data and update
                updateDashboard({...system, ...metrics});
                
                // Update IP from system data
                if (system.wifi && system.wifi.ip) {
                    document.getElementById('ip-address').textContent = system.wifi.ip;
                }
            } catch (e) {
                console.error('Initial update failed:', e);
            }
        }
        
        // Start everything
        window.addEventListener('load', () => {
            console.log('Dashboard loaded, starting updates...');
            initialUpdate();
            connectSSE();
        });
        
        // Cleanup on unload
        window.addEventListener('beforeunload', () => {
            if (eventSource) {
                eventSource.close();
            }
        });
        // Resize embedded control to content height when loaded
        const controlFrame = document.getElementById('controlFrame');
        if (controlFrame) {
            controlFrame.addEventListener('load', () => {
                try {
                    const d = controlFrame.contentWindow.document;
                    const h = Math.max(d.body.scrollHeight, d.documentElement.scrollHeight);
                    if (h && h > 0) controlFrame.style.minHeight = (h + 40) + 'px';
                } catch (e) { /* ignore */ }
            });
        }
    </script>
</body>
</html>"#)?;
    
    Ok(())
}