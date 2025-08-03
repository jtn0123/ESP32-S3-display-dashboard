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