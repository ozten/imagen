//! Records interactions into a cassette file.

use std::path::PathBuf;

use chrono::Utc;

use super::format::{Cassette, Interaction};

/// Records interactions and writes them as a YAML cassette file.
#[derive(Debug)]
pub struct CassetteRecorder {
    path: PathBuf,
    name: String,
    commit: String,
    interactions: Vec<Interaction>,
    next_seq: u64,
}

impl CassetteRecorder {
    /// Create a new recorder that will write to the given path.
    pub fn new(
        path: impl Into<PathBuf>,
        name: impl Into<String>,
        commit: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            commit: commit.into(),
            interactions: Vec::new(),
            next_seq: 0,
        }
    }

    /// Record an interaction. The `seq` field is assigned automatically.
    pub fn record(
        &mut self,
        port: impl Into<String>,
        method: impl Into<String>,
        input: serde_json::Value,
        output: serde_json::Value,
    ) {
        let interaction = Interaction {
            seq: self.next_seq,
            port: port.into(),
            method: method.into(),
            input,
            output,
        };
        self.next_seq += 1;
        self.interactions.push(interaction);
    }

    /// Finish recording and write the cassette YAML file to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn finish(self) -> Result<PathBuf, std::io::Error> {
        let cassette = Cassette {
            name: self.name,
            recorded_at: Utc::now(),
            commit: self.commit,
            interactions: self.interactions,
        };
        let yaml = serde_yaml::to_string(&cassette).map_err(std::io::Error::other)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, yaml)?;
        Ok(self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn record_and_finish() {
        let dir = std::env::temp_dir().join("imagen_cassette_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.cassette.yaml");

        let mut recorder = CassetteRecorder::new(&path, "test-recording", "deadbeef");
        recorder.record(
            "image_generator",
            "generate",
            json!({"prompt": "a cat"}),
            json!({"Ok": {"images": []}}),
        );
        recorder.record(
            "image_generator",
            "generate",
            json!({"prompt": "a dog"}),
            json!({"Ok": {"images": []}}),
        );

        let result_path = recorder.finish().expect("finish should succeed");
        assert_eq!(result_path, path);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("image_generator"));
        assert!(content.contains("generate"));
        assert!(content.contains("a cat"));
        assert!(content.contains("a dog"));

        let cassette: super::super::format::Cassette = serde_yaml::from_str(&content).unwrap();
        assert_eq!(cassette.interactions.len(), 2);
        assert_eq!(cassette.interactions[0].seq, 0);
        assert_eq!(cassette.interactions[1].seq, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
