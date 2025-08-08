use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::sensors::history::SensorHistory;
use crate::network::validators;
use crate::network::error_handler::ErrorResponse;

pub fn register_api_v1_routes(
    server: &mut EspHttpServer<'static>,
    config: Arc<Mutex<Config>>,
    sensor_history: Arc<Mutex<SensorHistory>>,
) -> Result<()> {
    
    // GET /api/v1/sensors/temperature/history?hours=24
    let history_clone = sensor_history.clone();
    server.fn_handler("/api/v1/sensors/temperature/history", Method::Get, move |req| {
        let hours = req.uri()
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("hours=")))
            .and_then(|p| p.strip_prefix("hours="))
            .and_then(|h| h.parse::<u32>().ok())
            .unwrap_or(24);

        let history = match history_clone.lock() {
            Ok(h) => h,
            Err(e) => {
                return ErrorResponse::bad_request(format!("history lock failed: {}", e)).send(req);
            }
        };
        let data = history.get_temperature_history(hours);
        
        let response = serde_json::json!({
            "hours": hours,
            "data": data,
            "unit": "celsius"
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

    // GET /api/v1/sensors/battery/history?hours=24
    let history_clone2 = sensor_history.clone();
    server.fn_handler("/api/v1/sensors/battery/history", Method::Get, move |req| {
        let hours = req.uri()
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("hours=")))
            .and_then(|p| p.strip_prefix("hours="))
            .and_then(|h| h.parse::<u32>().ok())
            .unwrap_or(24);

        let history = match history_clone2.lock() {
            Ok(h) => h,
            Err(e) => {
                return ErrorResponse::bad_request(format!("history lock failed: {}", e)).send(req);
            }
        };
        let data = history.get_battery_history(hours);
        
        let response = serde_json::json!({
            "hours": hours,
            "data": data,
            "unit": "percentage"
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

    // GET /api/v1/system/processes
    server.fn_handler("/api/v1/system/processes", Method::Get, move |req| {
        let mut processes = Vec::new();
        
        // Get current task info using available ESP-IDF APIs
        let current_task = unsafe {
            let handle = esp_idf_sys::xTaskGetCurrentTaskHandle();
            let name = esp_idf_sys::pcTaskGetName(handle);
            let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();
            
            serde_json::json!({
                "name": name_str,
                "core": esp_idf_sys::xTaskGetCoreID(handle),
                "priority": esp_idf_sys::uxTaskPriorityGet(handle),
                "stack_watermark": esp_idf_sys::uxTaskGetStackHighWaterMark(handle)
            })
        };
        
        processes.push(current_task);
        
        // TODO: Add system-wide task enumeration when available

        let response = serde_json::json!({
            "total": processes.len(),
            "processes": processes,
            "note": "Currently showing only current task info"
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

    // POST /api/v1/display/screenshot
    server.fn_handler("/api/v1/display/screenshot", Method::Post, move |req| {
        // For now, return a placeholder response
        // TODO: Implement actual screenshot capture once display module supports it
        
        let response = serde_json::json!({
            "format": "rgb565",
            "width": 320,
            "height": 170,
            "data": "", // Empty for now
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "message": "Screenshot capture not yet implemented"
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

    // PATCH /api/v1/config/:field
    let config_clone = config.clone();
    server.fn_handler("/api/v1/config/*", Method::Patch, move |mut req| {
        // Extract field name from URL (before any mutable borrows)
        let uri = req.uri().to_string();
        let field = uri
            .strip_prefix("/api/v1/config/")
            .unwrap_or("")
            .to_string();

        if field.is_empty() {
            return ErrorResponse::bad_request("Missing field name").send(req);
        }

        // Read patch data
        let mut buf = vec![0; 512];
        let len = req.read(&mut buf)?;
        buf.truncate(len);
        
        let json_str = std::str::from_utf8(&buf)?;
        let value: serde_json::Value = serde_json::from_str(json_str)?;

        // Apply patch based on field
        let mut cfg = match config_clone.lock() {
            Ok(c) => c,
            Err(e) => {
                return ErrorResponse::bad_request(format!("config lock failed: {}", e)).send(req);
            }
        };
        match field.as_str() {
            "wifi_ssid" => {
                if let Some(ssid) = value.as_str() {
                    validators::validate_ssid(ssid)?;
                    cfg.wifi_ssid = ssid.to_string();
                } else {
                    return ErrorResponse::bad_request("wifi_ssid must be a string").send(req);
                }
            }
            "brightness" => {
                if let Some(brightness) = value.as_u64() {
                    validators::validate_brightness(brightness as u8)?;
                    cfg.brightness = brightness as u8;
                } else {
                    return ErrorResponse::bad_request("brightness must be 0-255").send(req);
                }
            }
            "auto_brightness" => {
                if let Some(auto) = value.as_bool() {
                    cfg.auto_brightness = auto;
                } else {
                    return ErrorResponse::bad_request("auto_brightness must be boolean").send(req);
                }
            }
            _ => {
                return ErrorResponse::bad_request(format!("Unknown field: {}", field)).send(req);
            }
        }

        // Save config
        cfg.save()?;
        drop(cfg);

        let response = serde_json::json!({
            "field": field,
            "value": value,
            "status": "updated"
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

    // POST /api/v1/debug/log-level {"level":"trace|debug|info|warn|error|off"} (also supports ?level=)
    server.fn_handler("/api/v1/debug/log-level", Method::Post, move |mut req| {
        // Read body
        let mut buf = [0u8; 64];
        let len = req.read(&mut buf)?;
        let body = core::str::from_utf8(&buf[..len]).unwrap_or("");
        let level = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("level").and_then(|s| s.as_str()).map(|s| s.to_owned()))
            .or_else(|| {
                // Fallback to query string: ?level=debug
                req.uri().split('?').nth(1)
                    .and_then(|q| q.split('&').find(|p| p.starts_with("level=")))
                    .and_then(|p| p.strip_prefix("level=")).map(|s| s.to_string())
            });

        let Some(level_str) = level else {
            return ErrorResponse::bad_request("missing 'level'").send(req);
        };

        if crate::logging::set_max_level_from_str(&level_str) {
            let resp = serde_json::json!({
                "ok": true,
                "level": level_str,
            });
            let json = serde_json::to_string(&resp)?;
            let mut http_response = req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?;
            http_response.write_all(json.as_bytes())?;
            Ok(()) as Result<(), Box<dyn std::error::Error>>
        } else {
            ErrorResponse::bad_request("invalid level; use off|error|warn|info|debug|trace").send(req)
        }
    })?;

    // GET /api/v1/logs/recent?count=50
    server.fn_handler("/api/v1/logs/recent", Method::Get, move |req| {
        let count = req.uri()
            .split('?')
            .nth(1)
            .and_then(|query| query.split('&').find(|p| p.starts_with("count=")))
            .and_then(|p| p.strip_prefix("count="))
            .and_then(|h| h.parse::<usize>().ok())
            .map(|n| n.min(500))
            .unwrap_or(50);

        let streamer = crate::network::log_streamer::init(None);
        let logs = streamer.get_recent_logs(count);
        let json = serde_json::to_string(&logs)?;
        let mut http_response = req.into_response(200, Some("OK"), &[("Content-Type", "application/json")])?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    // GET /api/v1/diagnostics/health
    server.fn_handler("/api/v1/diagnostics/health", Method::Get, move |req| {
        let heap_free = unsafe { esp_idf_sys::esp_get_free_heap_size() };
        let heap_min = unsafe { esp_idf_sys::esp_get_minimum_free_heap_size() };
        // Get temperature from system (placeholder for now)
        let temp = 45.0; // TODO: Get from sensor manager
        
        let health = serde_json::json!({
            "status": if heap_free > 50000 && temp < 80.0 { "healthy" } else { "degraded" },
            "checks": {
                "memory": {
                    "status": if heap_free > 50000 { "ok" } else { "low" },
                    "free": heap_free,
                    "minimum": heap_min
                },
                "temperature": {
                    "status": if temp < 70.0 { "ok" } else if temp < 80.0 { "warm" } else { "hot" },
                    "value": temp
                },
                "network": {
                    "status": "unknown" // TODO: Get from network manager
                }
            },
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let json = serde_json::to_string(&health)?;
        let mut http_response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        http_response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;

    log::info!("API v1 routes registered");
    Ok(())
}