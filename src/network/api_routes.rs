use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::sensors::history::SensorHistory;
use crate::network::validators;

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

        let history = history_clone.lock().unwrap();
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

        let history = history_clone2.lock().unwrap();
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
            let mut response = req.into_status_response(400)?;
            response.write_all(b"{\"error\":\"Missing field name\"}")?;
            return Ok(());
        }

        // Read patch data
        let mut buf = vec![0; 512];
        let len = req.read(&mut buf)?;
        buf.truncate(len);
        
        let json_str = std::str::from_utf8(&buf)?;
        let value: serde_json::Value = serde_json::from_str(json_str)?;

        // Apply patch based on field
        let mut cfg = config_clone.lock().unwrap();
        match field.as_str() {
            "wifi_ssid" => {
                if let Some(ssid) = value.as_str() {
                    validators::validate_ssid(ssid)?;
                    cfg.wifi_ssid = ssid.to_string();
                } else {
                    let mut response = req.into_status_response(400)?;
                    response.write_all(b"{\"error\":\"wifi_ssid must be a string\"}")?;
                    return Ok(());
                }
            }
            "brightness" => {
                if let Some(brightness) = value.as_u64() {
                    validators::validate_brightness(brightness as u8)?;
                    cfg.brightness = brightness as u8;
                } else {
                    let mut response = req.into_status_response(400)?;
                    response.write_all(b"{\"error\":\"brightness must be 0-255\"}")?;
                    return Ok(());
                }
            }
            "auto_brightness" => {
                if let Some(auto) = value.as_bool() {
                    cfg.auto_brightness = auto;
                } else {
                    let mut response = req.into_status_response(400)?;
                    response.write_all(b"{\"error\":\"auto_brightness must be boolean\"}")?;
                    return Ok(());
                }
            }
            _ => {
                let error_msg = format!("{{\"error\":\"Unknown field: {}\"}}", field);
                let mut response = req.into_status_response(400)?;
                response.write_all(error_msg.as_bytes())?;
                return Ok(());
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