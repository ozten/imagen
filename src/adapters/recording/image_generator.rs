//! Recording adapter for the `ImageGenerator` port.

use std::sync::{Arc, Mutex};

use super::record_result;
use crate::cassette::recorder::CassetteRecorder;
use crate::ports::image_generator::{GenerateFuture, ImageGenerator, ImageRequest};

/// Records image generation interactions while delegating to an inner implementation.
pub struct RecordingImageGenerator {
    inner: Box<dyn ImageGenerator>,
    recorder: Arc<Mutex<CassetteRecorder>>,
}

impl RecordingImageGenerator {
    /// Creates a new recording generator wrapping the given implementation.
    pub fn new(inner: Box<dyn ImageGenerator>, recorder: Arc<Mutex<CassetteRecorder>>) -> Self {
        Self { inner, recorder }
    }
}

impl ImageGenerator for RecordingImageGenerator {
    fn generate(&self, request: &ImageRequest) -> GenerateFuture<'_> {
        let request_clone = request.clone();
        let recorder = Arc::clone(&self.recorder);

        Box::pin(async move {
            let result = self.inner.generate(&request_clone).await;
            record_result(&recorder, "image_generator", "generate", &request_clone, &result);
            result
        })
    }
}
