/// Streaming response utilities to prevent memory fragmentation
use esp_idf_svc::http::server::{EspHttpConnection, Request, Response};
use esp_idf_svc::io::Write;
use core::fmt::Write as FmtWrite;
use heapless::String;
use heapless::Vec;

/// Buffer size for streaming chunks (small to fit in internal DRAM)
const CHUNK_SIZE: usize = 512;

/// Stream helper that writes data in small chunks
pub struct StreamingResponse<'a> {
    response: Response<&'a mut EspHttpConnection<'a>>,
    buffer: Vec<u8, CHUNK_SIZE>,
}

impl<'a> StreamingResponse<'a> {
    /// Create a new streaming response
    pub fn new(req: Request<&'a mut EspHttpConnection<'a>>) -> Result<Self, Box<dyn std::error::Error>> {
        let response = req.into_response(
            200,
            Some("OK"),
            &[
                ("Content-Type", "text/html; charset=utf-8"),
                ("Transfer-Encoding", "chunked"),
                ("Connection", "close"),
            ]
        )?;
        
        Ok(Self {
            response,
            buffer: Vec::new(),
        })
    }
    
    /// Write a string slice, automatically flushing when buffer is full
    pub fn write_str(&mut self, s: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_bytes(s.as_bytes())
    }
    
    /// Write bytes, automatically flushing when buffer is full
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut offset = 0;
        
        while offset < data.len() {
            let remaining_buffer = CHUNK_SIZE - self.buffer.len();
            let chunk_end = (offset + remaining_buffer).min(data.len());
            
            // Add to buffer
            self.buffer.extend_from_slice(&data[offset..chunk_end]).ok();
            
            // Flush if buffer is full
            if self.buffer.len() >= CHUNK_SIZE {
                self.flush()?;
            }
            
            offset = chunk_end;
        }
        
        Ok(())
    }
    
    /// Write formatted data using stack-based formatting
    pub fn write_fmt<T: core::fmt::Display>(&mut self, args: core::fmt::Arguments<'_>) -> Result<(), Box<dyn std::error::Error>> {
        // Use a small stack buffer for formatting
        let mut s: String<256> = String::new();
        
        // Format into the heapless string
        write!(&mut s, "{}", args).ok();
        
        // Write the formatted string
        self.write_str(&s)
    }
    
    /// Flush any buffered data
    pub fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.buffer.is_empty() {
            self.response.write_all(&self.buffer)?;
            self.buffer.clear();
        }
        Ok(())
    }
    
    /// Finish the response, flushing any remaining data
    pub fn finish(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.flush()?;
        // Response will be closed when dropped
        Ok(())
    }
}

/// Helper macro for writing formatted strings to a streaming response
#[macro_export]
macro_rules! stream_write {
    ($stream:expr, $($arg:tt)*) => {
        $stream.write_fmt(format_args!($($arg)*))
    };
}

/// Stream a static HTML template with dynamic values
pub fn stream_template_header(stream: &mut StreamingResponse) -> Result<(), Box<dyn std::error::Error>> {
    stream.write_str(r#"<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 Dashboard</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>"#)?;
    
    // Stream CSS in chunks
    stream.write_str(include_str!("../templates/styles.css"))?;
    
    stream.write_str(r#"</style>
</head>
<body>
    <div class="container">"#)?;
    
    Ok(())
}

/// Stream template footer
pub fn stream_template_footer(stream: &mut StreamingResponse) -> Result<(), Box<dyn std::error::Error>> {
    stream.write_str(r#"
    </div>
</body>
</html>"#)?;
    Ok(())
}

/// Stream a card with title and content
pub fn stream_card(stream: &mut StreamingResponse, title: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    stream.write_str(r#"<div class="card">"#)?;
    stream.write_str("<h2>")?;
    stream.write_str(title)?;
    stream.write_str("</h2>")?;
    stream.write_str("<div class=\"card-content\">")?;
    stream.write_str(content)?;
    stream.write_str("</div></div>")?;
    Ok(())
}

/// Format a value into a heapless string
pub fn format_value<T: core::fmt::Display>(value: T) -> heapless::String<64> {
    let mut s = heapless::String::new();
    write!(&mut s, "{}", value).ok();
    s
}