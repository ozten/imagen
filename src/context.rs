//! Service context that bundles all port trait objects.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::adapters::live::gemini::GeminiGenerator;
use crate::adapters::live::openai::OpenAiGenerator;
use crate::adapters::recording::image_generator::RecordingImageGenerator;
use crate::adapters::replaying::image_generator::ReplayingImageGenerator;
use crate::cassette::config::load_cassette;
use crate::cassette::recorder::CassetteRecorder;
use crate::config::Config;
use crate::error::ImageError;
use crate::model::Provider;
use crate::ports::ImageGenerator;

/// Bundles all port trait objects into a single context.
pub struct ServiceContext {
    /// Image generator port.
    pub generator: Box<dyn ImageGenerator>,
}

/// Handle to a recording session that must be finished after use.
pub struct RecordingSession {
    recorder: Arc<Mutex<CassetteRecorder>>,
}

impl RecordingSession {
    /// Finish the recording and write cassette files to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the cassette file cannot be written.
    pub fn finish(self) -> Result<std::path::PathBuf, String> {
        let recorder = Arc::try_unwrap(self.recorder)
            .map_err(|_| "Recording adapter still has references".to_string())?
            .into_inner()
            .map_err(|e| format!("Recorder lock poisoned: {e}"))?;
        recorder.finish().map_err(|e| format!("Failed to write cassette: {e}"))
    }
}

impl ServiceContext {
    /// Create a live context for the given provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is not configured.
    pub fn live(provider: Provider, config: &Config) -> Result<Self, ImageError> {
        let generator: Box<dyn ImageGenerator> = match provider {
            Provider::Gemini => {
                let key = config.gemini_key().ok_or(ImageError::MissingApiKey {
                    provider: "Gemini".into(),
                    env_var: "GEMINI_API_KEY".into(),
                })?;
                Box::new(GeminiGenerator::new(key))
            }
            Provider::OpenAi => {
                let key = config.openai_key().ok_or(ImageError::MissingApiKey {
                    provider: "OpenAI".into(),
                    env_var: "OPENAI_API_KEY".into(),
                })?;
                Box::new(OpenAiGenerator::new(key))
            }
        };
        Ok(Self { generator })
    }

    /// Create a recording context that wraps a live adapter with a recorder.
    ///
    /// # Errors
    ///
    /// Returns an error if the recording session cannot be initialized.
    pub fn recording(
        provider: Provider,
        config: &Config,
    ) -> Result<(Self, RecordingSession), ImageError> {
        let live_ctx = Self::live(provider, config)?;

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let output_dir = std::path::PathBuf::from(".imagen/cassettes").join(&timestamp);

        let commit = get_commit_hash();
        let path = output_dir.join("image_generator.cassette.yaml");
        let recorder = Arc::new(Mutex::new(CassetteRecorder::new(
            path,
            format!("{timestamp}-image_generator"),
            &commit,
        )));

        let recording_gen = RecordingImageGenerator::new(live_ctx.generator, Arc::clone(&recorder));

        let ctx = Self { generator: Box::new(recording_gen) };
        let session = RecordingSession { recorder };

        Ok((ctx, session))
    }

    /// Create a replaying context from a cassette file.
    ///
    /// # Errors
    ///
    /// Returns an error if the cassette file cannot be loaded.
    pub fn replaying(path: &Path) -> Result<Self, ImageError> {
        let replayer = load_cassette(path)
            .map_err(|e| ImageError::Config(format!("Failed to load cassette: {e}")))?;
        let replayer = Arc::new(Mutex::new(replayer));
        let generator = Box::new(ReplayingImageGenerator::new(replayer));
        Ok(Self { generator })
    }
}

/// Get the current git commit hash, or "unknown" if unavailable.
fn get_commit_hash() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}
