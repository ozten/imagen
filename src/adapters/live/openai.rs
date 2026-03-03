//! Live adapter for the `OpenAI` image generation API.

use base64::Engine;
use reqwest::Client;
use reqwest::multipart;
use serde::Deserialize;

use crate::error::ImageError;
use crate::params::aspect_ratio_to_openai_size;
use crate::ports::image_generator::{
    GenerateFuture, GeneratedImage, ImageGenerator, ImageRequest, ImageResponse,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/images/generations";
const OPENAI_EDITS_API_URL: &str = "https://api.openai.com/v1/images/edits";

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

/// Parse an `OpenAI` image response body into `ImageResponse`.
fn parse_response(response_text: &str, format: &str) -> Result<ImageResponse, ImageError> {
    let parsed: OpenAiResponse = serde_json::from_str(response_text).map_err(|e| {
        ImageError::Api { status: 200, message: format!("Failed to parse response: {e}") }
    })?;

    let mime_type = format!("image/{format}");
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
            response_text.to_string()
        };
        return Err(ImageError::Api {
            status: 200,
            message: format!("No images in response. Body: {truncated}"),
        });
    }

    Ok(ImageResponse { images })
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

            let response_text = if request.input_images.is_empty() {
                // --- Text-to-image: JSON POST to /generations ---
                let mut body = serde_json::json!({
                    "model": request.model,
                    "prompt": request.prompt,
                    "n": request.count,
                    "size": size,
                    "quality": request.quality,
                    "output_format": request.format,
                });
                if let Some(ref bg) = request.background {
                    body["background"] = serde_json::Value::String(bg.clone());
                }

                let response = self
                    .client
                    .post(OPENAI_API_URL)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .json(&body)
                    .send()
                    .await?;

                let status = response.status();
                let text = response.text().await?;
                if !status.is_success() {
                    return Err(ImageError::Api { status: status.as_u16(), message: text });
                }
                text
            } else {
                // --- Image editing: multipart POST to /edits ---
                let mut form = multipart::Form::new()
                    .text("model", request.model.clone())
                    .text("prompt", request.prompt.clone())
                    .text("n", request.count.to_string())
                    .text("size", size.to_string())
                    .text("quality", request.quality.clone())
                    .text("output_format", request.format.clone());

                if let Some(ref bg) = request.background {
                    form = form.text("background", bg.clone());
                }

                for img in &request.input_images {
                    let part = multipart::Part::bytes(img.data.clone())
                        .file_name(img.filename.clone())
                        .mime_str(&img.mime_type)
                        .map_err(|e| ImageError::Api {
                            status: 0,
                            message: format!("Failed to build multipart: {e}"),
                        })?;
                    form = form.part("image[]", part);
                }

                let response = self
                    .client
                    .post(OPENAI_EDITS_API_URL)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .multipart(form)
                    .send()
                    .await?;

                let status = response.status();
                let text = response.text().await?;
                if !status.is_success() {
                    return Err(ImageError::Api { status: status.as_u16(), message: text });
                }
                text
            };

            parse_response(&response_text, &request.format)
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
