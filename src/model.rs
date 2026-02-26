//! Model name resolution and provider detection.

/// Supported API providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Google Gemini API.
    Gemini,
    /// `OpenAI` API.
    OpenAi,
}

/// Short name aliases for popular models.
const ALIASES: &[(&str, &str)] = &[
    ("nano-banana", "gemini-3.1-flash-image-preview"),
    ("nano-banana-pro", "gemini-3-pro-image-preview"),
    ("gpt-1.5", "gpt-image-1.5"),
    ("gpt-1", "gpt-image-1"),
    ("gpt-1-mini", "gpt-image-1-mini"),
];

/// Resolve a model name (alias or exact) to the full model identifier.
#[must_use]
pub fn resolve_model(name: &str) -> String {
    for &(alias, full) in ALIASES {
        if name == alias {
            return full.to_string();
        }
    }
    name.to_string()
}

/// Detect the provider from a resolved model name.
///
/// # Errors
///
/// Returns an error if the model name doesn't match a known provider prefix.
pub fn detect_provider(model: &str) -> Result<Provider, String> {
    if model.starts_with("gemini") {
        Ok(Provider::Gemini)
    } else if model.starts_with("gpt-image") {
        Ok(Provider::OpenAi)
    } else {
        Err(format!("Unknown provider for model '{model}'. Expected 'gemini-*' or 'gpt-image-*'."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_nano_banana() {
        assert_eq!(resolve_model("nano-banana"), "gemini-3.1-flash-image-preview");
    }

    #[test]
    fn resolve_nano_banana_pro() {
        assert_eq!(resolve_model("nano-banana-pro"), "gemini-3-pro-image-preview");
    }

    #[test]
    fn resolve_gpt_aliases() {
        assert_eq!(resolve_model("gpt-1.5"), "gpt-image-1.5");
        assert_eq!(resolve_model("gpt-1"), "gpt-image-1");
        assert_eq!(resolve_model("gpt-1-mini"), "gpt-image-1-mini");
    }

    #[test]
    fn resolve_exact_name_passthrough() {
        assert_eq!(resolve_model("gemini-3-pro-image-preview"), "gemini-3-pro-image-preview");
        assert_eq!(resolve_model("gpt-image-1.5"), "gpt-image-1.5");
    }

    #[test]
    fn detect_gemini_provider() {
        assert_eq!(detect_provider("gemini-3-pro-image-preview").unwrap(), Provider::Gemini);
    }

    #[test]
    fn detect_openai_provider() {
        assert_eq!(detect_provider("gpt-image-1").unwrap(), Provider::OpenAi);
        assert_eq!(detect_provider("gpt-image-1.5").unwrap(), Provider::OpenAi);
        assert_eq!(detect_provider("gpt-image-1-mini").unwrap(), Provider::OpenAi);
    }

    #[test]
    fn detect_unknown_provider() {
        assert!(detect_provider("dall-e-3").is_err());
        assert!(detect_provider("unknown-model").is_err());
    }
}
