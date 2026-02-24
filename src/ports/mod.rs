//! Port traits defining external boundaries.
//!
//! Each trait represents a boundary between the application core and an
//! external system. Implementations live in `src/adapters/`.

pub mod image_generator;

pub use image_generator::{ImageGenerator, ImageRequest};
