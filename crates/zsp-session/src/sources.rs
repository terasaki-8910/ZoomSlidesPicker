//! Frame sources backed by zsp-capture.

use std::path::Path;

use zsp_capture::mp4::Mp4Frames;
use zsp_capture::{CaptureError, Region, WindowCapture, crop};

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
        match self.capture.capture() {
            Ok(frame) => {
                // The whole window needs no copy.
                let (w, h) = frame.dimensions();
                let rect = self.region.to_pixels(w, h);
                let frame = if (rect.x, rect.y, rect.w, rect.h) == (0, 0, w, h) {
                    frame
                } else {
                    crop(&frame, &self.region)
                };
                Ok(Some((t, frame)))
            }
            Err(e @ (CaptureError::NotFound | CaptureError::Minimized)) => {
                Err(SourceError::Lost(e.to_string()))
            }
            Err(e @ CaptureError::Failed(_)) => Err(SourceError::Failed(e.to_string())),
        }
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
        let frames = Mp4Frames::open(path, 1.0 / interval_s)?;
        Ok(Self { frames })
    }
}

impl FrameSource for VideoSource {
    fn next_frame(&mut self, _t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError> {
        match self.frames.next() {
            Some(Ok((t, frame))) => Ok(Some((t, frame))),
            Some(Err(e)) => Err(SourceError::Failed(e.to_string())),
            None => Ok(None),
        }
    }

    fn is_live(&self) -> bool {
        false
    }
}
