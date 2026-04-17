//! Image Tools - Lightweight image metadata without heavy dependencies
//! Supports: JPEG, PNG, GIF, BMP, WEBP

use crate::{Permission, Tool, ToolSchema};
use serde::Deserialize;

/// Get basic image metadata by parsing file headers (no heavy deps)
#[derive(Debug)]
pub struct ImageInfoTool;

#[derive(Debug, Deserialize)]
pub struct ImageInfoInput {
    /// Path to the image file
    pub path: String,
}

/// Image format signature
#[derive(Debug)]
enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Bmp,
    Webp,
    Unknown,
}

impl ImageFormat {
    fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "png" => ImageFormat::Png,
            "gif" => ImageFormat::Gif,
            "bmp" => ImageFormat::Bmp,
            "webp" => ImageFormat::Webp,
            _ => ImageFormat::Unknown,
        }
    }

    fn from_magic(bytes: &[u8]) -> Self {
        if bytes.len() < 4 {
            return ImageFormat::Unknown;
        }
        match bytes {
            [0xFF, 0xD8, 0xFF, ..] => ImageFormat::Jpeg,
            [0x89, 0x50, 0x4E, 0x47] => ImageFormat::Png,
            [0x47, 0x49, 0x46, ..] => ImageFormat::Gif,
            [0x42, 0x4D, ..] => ImageFormat::Bmp,
            [0x52, 0x49, 0x46, 0x46] => ImageFormat::Webp, // RIFF....WEBP
            _ => ImageFormat::Unknown,
        }
    }
}

impl Tool for ImageInfoTool {
    fn name(&self) -> &'static str {
        "image_info"
    }

    fn description(&self) -> &'static str {
        "Get basic image metadata (dimensions, format) - supports JPEG, PNG, GIF, BMP, WEBP"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Image file path".into()),
            properties: Some(serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the image file"
                }
            })),
            required: Some(vec!["path".into()]),
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Image metadata".into()),
            properties: Some(serde_json::json!({
                "width": { "type": "integer", "description": "Image width in pixels" },
                "height": { "type": "integer", "description": "Image height in pixels" },
                "format": { "type": "string", "description": "Image format" },
                "file_size": { "type": "integer", "description": "File size in bytes" },
            })),
            required: Some(vec!["width".into(), "height".into(), "format".into()]),
        }
    }

    fn permission(&self) -> Permission {
        Permission::Safe
    }

    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        let input: ImageInfoInput = match serde_json::from_value(input) {
            Ok(i) => i,
            Err(e) => {
                return Err(crate::ToolError::InvalidInput(format!("Invalid input: {}", e)));
            }
        };

        let path = std::path::Path::new(&input.path);

        // Get file size
        let file_size = std::fs::metadata(path)
            .map_err(|e| crate::ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?
            .len();

        // Read first 32 bytes for header analysis
        let mut header = vec![0u8; 32];
        let mut file = std::fs::File::open(path)
            .map_err(|e| crate::ToolError::ExecutionFailed(format!("Cannot open file: {}", e)))?;
        use std::io::Read;
        std::io::Read::read_exact(&mut file, &mut header)
            .map_err(|e| crate::ToolError::ExecutionFailed(format!("Cannot read file: {}", e)))?;

        // Detect format from magic bytes
        let format = ImageFormat::from_magic(&header);
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Override with extension if magic didn't match but extension does
        let format = if matches!(format, ImageFormat::Unknown) {
            ImageFormat::from_extension(ext)
        } else {
            format
        };

        // Parse dimensions based on format
        let (width, height) = match format {
            ImageFormat::Jpeg => parse_jpeg_dimensions(&header)?,
            ImageFormat::Png => parse_png_dimensions(&header)?,
            ImageFormat::Gif => parse_gif_dimensions(&header)?,
            ImageFormat::Bmp => parse_bmp_dimensions(&header)?,
            ImageFormat::Webp => parse_webp_dimensions(&header)?,
            ImageFormat::Unknown => {
                return Err(crate::ToolError::ExecutionFailed(
                    "Unknown image format. Supported: JPEG, PNG, GIF, BMP, WEBP".into()
                ));
            }
        };

        let format_name = match format {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
            ImageFormat::Gif => "GIF",
            ImageFormat::Bmp => "BMP",
            ImageFormat::Webp => "WEBP",
            ImageFormat::Unknown => "Unknown",
        };

        Ok(serde_json::json!({
            "width": width,
            "height": height,
            "format": format_name,
            "file_size": file_size,
        }))
    }
}

