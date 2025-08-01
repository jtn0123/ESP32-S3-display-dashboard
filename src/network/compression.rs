use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use esp_idf_svc::http::server::Request;
use esp_idf_svc::io::Write as EspWrite;

/// Compress data using gzip
pub fn gzip_compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

/// Write compressed response if client supports gzip
pub fn write_compressed_response<'a>(
    req: Request<&mut esp_idf_svc::http::server::EspHttpConnection<'a>>,
    content: &[u8],
    content_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if client accepts gzip by looking for Accept-Encoding header
    let accept_encoding = req.header("Accept-Encoding").unwrap_or("");
    let accepts_gzip = accept_encoding.contains("gzip");
    
    if accepts_gzip && content.len() > 1000 { // Only compress if > 1KB
        match gzip_compress(content) {
            Ok(compressed) => {
                log::debug!("Compressed {} bytes to {} bytes", content.len(), compressed.len());
                let mut response = req.into_response(
                    200,
                    Some("OK"),
                    &[
                        ("Content-Type", content_type),
                        ("Content-Encoding", "gzip"),
                        ("Vary", "Accept-Encoding"),
                    ]
                )?;
                response.write_all(&compressed)?;
            }
            Err(e) => {
                log::warn!("Compression failed: {}", e);
                let mut response = req.into_ok_response()?;
                response.write_all(content)?;
            }
        }
    } else {
        let mut response = req.into_ok_response()?;
        response.write_all(content)?;
    }
    
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_compress() {
        let data = b"Hello, World! This is a test of gzip compression.";
        let compressed = gzip_compress(data).unwrap();
        
        // Compressed should be smaller for this text
        assert!(compressed.len() < data.len());
        
        // Check gzip magic number
        assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
    }

}