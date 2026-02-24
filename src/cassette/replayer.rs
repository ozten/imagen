//! Replays recorded interactions from a cassette.

use std::collections::HashMap;

use super::format::{Cassette, Interaction};

/// Key for indexing interactions by port and method.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct PortMethodKey {
    port: String,
    method: String,
}

/// Replays interactions from a loaded cassette, serving them sequentially
/// per port/method pair.
pub struct CassetteReplayer {
    queues: HashMap<PortMethodKey, Vec<Interaction>>,
    cursors: HashMap<PortMethodKey, usize>,
}

impl CassetteReplayer {
    /// Create a new replayer from a loaded cassette.
    #[must_use]
    pub fn new(cassette: &Cassette) -> Self {
        let mut queues: HashMap<PortMethodKey, Vec<Interaction>> = HashMap::new();
        for interaction in &cassette.interactions {
            let key = PortMethodKey {
                port: interaction.port.clone(),
                method: interaction.method.clone(),
            };
            queues.entry(key).or_default().push(interaction.clone());
        }
        let cursors = queues.keys().map(|k| (k.clone(), 0)).collect();
        Self { queues, cursors }
    }

    /// Return the next interaction for the given port and method.
    ///
    /// # Panics
    ///
    /// Panics if the cassette has no (more) interactions for the given
    /// port/method combination.
    pub fn next_interaction(&mut self, port: &str, method: &str) -> &Interaction {
        let key = PortMethodKey { port: port.to_string(), method: method.to_string() };

        let queue = self.queues.get(&key).unwrap_or_else(|| {
            let available: Vec<String> =
                self.queues.keys().map(|k| format!("{}::{}", k.port, k.method)).collect();
            panic!(
                "Cassette exhausted: no interactions recorded for port={port:?} method={method:?}. \
                 Available port::method pairs: [{}]",
                available.join(", ")
            );
        });

        let cursor = self.cursors.get_mut(&key).expect("cursor must exist");
        assert!(
            *cursor < queue.len(),
            "Cassette exhausted: all {count} interactions for port={port:?} method={method:?} \
             have been consumed.",
            count = queue.len(),
        );

        let interaction = &queue[*cursor];
        *cursor += 1;
        interaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cassette::format::{Cassette, Interaction};
    use chrono::Utc;
    use serde_json::json;

    fn make_cassette(interactions: Vec<Interaction>) -> Cassette {
        Cassette {
            name: "test".into(),
            recorded_at: Utc::now(),
            commit: "abc".into(),
            interactions,
        }
    }

    #[test]
    fn replay_in_order() {
        let cassette = make_cassette(vec![
            Interaction {
                seq: 0,
                port: "image_generator".into(),
                method: "generate".into(),
                input: json!({"prompt": "first"}),
                output: json!({"Ok": {"images": []}}),
            },
            Interaction {
                seq: 1,
                port: "image_generator".into(),
                method: "generate".into(),
                input: json!({"prompt": "second"}),
                output: json!({"Ok": {"images": []}}),
            },
        ]);

        let mut replayer = CassetteReplayer::new(&cassette);

        let i1 = replayer.next_interaction("image_generator", "generate");
        assert_eq!(i1.seq, 0);

        let i2 = replayer.next_interaction("image_generator", "generate");
        assert_eq!(i2.seq, 1);
    }

    #[test]
    #[should_panic(expected = "Cassette exhausted")]
    fn exhausted_replayer_panics() {
        let cassette = make_cassette(vec![Interaction {
            seq: 0,
            port: "image_generator".into(),
            method: "generate".into(),
            input: json!({}),
            output: json!({}),
        }]);

        let mut replayer = CassetteReplayer::new(&cassette);
        let _ = replayer.next_interaction("image_generator", "generate");
        let _ = replayer.next_interaction("image_generator", "generate"); // panics
    }

    #[test]
    #[should_panic(expected = "no interactions recorded")]
    fn unknown_port_panics() {
        let cassette = make_cassette(vec![]);
        let mut replayer = CassetteReplayer::new(&cassette);
        let _ = replayer.next_interaction("unknown", "method");
    }
}
