use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use std::fs;
use std::path::PathBuf;
use crate::network::error_handler::ErrorResponse;
use crate::network::validators;

const MAX_FILE_SIZE: usize = 256 * 1024; // 256KB for ESP32
const ALLOWED_EXTENSIONS: &[&str] = &["json", "toml", "log", "bin", "txt", "md"];
const BASE_PATH: &str = "/spiffs"; // or "/littlefs" based on your partition

pub fn register_file_routes(server: &mut EspHttpServer<'static>) -> Result<()> {
    // GET /api/files - List files
    server.fn_handler("/api/files", Method::Get, |req| {
        let path = req.uri()
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("path=")))
            .and_then(|p| p.strip_prefix("path="))
            .unwrap_or("/");

        let base_path = PathBuf::from(BASE_PATH);
        let full_path = base_path.join(path.trim_start_matches('/'));

        // Security check - prevent directory traversal
        if !full_path.starts_with(&base_path) {
            return ErrorResponse::bad_request("Invalid path").send(req);
        }

        let mut files = Vec::new();
        
        if full_path.exists() && full_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&full_path) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        files.push(serde_json::json!({
                            "name": name,
                            "type": if metadata.is_dir() { "directory" } else { "file" },
                            "size": metadata.len(),
                            "modified": metadata.modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        }));
                    }
                }
            }
        }

        let response = serde_json::json!({
            "path": path,
            "files": files,
        });

        let json = serde_json::to_string(&response)?;
        let mut http_response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    // GET /api/files/content - Read file content
    server.fn_handler("/api/files/content", Method::Get, |req| {
        let filename = req.uri()
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("file=")))
            .and_then(|p| p.strip_prefix("file="))
            .ok_or_else(|| anyhow::anyhow!("Missing file parameter"))?;

        validators::validate_filename(&filename)?;

        let base_path = PathBuf::from("/data");
        let file_path = base_path.join(&filename);

        // Security check
        if !file_path.starts_with(&base_path) {
            return ErrorResponse::bad_request("Invalid file path").send(req);
        }

        // Check file extension
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if !ALLOWED_EXTENSIONS.contains(&extension) {
            return ErrorResponse::bad_request("File type not allowed").send(req);
        }

        // Read file
        if !file_path.exists() {
            return ErrorResponse::not_found("File not found").send(req);
        }

        let content = fs::read_to_string(&file_path)?;
        let metadata = fs::metadata(&file_path)?;

        let response = serde_json::json!({
            "filename": filename,
            "content": content,
            "size": metadata.len(),
            "readonly": !metadata.permissions().readonly(),
            "type": extension,
        });

        let json = serde_json::to_string(&response)?;
        let mut http_response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    // PUT /api/files/content - Save file content
    server.fn_handler("/api/files/content", Method::Put, |mut req| {
        let uri = req.uri().to_string();
        let filename = uri
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("file=")))
            .and_then(|p| p.strip_prefix("file="))
            .ok_or_else(|| anyhow::anyhow!("Missing file parameter"))?
            .to_string();

        validators::validate_filename(&filename)?;

        // Read request body
        let mut buf = vec![0; MAX_FILE_SIZE];
        let len = req.read(&mut buf)?;
        buf.truncate(len);

        let json_str = std::str::from_utf8(&buf)?;
        let data: serde_json::Value = serde_json::from_str(json_str)?;
        
        let content = data.get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing content field"))?;

        let base_path = PathBuf::from("/data");
        let file_path = base_path.join(&filename);

        // Security check
        if !file_path.starts_with(&base_path) {
            return ErrorResponse::bad_request("Invalid file path").send(req);
        }

        // Check file extension
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if !ALLOWED_EXTENSIONS.contains(&extension) || extension == "bin" {
            return ErrorResponse::bad_request("Cannot edit this file type").send(req);
        }

        // Create backup
        if file_path.exists() {
            let backup_path = file_path.with_extension(format!("{}.bak", extension));
            fs::copy(&file_path, &backup_path)?;
        }

        // Write file
        fs::write(&file_path, content)?;

        let response = serde_json::json!({
            "status": "saved",
            "filename": filename,
            "size": content.len(),
        });

        let json = serde_json::to_string(&response)?;
        let mut http_response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    // POST /api/files/upload - Upload file
    server.fn_handler("/api/files/upload", Method::Post, |mut req| {
        let filename = req.header("X-Filename")
            .ok_or_else(|| anyhow::anyhow!("Missing X-Filename header"))?
            .to_string();
        
        let content_length = req.header("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or_else(|| anyhow::anyhow!("Missing Content-Length"))?;

        validators::validate_filename(&filename)?;

        if content_length > MAX_FILE_SIZE {
            return ErrorResponse::bad_request("File too large").send(req);
        }

        let base_path = PathBuf::from("/data/uploads");
        fs::create_dir_all(&base_path)?;
        
        let file_path = base_path.join(&filename);

        // Security check
        if !file_path.starts_with(&base_path) {
            return ErrorResponse::bad_request("Invalid file path").send(req);
        }

        // Read and save file
        let mut buffer = vec![0u8; 4096];
        let mut file = fs::File::create(&file_path)?;
        let mut total_written = 0;

        loop {
            match req.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    use std::io::Write as StdWrite;
                    file.write_all(&buffer[..n])?;
                    total_written += n;
                }
                Err(e) => return Err(e.into()),
            }
        }

        let response = serde_json::json!({
            "status": "uploaded",
            "filename": filename,
            "size": total_written,
            "path": format!("/data/uploads/{}", filename),
        });

        let json = serde_json::to_string(&response)?;
        let mut http_response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    // DELETE /api/files - Delete file
    server.fn_handler("/api/files", Method::Delete, |req| {
        let filename = req.uri()
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("file=")))
            .and_then(|p| p.strip_prefix("file="))
            .ok_or_else(|| anyhow::anyhow!("Missing file parameter"))?
            .to_string();

        validators::validate_filename(&filename)?;

        let base_path = PathBuf::from("/data");
        let file_path = base_path.join(&filename);

        // Security check
        if !file_path.starts_with(&base_path) {
            return ErrorResponse::bad_request("Invalid file path").send(req);
        }

        if !file_path.exists() {
            return ErrorResponse::not_found("File not found").send(req);
        }

        // Don't allow deleting critical files
        let critical_files = ["config.json", "wifi_config.json"];
        if critical_files.contains(&filename.as_str()) {
            return ErrorResponse::bad_request("Cannot delete system files").send(req);
        }

        fs::remove_file(&file_path)?;

        let response = serde_json::json!({
            "status": "deleted",
            "filename": filename,
        });

        let json = serde_json::to_string(&response)?;
        let mut http_response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    // File manager UI page
    server.fn_handler("/files", Method::Get, |req| {
        let html = include_str!("../templates/files.html");
        let mut response = req.into_ok_response()?;
        response.write_all(html.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    log::info!("File manager routes registered");
    Ok(())
}