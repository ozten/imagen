//! Live adapter for the Gemini image generation API.

use base64::Engine;
use reqwest::Client;
use serde::Deserialize;

use crate::error::ImageError;
use crate::ports::image_generator::{
    GenerateFuture, GeneratedImage, ImageGenerator, ImageRequest, ImageResponse,
};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Live Gemini image generator that calls the Google AI API.
pub struct GeminiGenerator {
    client: Client,
    api_key: String,
}

impl GeminiGenerator {
    /// Create a new Gemini generator with the given API key.
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { client: Client::new(), api_key }
    }
}

impl ImageGenerator for GeminiGenerator {
    fn generate(&self, request: &ImageRequest) -> GenerateFuture<'_> {
        let request = request.clone();
        Box::pin(async move {
            let url = format!("{GEMINI_API_BASE}/{}:generateContent", request.model);

            let mut generation_config = serde_json::json!({
                "responseModalities": ["IMAGE"],
                "imageConfig": {
                    "aspectRatio": request.aspect_ratio,
                    "imageSize": request.size,
                }
            });

            if let Some(ref thinking) = request.thinking {
                generation_config["thinkingConfig"] = serde_json::json!({
                    "thinkingLevel": thinking.to_uppercase()
                });
            }

            let body = serde_json::json!({
                "contents": [{
                    "parts": [{"text": request.prompt}]
                }],
                "generationConfig": generation_config
            });

            let response = self
                .client
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .json(&body)
                .send()
                .await?;

            let status = response.status();
            let response_text = response.text().await?;

            if !status.is_success() {
                return Err(ImageError::Api { status: status.as_u16(), message: response_text });
            }

            let parsed: GeminiResponse = serde_json::from_str(&response_text).map_err(|e| {
                ImageError::Api { status: 200, message: format!("Failed to parse response: {e}") }
            })?;

            let mut images = Vec::new();
            for candidate in parsed.candidates {
                for part in candidate.content.parts {
                    if let Some(inline) = part.inline_data {
                        let data = base64::engine::general_purpose::STANDARD
                            .decode(&inline.data)
                            .map_err(|e| ImageError::Api {
                            status: 200,
                            message: format!("Failed to decode base64: {e}"),
                        })?;
                        images.push(GeneratedImage { data, mime_type: inline.mime_type });
                    }
                }
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

// --- Gemini API response types ---

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPart {
    #[allow(dead_code)]
    text: Option<String>,
    inline_data: Option<GeminiInlineData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiInlineData {
    mime_type: String,
    data: String,
}
