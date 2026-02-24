//! Unified error type for imagen.

use thiserror::Error;

/// Errors that can occur during image generation.
#[derive(Debug, Error)]
pub enum ImageError {
    /// An API returned an error response.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },

    /// A network error occurred.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("Config error: {0}")]
    Config(String),

    /// Invalid argument.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Image format conversion error.
    #[error("Image conversion error: {0}")]
    ImageConversion(String),

    /// No API key configured for the provider.
    #[error("No API key for {provider}. Set {env_var} or add it to config file.")]
    MissingApiKey {
        /// The provider name.
        provider: String,
        /// The environment variable name.
        env_var: String,
    },
}
