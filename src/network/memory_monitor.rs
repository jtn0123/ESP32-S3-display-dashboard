use anyhow::Result;
use esp_idf_svc::http::server::{EspHttpServer, Method};
use esp_idf_svc::io::Write;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MemoryStats {
    // Heap memory
    free_heap: u32,
    largest_free_block: u32,
    minimum_free_heap: u32,
    
    // PSRAM (if available)
    free_psram: u32,
    largest_psram_block: u32,
    
    // Stack info
    current_task_stack_watermark: u32,
    
    // Internal memory regions
    internal_free: u32,
    internal_largest_block: u32,
    
    // DMA capable memory
    dma_free: u32,
    dma_largest_block: u32,
    
    // Memory allocations
    malloc_count: u32,
    free_count: u32,
}

impl MemoryStats {
    fn collect() -> Self {
        unsafe {
            // Basic heap stats
            let free_heap = esp_idf_sys::esp_get_free_heap_size();
            let largest_free_block = esp_idf_sys::heap_caps_get_largest_free_block(
                esp_idf_sys::MALLOC_CAP_DEFAULT
            );
            let minimum_free_heap = esp_idf_sys::esp_get_minimum_free_heap_size();
            
            // PSRAM stats
            let free_psram = esp_idf_sys::heap_caps_get_free_size(
                esp_idf_sys::MALLOC_CAP_SPIRAM
            );
            let largest_psram_block = esp_idf_sys::heap_caps_get_largest_free_block(
                esp_idf_sys::MALLOC_CAP_SPIRAM
            );
            
            // Current task stack watermark
            let current_task_stack_watermark = esp_idf_sys::uxTaskGetStackHighWaterMark(
                std::ptr::null_mut()
            );
            
            // Internal memory stats
            let internal_free = esp_idf_sys::heap_caps_get_free_size(
                esp_idf_sys::MALLOC_CAP_INTERNAL
            );
            let internal_largest_block = esp_idf_sys::heap_caps_get_largest_free_block(
                esp_idf_sys::MALLOC_CAP_INTERNAL
            );
            
            // DMA capable memory
            let dma_free = esp_idf_sys::heap_caps_get_free_size(
                esp_idf_sys::MALLOC_CAP_DMA
            );
            let dma_largest_block = esp_idf_sys::heap_caps_get_largest_free_block(
                esp_idf_sys::MALLOC_CAP_DMA
            );
            
            // Get allocation info
            let mut info = esp_idf_sys::multi_heap_info_t::default();
            esp_idf_sys::heap_caps_get_info(
                &mut info,
                esp_idf_sys::MALLOC_CAP_DEFAULT
            );
            
            Self {
                free_heap,
                largest_free_block: largest_free_block as u32,
                minimum_free_heap: minimum_free_heap as u32,
                free_psram: free_psram as u32,
                largest_psram_block: largest_psram_block as u32,
                current_task_stack_watermark: current_task_stack_watermark as u32,
                internal_free: internal_free as u32,
                internal_largest_block: internal_largest_block as u32,
                dma_free: dma_free as u32,
                dma_largest_block: dma_largest_block as u32,
                malloc_count: info.total_allocated_bytes / 32, // Rough estimate
                free_count: info.total_free_bytes / 32, // Rough estimate
            }
        }
    }
}

pub fn register_memory_monitoring_endpoints(server: &mut EspHttpServer<'static>) -> Result<()> {
    // Detailed memory stats endpoint
    server.fn_handler("/api/memory/detailed", Method::Get, |req| {
        let stats = MemoryStats::collect();
        let json = serde_json::to_string(&stats)?;
        
        let mut response = req.into_response(
            200,
            Some("OK"),
            &[("Content-Type", "application/json")]
        )?;
        response.write_all(json.as_bytes())?;
        Ok(()) as Result<(), Box<dyn std::error::Error>>
    })?;
    
    // Memory pressure test endpoint
    server.fn_handler("/api/memory/pressure-test", Method::Post, |mut req| {
        // Read size from request
        let mut buf = vec![0; 256];
        let len = req.read(&mut buf)?;
        buf.truncate(len);
        
        let json_str = std::str::from_utf8(&buf)?;
        let params: serde_json::Value = serde_json::from_str(json_str)?;
        let size = params.get("size")
            .and_then(|s| s.as_u64())
            .unwrap_or(1024) as usize;
        
        // Record memory before
        let before = MemoryStats::collect();
        
        // Allocate memory
        let _test_allocation = vec![0u8; size];
        
        // Record memory after
        let after = MemoryStats::collect();
        
        let response = serde_json::json!({
            "requested_size": size,
            "heap_before": before.free_heap,
            "heap_after": after.free_heap,
            "actual_allocated": before.free_heap - after.free_heap,
            "largest_block_after": after.largest_free_block,
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
    
    // Heap dump endpoint (simplified)
    server.fn_handler("/api/memory/heap-info", Method::Get, |req| {
        let info = unsafe {
            let mut i = esp_idf_sys::multi_heap_info_t::default();
            esp_idf_sys::heap_caps_get_info(&mut i, esp_idf_sys::MALLOC_CAP_DEFAULT);
            i
        };
        
        let response = serde_json::json!({
            "total_free_bytes": info.total_free_bytes,
            "total_allocated_bytes": info.total_allocated_bytes,
            "largest_free_block": info.largest_free_block,
            "minimum_free_bytes": info.minimum_free_bytes,
            "allocated_blocks": info.allocated_blocks,
            "free_blocks": info.free_blocks,
            "total_blocks": info.total_blocks,
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
    
    log::info!("Memory monitoring endpoints registered");
    Ok(())
}