//! Adapter implementations for port traits.
//!
//! - `live/` — Real API implementations
//! - `recording/` — Record interactions to cassettes
//! - `replaying/` — Replay interactions from cassettes

pub mod live;
pub mod recording;
pub mod replaying;
