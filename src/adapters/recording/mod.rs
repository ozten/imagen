//! Recording adapters that capture interactions to cassettes.
//!
//! Placeholder for Phase 3 implementation.

pub mod image_generator;

use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::cassette::recorder::CassetteRecorder;

/// Record a `Result<T, E>` interaction using the Ok/Err JSON convention.
pub(crate) fn record_result<T, E, I>(
    recorder: &Arc<Mutex<CassetteRecorder>>,
    port: &str,
    method: &str,
    input: &I,
    result: &Result<T, E>,
) where
    T: Serialize,
    E: std::fmt::Display,
    I: Serialize,
{
    let input_json = serde_json::to_value(input).expect("failed to serialize recording input");

    let output_json = match result {
        Ok(v) => {
            let inner = serde_json::to_value(v).expect("failed to serialize Ok value");
            serde_json::json!({ "Ok": inner })
        }
        Err(e) => serde_json::json!({ "Err": e.to_string() }),
    };

    let mut guard = recorder.lock().expect("recorder lock poisoned");
    guard.record(port, method, input_json, output_json);
}
