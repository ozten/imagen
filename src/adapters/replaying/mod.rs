//! Replaying adapters that serve recorded interactions from cassettes.

pub mod image_generator;

use std::sync::{Arc, Mutex};

use crate::cassette::replayer::CassetteReplayer;

/// Retrieve the next recorded output for a given port and method.
///
/// # Panics
///
/// Panics if the replayer is `None` or the cassette has no more interactions.
pub(crate) fn next_output(
    replayer: Option<&Arc<Mutex<CassetteReplayer>>>,
    port: &str,
    method: &str,
) -> serde_json::Value {
    let replayer = replayer.unwrap_or_else(|| {
        panic!(
            "Replaying adapter: no cassette configured for port '{port}'. \
             Configure a cassette or use live mode."
        );
    });
    let mut guard = replayer.lock().expect("replayer lock poisoned");
    guard.next_interaction(port, method).output.clone()
}

/// Deserialize a replayed output as `Result<T, Error>`.
pub(crate) fn replay_result<T: serde::de::DeserializeOwned>(
    output: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(err_val) = output.get("Err").or_else(|| output.get("err")) {
        let msg = err_val.as_str().unwrap_or("replayed error").to_string();
        return Err(msg.into());
    }
    if let Some(ok_val) = output.get("Ok").or_else(|| output.get("ok")) {
        return serde_json::from_value(ok_val.clone())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
    }
    serde_json::from_value(output)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}
