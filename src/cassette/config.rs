//! Cassette configuration for loading and replaying.

use std::path::Path;

use super::format::Cassette;
use super::replayer::CassetteReplayer;

/// Load a cassette file and create a replayer.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_cassette(path: &Path) -> Result<CassetteReplayer, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read cassette file {}: {e}", path.display()))?;
    let cassette: Cassette = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse cassette file {}: {e}", path.display()))?;
    Ok(CassetteReplayer::new(&cassette))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cassette::format::{Cassette, Interaction};
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn load_valid_cassette() {
        let dir = std::env::temp_dir().join("imagen_cassette_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.cassette.yaml");

        let cassette = Cassette {
            name: "test".into(),
            recorded_at: Utc::now(),
            commit: "abc".into(),
            interactions: vec![Interaction {
                seq: 0,
                port: "image_generator".into(),
                method: "generate".into(),
                input: json!({}),
                output: json!({"Ok": {"images": []}}),
            }],
        };
        let yaml = serde_yaml::to_string(&cassette).unwrap();
        std::fs::write(&path, yaml).unwrap();

        let mut replayer = load_cassette(&path).unwrap();
        let i = replayer.next_interaction("image_generator", "generate");
        assert_eq!(i.seq, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_nonexistent_fails() {
        assert!(load_cassette(Path::new("/nonexistent/cassette.yaml")).is_err());
    }
}
