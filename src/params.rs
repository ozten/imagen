//! Parameter translation between CLI inputs and provider-specific formats.

use crate::model::Provider;

/// Translate an aspect ratio string to `OpenAI` pixel dimensions.
///
/// `OpenAI` supports: `1024x1024`, `1536x1024`, `1024x1536`, `auto`.
#[must_use]
pub fn aspect_ratio_to_openai_size(ratio: &str) -> &'static str {
    match ratio {
        "1:1" => "1024x1024",
        // Landscape ratios
        "16:9" | "3:2" | "4:3" | "21:9" | "5:4" => "1536x1024",
        // Portrait ratios
        "9:16" | "2:3" | "3:4" | "4:5" => "1024x1536",
        _ => "auto",
    }
}

/// Validate that an aspect ratio is supported by the given provider.
///
/// # Errors
///
/// Returns an error if the ratio is not recognized.
pub fn validate_aspect_ratio(ratio: &str, provider: Provider) -> Result<(), String> {
    let valid_gemini = ["1:1", "2:3", "3:2", "3:4", "4:3", "4:5", "5:4", "9:16", "16:9", "21:9"];
    let valid_openai = ["1:1", "16:9", "9:16", "3:2", "2:3", "4:3", "3:4", "5:4", "4:5", "21:9"];

    let valid = match provider {
        Provider::Gemini => &valid_gemini[..],
        Provider::OpenAi => &valid_openai[..],
    };

    if valid.contains(&ratio) {
        Ok(())
    } else {
        Err(format!("Unsupported aspect ratio '{ratio}' for {provider:?}. Valid: {valid:?}"))
    }
}

/// Validate the image size parameter.
///
/// # Errors
///
/// Returns an error if the size is not recognized.
pub fn validate_size(size: &str) -> Result<(), String> {
    match size {
        "1K" | "2K" | "4K" => Ok(()),
        _ => Err(format!("Unsupported size '{size}'. Valid: 1K, 2K, 4K")),
    }
}

/// Validate the quality parameter.
///
/// # Errors
///
/// Returns an error if the quality value is not recognized.
pub fn validate_quality(quality: &str) -> Result<(), String> {
    match quality {
        "auto" | "low" | "medium" | "high" => Ok(()),
        _ => Err(format!("Unsupported quality '{quality}'. Valid: auto, low, medium, high")),
    }
}

/// Validate the output format parameter.
///
/// # Errors
///
/// Returns an error if the format is not recognized.
pub fn validate_format(format: &str) -> Result<(), String> {
    match format {
        "jpeg" | "png" | "webp" => Ok(()),
        _ => Err(format!("Unsupported format '{format}'. Valid: jpeg, png, webp")),
    }
}

/// Validate the thinking level parameter (Gemini only).
///
/// # Errors
///
/// Returns an error if the thinking level is not recognized.
pub fn validate_thinking(thinking: &str, provider: Provider) -> Result<(), String> {
    if provider != Provider::Gemini {
        return Err("--thinking is only supported for Gemini models".to_string());
    }
    match thinking {
        "none" | "minimal" | "low" | "medium" | "high" => Ok(()),
        _ => Err(format!(
            "Unsupported thinking level '{thinking}'. Valid: none, minimal, low, medium, high"
        )),
    }
}

/// Get the file extension for an output format.
#[must_use]
pub fn format_extension(format: &str) -> &'static str {
    match format {
        "png" => "png",
        "webp" => "webp",
        // jpeg and any unknown format default to jpg
        _ => "jpg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_square() {
        assert_eq!(aspect_ratio_to_openai_size("1:1"), "1024x1024");
    }

    #[test]
    fn aspect_ratio_landscape() {
        assert_eq!(aspect_ratio_to_openai_size("16:9"), "1536x1024");
        assert_eq!(aspect_ratio_to_openai_size("3:2"), "1536x1024");
        assert_eq!(aspect_ratio_to_openai_size("4:3"), "1536x1024");
        assert_eq!(aspect_ratio_to_openai_size("21:9"), "1536x1024");
        assert_eq!(aspect_ratio_to_openai_size("5:4"), "1536x1024");
    }

    #[test]
    fn aspect_ratio_portrait() {
        assert_eq!(aspect_ratio_to_openai_size("9:16"), "1024x1536");
        assert_eq!(aspect_ratio_to_openai_size("2:3"), "1024x1536");
        assert_eq!(aspect_ratio_to_openai_size("3:4"), "1024x1536");
        assert_eq!(aspect_ratio_to_openai_size("4:5"), "1024x1536");
    }

    #[test]
    fn aspect_ratio_unknown_returns_auto() {
        assert_eq!(aspect_ratio_to_openai_size("7:3"), "auto");
    }

    #[test]
    fn validate_aspect_ratio_gemini() {
        assert!(validate_aspect_ratio("1:1", Provider::Gemini).is_ok());
        assert!(validate_aspect_ratio("16:9", Provider::Gemini).is_ok());
        assert!(validate_aspect_ratio("21:9", Provider::Gemini).is_ok());
    }

    #[test]
    fn validate_aspect_ratio_openai() {
        assert!(validate_aspect_ratio("1:1", Provider::OpenAi).is_ok());
        assert!(validate_aspect_ratio("16:9", Provider::OpenAi).is_ok());
    }

    #[test]
    fn validate_size_valid() {
        assert!(validate_size("1K").is_ok());
        assert!(validate_size("2K").is_ok());
        assert!(validate_size("4K").is_ok());
    }

    #[test]
    fn validate_size_invalid() {
        assert!(validate_size("8K").is_err());
        assert!(validate_size("small").is_err());
    }

    #[test]
    fn validate_quality_valid() {
        assert!(validate_quality("auto").is_ok());
        assert!(validate_quality("low").is_ok());
        assert!(validate_quality("medium").is_ok());
        assert!(validate_quality("high").is_ok());
    }

    #[test]
    fn validate_quality_invalid() {
        assert!(validate_quality("ultra").is_err());
    }

    #[test]
    fn validate_format_valid() {
        assert!(validate_format("jpeg").is_ok());
        assert!(validate_format("png").is_ok());
        assert!(validate_format("webp").is_ok());
    }

    #[test]
    fn validate_format_invalid() {
        assert!(validate_format("gif").is_err());
        assert!(validate_format("bmp").is_err());
    }

    #[test]
    fn validate_thinking_valid() {
        assert!(validate_thinking("none", Provider::Gemini).is_ok());
        assert!(validate_thinking("minimal", Provider::Gemini).is_ok());
        assert!(validate_thinking("low", Provider::Gemini).is_ok());
        assert!(validate_thinking("medium", Provider::Gemini).is_ok());
        assert!(validate_thinking("high", Provider::Gemini).is_ok());
    }

    #[test]
    fn validate_thinking_invalid() {
        assert!(validate_thinking("ultra", Provider::Gemini).is_err());
    }

    #[test]
    fn validate_thinking_wrong_provider() {
        assert!(validate_thinking("medium", Provider::OpenAi).is_err());
    }

    #[test]
    fn format_extension_mapping() {
        assert_eq!(format_extension("jpeg"), "jpg");
        assert_eq!(format_extension("png"), "png");
        assert_eq!(format_extension("webp"), "webp");
    }
}
