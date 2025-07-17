// Web server for OTA updates - provides HTTP interface for firmware upload

use esp_idf_svc::{
    http::server::{Configuration, EspHttpServer},
    io::Write,
};


pub const OTA_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 OTA Update</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 600px;
            margin: 50px auto;
            background: #1a1a1a;
            color: #ffffff;
        }
        .container {
            background: #2a2a2a;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.3);
        }
        h1 {
            color: #4CAF50;
            text-align: center;
        }
        .upload-area {
            border: 2px dashed #4CAF50;
            padding: 30px;
            text-align: center;
            margin: 20px 0;
            border-radius: 5px;
            background: #1a1a1a;
        }
        input[type="file"] {
            margin: 20px 0;
        }
        button {
            background: #4CAF50;
            color: white;
            padding: 10px 30px;
            border: none;
            border-radius: 5px;
            font-size: 16px;
            cursor: pointer;
        }
        button:hover {
            background: #45a049;
        }
        button:disabled {
            background: #666;
            cursor: not-allowed;
        }
        .progress {
            width: 100%;
            height: 30px;
            background: #333;
            border-radius: 5px;
            overflow: hidden;
            margin: 20px 0;
            display: none;
        }
        .progress-bar {
            height: 100%;
            background: #4CAF50;
            width: 0%;
            transition: width 0.3s;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: bold;
        }
        .status {
            text-align: center;
            margin: 20px 0;
            font-weight: bold;
        }
        .info {
            background: #333;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>ESP32-S3 Dashboard OTA Update</h1>
        
        <div class="info">
            <p><strong>Current Version:</strong> v4.13-rust</p>
            <p><strong>Device:</strong> ESP32-S3 T-Display</p>
        </div>
        
        <div class="upload-area">
            <p>Select firmware file (.bin)</p>
            <input type="file" id="fileInput" accept=".bin" />
            <br>
            <button id="uploadBtn" onclick="uploadFirmware()">Upload Firmware</button>
        </div>
        
        <div class="progress" id="progressDiv">
            <div class="progress-bar" id="progressBar">0%</div>
        </div>
        
        <div class="status" id="status"></div>
    </div>
    
    <script>
        async function uploadFirmware() {
            const fileInput = document.getElementById('fileInput');
            const file = fileInput.files[0];
            
            if (!file) {
                alert('Please select a firmware file');
                return;
            }
            
            if (!file.name.endsWith('.bin')) {
                alert('Please select a .bin file');
                return;
            }
            
            const uploadBtn = document.getElementById('uploadBtn');
            const progressDiv = document.getElementById('progressDiv');
            const progressBar = document.getElementById('progressBar');
            const status = document.getElementById('status');
            
            uploadBtn.disabled = true;
            progressDiv.style.display = 'block';
            status.textContent = 'Uploading...';
            
            try {
                const response = await fetch('/ota/update', {
                    method: 'POST',
                    body: file,
                    headers: {
                        'Content-Length': file.size
                    }
                });
                
                if (response.ok) {
                    progressBar.style.width = '100%';
                    progressBar.textContent = '100%';
                    status.textContent = 'Update successful! Device will restart...';
                    status.style.color = '#4CAF50';
                } else {
                    throw new Error('Upload failed');
                }
            } catch (error) {
                status.textContent = 'Update failed: ' + error.message;
                status.style.color = '#f44336';
                uploadBtn.disabled = false;
            }
        }
        
        // Simulate progress (real progress would come from server)
        function updateProgress(percent) {
            const progressBar = document.getElementById('progressBar');
            progressBar.style.width = percent + '%';
            progressBar.textContent = percent + '%';
        }
    </script>
</body>
</html>
"#;

#[allow(dead_code)]
pub struct OtaWebServer<'a> {
    pub server: EspHttpServer<'a>,
}

impl<'a> OtaWebServer<'a> {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut server = EspHttpServer::new(&Configuration::default())?;
        
        // Serve the OTA HTML page
        server.fn_handler("/", esp_idf_svc::http::Method::Get, |req| {
            let mut response = req.into_ok_response()?;
            response.write_all(OTA_HTML.as_bytes())?;
            Ok::<(), anyhow::Error>(())
        })?;
        
        Ok(Self { server })
    }
    
}

