//! Configuration file loading with environment variable overrides.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level configuration.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// API key configuration.
    #[serde(default)]
    pub keys: KeysConfig,

    /// Default parameter values (used when CLI flags are at their defaults).
    #[serde(default)]
    #[allow(dead_code)] // Wired in Phase 2 when config defaults override CLI defaults
    pub defaults: DefaultsConfig,
}

/// API key configuration.
#[derive(Debug, Default, Deserialize)]
pub struct KeysConfig {
    /// Gemini API key.
    pub gemini: Option<String>,
    /// `OpenAI` API key.
    pub openai: Option<String>,
}

/// Default parameter values from config file.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used in Phase 2 when config defaults override CLI defaults
pub struct DefaultsConfig {
    /// Default model name.
    pub model: String,
    /// Default aspect ratio.
    pub aspect_ratio: String,
    /// Default image size.
    pub size: String,
    /// Default quality.
    pub quality: String,
    /// Default output format.
    pub format: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            model: "nano-banana".to_string(),
            aspect_ratio: "1:1".to_string(),
            size: "1K".to_string(),
            quality: "auto".to_string(),
            format: "jpeg".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from the given path, or return defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config {}: {e}", path.display()))?;
        toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config {}: {e}", path.display()))
    }

    /// Get the Gemini API key, preferring environment variable.
    #[must_use]
    pub fn gemini_key(&self) -> Option<String> {
        std::env::var("GEMINI_API_KEY").ok().or_else(|| self.keys.gemini.clone())
    }

    /// Get the `OpenAI` API key, preferring environment variable.
    #[must_use]
    pub fn openai_key(&self) -> Option<String> {
        std::env::var("OPENAI_API_KEY").ok().or_else(|| self.keys.openai.clone())
    }
}

/// Discover the config file path using the resolution order:
/// 1. Explicit path (from `--config` flag)
/// 2. `IMAGEN_CONFIG` environment variable
/// 3. `~/.config/imagen/config.toml`
#[must_use]
pub fn discover_config_path(explicit: Option<&str>) -> PathBuf {
    if let Some(p) = explicit {
        return PathBuf::from(p);
    }

    if let Ok(p) = std::env::var("IMAGEN_CONFIG") {
        return PathBuf::from(p);
    }

    default_config_path()
}

/// Default config path: `~/.config/imagen/config.toml`.
fn default_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/imagen/config.toml")
    } else {
        PathBuf::from("imagen.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::default();
        assert!(config.keys.gemini.is_none());
        assert!(config.keys.openai.is_none());
        assert_eq!(config.defaults.model, "nano-banana");
        assert_eq!(config.defaults.aspect_ratio, "1:1");
        assert_eq!(config.defaults.size, "1K");
        assert_eq!(config.defaults.quality, "auto");
        assert_eq!(config.defaults.format, "jpeg");
    }

    #[test]
    fn load_nonexistent_returns_defaults() {
        let config = Config::load(Path::new("/nonexistent/path/config.toml")).unwrap();
        assert_eq!(config.defaults.model, "nano-banana");
    }

    #[test]
    fn load_valid_toml() {
        let dir = std::env::temp_dir().join("imagen_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            r#"
[keys]
gemini = "test-gemini-key"
openai = "test-openai-key"

[defaults]
model = "gpt-1"
aspect_ratio = "16:9"
size = "2K"
quality = "high"
format = "png"
"#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.keys.gemini.as_deref(), Some("test-gemini-key"));
        assert_eq!(config.keys.openai.as_deref(), Some("test-openai-key"));
        assert_eq!(config.defaults.model, "gpt-1");
        assert_eq!(config.defaults.aspect_ratio, "16:9");
        assert_eq!(config.defaults.size, "2K");
        assert_eq!(config.defaults.quality, "high");
        assert_eq!(config.defaults.format, "png");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_invalid_toml() {
        let dir = std::env::temp_dir().join("imagen_config_bad_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.toml");
        std::fs::write(&path, "this is not valid toml {{{").unwrap();

        assert!(Config::load(&path).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gemini_key_env_override() {
        let config = Config {
            keys: KeysConfig { gemini: Some("from-file".into()), openai: None },
            ..Config::default()
        };

        // Without env var, returns file value
        std::env::remove_var("GEMINI_API_KEY");
        assert_eq!(config.gemini_key().as_deref(), Some("from-file"));
    }

    #[test]
    fn discover_explicit_path() {
        let path = discover_config_path(Some("/tmp/my-config.toml"));
        assert_eq!(path, PathBuf::from("/tmp/my-config.toml"));
    }
}
