//! Compression utilities for session persistence.
//!
//! Uses zstd for high-speed, high-ratio compression of session data.
//! Compressed sessions use ~10x less disk space and load faster from disk.

use std::io::{Read, Write};

/// Compress bytes using zstd.
pub fn compress(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut output = Vec::with_capacity(data.len());
    let mut encoder = zstd::Encoder::new(&mut output, 3)
        .map_err(|e| CompressionError::Encode(format!("{}", e)))?;
    encoder
        .write_all(data)
        .map_err(|e| CompressionError::Encode(format!("{}", e)))?;
    encoder
        .finish()
        .map_err(|e| CompressionError::Encode(format!("{}", e)))?;
    Ok(output)
}

/// Decompress bytes using zstd.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut decoder =
        zstd::Decoder::new(data).map_err(|e| CompressionError::Decode(format!("{}", e)))?;
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| CompressionError::Decode(format!("{}", e)))?;
    Ok(output)
}

/// Compress a JSON-serializable value to bytes.
pub fn compress_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, CompressionError> {
    let json = serde_json::to_vec(value).map_err(|e| CompressionError::Json(format!("{}", e)))?;
    compress(&json)
}

/// Decompress bytes into a deserializable type.
pub fn decompress_json<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T, CompressionError> {
    let decompressed = decompress(data)?;
    serde_json::from_slice(&decompressed).map_err(|e| CompressionError::Json(format!("{}", e)))
}

/// Compress level: 1 (fast) to 22 (best).
/// Default is 3 for a good balance of speed and ratio.
pub fn compress_with_level(data: &[u8], level: i32) -> Result<Vec<u8>, CompressionError> {
    let mut output = Vec::with_capacity(data.len());
    let mut encoder = zstd::Encoder::new(&mut output, level)
        .map_err(|e| CompressionError::Encode(format!("{}", e)))?;
    encoder
        .write_all(data)
        .map_err(|e| CompressionError::Encode(format!("{}", e)))?;
    encoder
        .finish()
        .map_err(|e| CompressionError::Encode(format!("{}", e)))?;
    Ok(output)
}

/// Get compression statistics.
pub fn stats(compressed: &[u8], original_size: usize) -> CompressionStats {
    let ratio = if original_size > 0 {
        compressed.len() as f64 / original_size as f64
    } else {
        1.0
    };
    CompressionStats {
        original_bytes: original_size,
        compressed_bytes: compressed.len(),
        ratio,
        savings_percent: ((1.0 - ratio) * 100.0).max(0.0),
    }
}

#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub ratio: f64,
    pub savings_percent: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("encode error: {0}")]
    Encode(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("json error: {0}")]
    Json(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog.".repeat(100);
        let compressed = compress(&original).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_compress_json_roundtrip() {
        let data = serde_json::json!({
            "name": "test-session",
            "messages": ["hello", "world"],
            "count": 42
        });
        let compressed = compress_json(&data).unwrap();
        let decompressed: serde_json::Value = decompress_json(&compressed).unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_compression_ratio() {
        let repeated = b"AAAAAAAABBBBBBBBCCCCCCCC".repeat(1000);
        let compressed = compress(&repeated).unwrap();
        let stats = stats(&compressed, repeated.len());
        assert!(
            stats.ratio < 0.3,
            "repeated data should compress well, got ratio {}",
            stats.ratio
        );
        assert!(stats.savings_percent > 50.0);
    }

    #[test]
    fn test_compression_stats() {
        let stats = stats(b"1234567890", 100);
        assert_eq!(stats.original_bytes, 100);
        assert_eq!(stats.compressed_bytes, 10);
        assert_eq!(stats.ratio, 0.1);
        assert_eq!(stats.savings_percent, 90.0);
    }

    #[test]
    fn test_compression_empty() {
        let compressed = compress(b"").unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, b"");
    }

    #[test]
    fn test_compress_with_level() {
        let data = b"hello world".repeat(100);
        // Level 1 = fast, level 19 = best
        let fast = compress_with_level(&data, 1).unwrap();
        let best = compress_with_level(&data, 19).unwrap();
        // Better compression at higher level
        assert!(best.len() <= fast.len());
        assert_eq!(decompress(&best).unwrap(), decompress(&fast).unwrap());
    }
}
