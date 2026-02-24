//! File naming, image saving, and format conversion.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::ImageError;
use crate::params::format_extension;

/// Generate an output filename from a prompt and format.
///
/// Sanitizes the first 50 characters of the prompt to kebab-case,
/// appends a unix timestamp, and adds the appropriate file extension.
#[must_use]
pub fn auto_filename(prompt: &str, format: &str) -> String {
    let sanitized = sanitize_for_filename(prompt, 50);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let ext = format_extension(format);
    format!("{sanitized}-{timestamp}.{ext}")
}

/// Sanitize a string for use in a filename.
///
/// Converts to lowercase, replaces non-alphanumeric chars with hyphens,
/// collapses consecutive hyphens, and trims to max length.
#[must_use]
pub fn sanitize_for_filename(input: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(max_len);
    let mut last_was_hyphen = true; // Prevents leading hyphen

    for ch in input.chars().take(max_len * 2) {
        if result.len() >= max_len {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            result.push('-');
            last_was_hyphen = true;
        }
    }

    // Trim trailing hyphen
    while result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        "image".to_string()
    } else {
        result
    }
}

/// Save raw image bytes to a file, converting format if necessary.
///
/// # Errors
///
/// Returns an error if the file cannot be written or format conversion fails.
pub fn save_image(
    data: &[u8],
    source_mime: &str,
    target_format: &str,
    output_path: &Path,
) -> Result<(), ImageError> {
    let needs_conversion = !mime_matches_format(source_mime, target_format);

    if needs_conversion {
        convert_and_save(data, target_format, output_path)
    } else {
        std::fs::write(output_path, data).map_err(ImageError::Io)
    }
}

/// Check if a MIME type matches the requested output format.
fn mime_matches_format(mime: &str, format: &str) -> bool {
    matches!((mime, format), ("image/jpeg", "jpeg") | ("image/png", "png") | ("image/webp", "webp"))
}

/// Convert image bytes to the target format and save.
fn convert_and_save(
    data: &[u8],
    target_format: &str,
    output_path: &Path,
) -> Result<(), ImageError> {
    let img = image::load_from_memory(data)
        .map_err(|e| ImageError::ImageConversion(format!("Failed to decode image: {e}")))?;

    let image_format = match target_format {
        "jpeg" => image::ImageFormat::Jpeg,
        "png" => image::ImageFormat::Png,
        "webp" => image::ImageFormat::WebP,
        other => {
            return Err(ImageError::ImageConversion(format!("Unsupported format: {other}")));
        }
    };

    img.save_with_format(output_path, image_format)
        .map_err(|e| ImageError::ImageConversion(format!("Failed to save as {target_format}: {e}")))
}

/// Resolve the output path: use explicit path or auto-generate.
#[must_use]
pub fn resolve_output_path(explicit: Option<&str>, prompt: &str, format: &str) -> PathBuf {
    match explicit {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(auto_filename(prompt, format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_basic() {
        assert_eq!(sanitize_for_filename("Hello World", 50), "hello-world");
    }

    #[test]
    fn sanitize_special_chars() {
        assert_eq!(
            sanitize_for_filename("A cat!! sitting on a mat...", 50),
            "a-cat-sitting-on-a-mat"
        );
    }

    #[test]
    fn sanitize_truncates() {
        let long = "a".repeat(100);
        let result = sanitize_for_filename(&long, 10);
        assert!(result.len() <= 10);
    }

    #[test]
    fn sanitize_empty() {
        assert_eq!(sanitize_for_filename("", 50), "image");
        assert_eq!(sanitize_for_filename("!!!", 50), "image");
    }

    #[test]
    fn sanitize_leading_special() {
        assert_eq!(sanitize_for_filename("  hello  ", 50), "hello");
    }

    #[test]
    fn auto_filename_format() {
        let name = auto_filename("a cat", "jpeg");
        assert!(name.starts_with("a-cat-"));
        assert_eq!(Path::new(&name).extension().unwrap(), "jpg");
    }

    #[test]
    fn auto_filename_png() {
        let name = auto_filename("test", "png");
        assert_eq!(Path::new(&name).extension().unwrap(), "png");
    }

    #[test]
    fn resolve_explicit() {
        let path = resolve_output_path(Some("my-image.png"), "ignored", "jpeg");
        assert_eq!(path, PathBuf::from("my-image.png"));
    }

    #[test]
    fn resolve_auto() {
        let path = resolve_output_path(None, "a cat", "jpeg");
        assert!(path.to_str().unwrap().starts_with("a-cat-"));
        assert_eq!(path.extension().unwrap(), "jpg");
    }

    #[test]
    fn mime_matches() {
        assert!(mime_matches_format("image/jpeg", "jpeg"));
        assert!(mime_matches_format("image/png", "png"));
        assert!(mime_matches_format("image/webp", "webp"));
        assert!(!mime_matches_format("image/jpeg", "png"));
        assert!(!mime_matches_format("image/png", "jpeg"));
    }
}
