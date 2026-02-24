//! Live adapter for the `OpenAI` image generation API.

use base64::Engine;
use reqwest::Client;
use serde::Deserialize;

use crate::error::ImageError;
use crate::params::aspect_ratio_to_openai_size;
use crate::ports::image_generator::{
    GenerateFuture, GeneratedImage, ImageGenerator, ImageRequest, ImageResponse,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/images/generations";

/// Live `OpenAI` image generator that calls the `OpenAI` Images API.
pub struct OpenAiGenerator {
    client: Client,
    api_key: String,
}

impl OpenAiGenerator {
    /// Create a new `OpenAI` generator with the given API key.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { client: Client::new(), api_key }
    }
}

impl ImageGenerator for OpenAiGenerator {
    fn generate(&self, request: &ImageRequest) -> GenerateFuture<'_> {
        let request = request.clone();
        Box::pin(async move {
            // OpenAI only supports 1K-range sizes (1024px); for 2K/4K use "auto".
            let size = if request.size == "1K" {
                aspect_ratio_to_openai_size(&request.aspect_ratio)
            } else {
                "auto"
            };

            let body = serde_json::json!({
                "model": request.model,
                "prompt": request.prompt,
                "n": request.count,
                "size": size,
                "quality": request.quality,
                "output_format": request.format,
            });

            let response = self
                .client
                .post(OPENAI_API_URL)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body)
                .send()
                .await?;

            let status = response.status();
            let response_text = response.text().await?;

            if !status.is_success() {
                return Err(ImageError::Api { status: status.as_u16(), message: response_text });
            }

            let parsed: OpenAiResponse = serde_json::from_str(&response_text).map_err(|e| {
                ImageError::Api { status: 200, message: format!("Failed to parse response: {e}") }
            })?;

            let mime_type = format!("image/{}", request.format);
            let mut images = Vec::new();
            for item in parsed.data {
                let data = base64::engine::general_purpose::STANDARD
                    .decode(&item.b64_json)
                    .map_err(|e| ImageError::Api {
                        status: 200,
                        message: format!("Failed to decode base64: {e}"),
                    })?;
                images.push(GeneratedImage { data, mime_type: mime_type.clone() });
            }

            if images.is_empty() {
                let truncated = if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text.clone()
                };
                return Err(ImageError::Api {
                    status: 200,
                    message: format!("No images in response. Body: {truncated}"),
                });
            }

            Ok(ImageResponse { images })
        })
    }
}

// --- OpenAI API response types ---

#[derive(Deserialize)]
struct OpenAiResponse {
    data: Vec<OpenAiImageData>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    b64_json: String,
}
