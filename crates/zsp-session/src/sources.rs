//! Frame sources backed by zsp-capture.

use std::path::Path;

use zsp_capture::mp4::Mp4Frames;
use zsp_capture::{CaptureError, Region, WindowCapture};

use crate::{FrameSource, RgbaImage, SourceError};

/// A window on screen, cropped to a region. `NotFound` and `Minimized` become
/// `SourceError::Lost`; other capture errors become `SourceError::Failed`.
pub struct WindowSource {
    capture: WindowCapture,
    region: Region,
}

impl WindowSource {
    pub fn new(capture: WindowCapture, region: Region) -> Self {
        Self { capture, region }
    }
}

impl FrameSource for WindowSource {
    fn next_frame(&mut self, t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError> {
        let _ = (t, &self.capture, &self.region, CaptureError::NotFound);
        todo!()
    }

    fn is_live(&self) -> bool {
        true
    }
}

/// A video file, at `1 / interval_s` frames per second.
pub struct VideoSource {
    frames: Mp4Frames,
}

impl VideoSource {
    pub fn open(path: &Path, interval_s: f64) -> std::io::Result<Self> {
        let _ = (path, interval_s);
        todo!()
    }
}

impl FrameSource for VideoSource {
    fn next_frame(&mut self, t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError> {
        let _ = (t, &mut self.frames);
        todo!()
    }

    fn is_live(&self) -> bool {
        false
    }
}
