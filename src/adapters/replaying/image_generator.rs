//! Replaying adapter for the `ImageGenerator` port.

use std::sync::{Arc, Mutex};

use super::{next_output, replay_result};
use crate::cassette::replayer::CassetteReplayer;
use crate::error::ImageError;
use crate::ports::image_generator::{GenerateFuture, ImageGenerator, ImageRequest, ImageResponse};

/// Serves recorded image generation results from a cassette.
pub struct ReplayingImageGenerator {
    replayer: Option<Arc<Mutex<CassetteReplayer>>>,
}

impl ReplayingImageGenerator {
    /// Create a replaying generator backed by the given replayer.
    #[must_use]
    pub fn new(replayer: Arc<Mutex<CassetteReplayer>>) -> Self {
        Self { replayer: Some(replayer) }
    }
}

impl ImageGenerator for ReplayingImageGenerator {
    fn generate(&self, _request: &ImageRequest) -> GenerateFuture<'_> {
        let output = next_output(self.replayer.as_ref(), "image_generator", "generate");
        Box::pin(async move {
            replay_result::<ImageResponse>(output)
                .map_err(|e| ImageError::Api { status: 0, message: e.to_string() })
        })
    }
}