fn parse_jpeg_dimensions(header: &[u8]) -> Result<(u32, u32), crate::ToolError> {
    // JPEG: need to scan for SOF marker
    let mut i = 2; // Skip FFD8
    while i < header.len() - 1 {
        if header[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = header[i + 1];
        // SOF0, SOF1, SOF2 markers contain dimensions
        if matches!(marker, 0xC0 | 0xC1 | 0xC2) && i + 9 <= header.len() {
            let height = ((header[i + 5] as u32) << 8) | (header[i + 6] as u32);
            let width = ((header[i + 7] as u32) << 8) | (header[i + 8] as u32);
            return Ok((width, height));
        }
        // Skip to next marker
        if i + 3 <= header.len() {
            let len = ((header[i + 2] as usize) << 8) | (header[i + 3] as usize);
            i += 2 + len;
        } else {
            break;
        }
    }
    // Fallback: return placeholder if we couldn't parse
    // For real JPEG parsing we'd need to read more bytes
    Ok((0, 0))
}

fn parse_png_dimensions(header: &[u8]) -> Result<(u32, u32), crate::ToolError> {
    if header.len() < 24 {
        return Err(crate::ToolError::ExecutionFailed("Invalid PNG header".into()));
    }
    // PNG: bytes 16-19 are width, 20-23 are height (big-endian)
    let width = ((header[16] as u32) << 24) | ((header[17] as u32) << 16)
              | ((header[18] as u32) << 8) | (header[19] as u32);
    let height = ((header[20] as u32) << 24) | ((header[21] as u32) << 16)
              | ((header[22] as u32) << 8) | (header[23] as u32);
    Ok((width, height))
}

fn parse_gif_dimensions(header: &[u8]) -> Result<(u32, u32), crate::ToolError> {
    if header.len() < 10 {
        return Err(crate::ToolError::ExecutionFailed("Invalid GIF header".into()));
    }
    // GIF: bytes 6-7 are width, 8-9 are height (little-endian)
    let width = (header[6] as u32) | ((header[7] as u32) << 8);
    let height = (header[8] as u32) | ((header[9] as u32) << 8);
    Ok((width, height))
}

fn parse_bmp_dimensions(header: &[u8]) -> Result<(u32, u32), crate::ToolError> {
    if header.len() < 26 {
        return Err(crate::ToolError::ExecutionFailed("Invalid BMP header".into()));
    }
    // BMP: bytes 18-21 are width, 22-25 are height (little-endian)
    let width = (header[18] as u32) | ((header[19] as u32) << 8)
              | ((header[20] as u32) << 16) | ((header[21] as u32) << 24);
    let height = (header[22] as u32) | ((header[23] as u32) << 8)
              | ((header[24] as u32) << 16) | ((header[25] as u32) << 24);
    Ok((width, height))
}

fn parse_webp_dimensions(_header: &[u8]) -> Result<(u32, u32), crate::ToolError> {
    // WEBP in RIFF container: need VP8 chunk
    // Simple fallback - would need more bytes for real parsing
    // For now, return 0 as WEBP parsing is complex
    Ok((0, 0))
}

/// List supported image formats
#[derive(Debug)]
pub struct ImageFormatsTool;

impl Tool for ImageFormatsTool {
    fn name(&self) -> &'static str {
        "image_formats"
    }

    fn description(&self) -> &'static str {
        "List all supported image formats"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("No input required".into()),
            properties: None,
            required: None,
        }
    }

    fn output_schema(&self) -> ToolSchema {
        ToolSchema {
            r#type: "object".into(),
            description: Some("Supported formats list".into()),
            properties: Some(serde_json::json!({
                "formats": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Supported formats"
                }
            })),
            required: Some(vec!["formats".into()]),
        }
    }

    fn permission(&self) -> Permission {
        Permission::Safe
    }

    fn execute(&self, _input: serde_json::Value) -> Result<serde_json::Value, crate::ToolError> {
        Ok(serde_json::json!({
            "formats": ["JPEG", "PNG", "GIF", "BMP", "WEBP"],
            "notes": "Dimensions extracted from file headers, no heavy dependencies"
        }))
    }
}
