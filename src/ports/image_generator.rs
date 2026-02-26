//! Image generator port for AI image generation APIs.

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use crate::error::ImageError;

/// A request to generate images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    /// The resolved model identifier (e.g., `"gemini-3.1-flash-image-preview"`).
    pub model: String,
    /// The text prompt describing the desired image.
    pub prompt: String,
    /// Aspect ratio (e.g., `"1:1"`, `"16:9"`).
    pub aspect_ratio: String,
    /// Image size (`"1K"`, `"2K"`, `"4K"`).
    pub size: String,
    /// Quality level (`"auto"`, `"low"`, `"medium"`, `"high"`).
    pub quality: String,
    /// Output format (`"jpeg"`, `"png"`, `"webp"`).
    pub format: String,
    /// Number of images to generate.
    pub count: u32,
    /// Thinking level for Gemini models (`"none"`, `"minimal"`, `"low"`, `"medium"`, `"high"`).
    #[serde(default)]
    pub thinking: Option<String>,
}

/// A single generated image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    /// Raw image bytes (decoded from base64).
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
    /// MIME type of the image (e.g., `"image/jpeg"`).
    pub mime_type: String,
}

/// Response containing generated images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    /// The generated images.
    pub images: Vec<GeneratedImage>,
}

/// Boxed future type returned by [`ImageGenerator::generate`].
pub type GenerateFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ImageResponse, ImageError>> + Send + 'a>>;

/// Generates images from text prompts via an external API.
pub trait ImageGenerator: Send + Sync {
    /// Generate images for the given request.
    fn generate(&self, request: &ImageRequest) -> GenerateFuture<'_>;
}

/// Serde helper for serializing `Vec<u8>` as base64 strings in cassettes.
mod base64_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serialize bytes as base64 string.
    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        serializer.serialize_str(&encoded)
    }

    /// Deserialize base64 string to bytes.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_request_serialization() {
        let request = ImageRequest {
            model: "gemini-3.1-flash-image-preview".into(),
            prompt: "a cat".into(),
            aspect_ratio: "1:1".into(),
            size: "1K".into(),
            quality: "auto".into(),
            format: "jpeg".into(),
            count: 1,
            thinking: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ImageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "gemini-3.1-flash-image-preview");
        assert_eq!(deserialized.prompt, "a cat");
        assert!(deserialized.thinking.is_none());
    }

    #[test]
    fn image_request_with_thinking() {
        let request = ImageRequest {
            model: "gemini-3.1-flash-image-preview".into(),
            prompt: "a cat".into(),
            aspect_ratio: "1:1".into(),
            size: "1K".into(),
            quality: "auto".into(),
            format: "jpeg".into(),
            count: 1,
            thinking: Some("medium".into()),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ImageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.thinking.as_deref(), Some("medium"));
    }

    #[test]
    fn generated_image_base64_round_trip() {
        let image = GeneratedImage {
            data: vec![0xFF, 0xD8, 0xFF, 0xE0], // JPEG magic bytes
            mime_type: "image/jpeg".into(),
        };
        let json = serde_json::to_string(&image).unwrap();
        let deserialized: GeneratedImage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.data, vec![0xFF, 0xD8, 0xFF, 0xE0]);
        assert_eq!(deserialized.mime_type, "image/jpeg");
    }

    #[test]
    fn image_response_serialization() {
        let response = ImageResponse {
            images: vec![GeneratedImage { data: vec![1, 2, 3], mime_type: "image/png".into() }],
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ImageResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.images.len(), 1);
        assert_eq!(deserialized.images[0].data, vec![1, 2, 3]);
    }
}
