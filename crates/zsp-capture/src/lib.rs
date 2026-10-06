//! Frame sources for ZoomSlidesPicker (SPEC.md > キャプチャ).
//!
//! - Windows on screen, by id, through xcap 0.9.8. On macOS this uses
//!   `CGWindowListCreateImage`, which returns the window's own pixels even when other
//!   windows cover it (verified in spike/). Needs Screen Recording permission on macOS.
//! - A crop region given as fractions of the window, so it follows window resizes.
//! - Frames of a video file through the `ffmpeg` command on PATH (the mp4 mode).

use std::fmt;

pub use image::RgbaImage;

pub mod mp4;
pub mod permission;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: u32,
    pub pid: u32,
    pub app: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum CaptureError {
    /// No window with that id (closed, or never existed).
    NotFound,
    /// The window is minimized; it cannot be captured.
    Minimized,
    /// Capture failed for another reason (message from the platform).
    Failed(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::NotFound => write!(f, "window not found"),
            CaptureError::Minimized => write!(f, "window is minimized"),
            CaptureError::Failed(m) => write!(f, "capture failed: {m}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Windows the user can pick: visible (not minimized), at least 200 x 120, with a title,
/// not owned by this process, and not system UI (Window Server, Dock, Control Center,
/// Notification Center, including their Japanese names). Sorted by app name, then title.
pub fn list_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    todo!()
}

/// A window being captured. Keeps the xcap handle so each capture is a single call.
pub struct WindowCapture {
    info: WindowInfo,
}

impl WindowCapture {
    /// Find the window by id. `NotFound` if it is gone, `Minimized` if minimized.
    pub fn open(id: u32) -> Result<Self, CaptureError> {
        let _ = id;
        todo!()
    }

    pub fn info(&self) -> &WindowInfo {
        &self.info
    }

    /// Capture the whole window in pixels (Retina: 2x the window's point size).
    /// `NotFound` once the window has closed, `Minimized` while minimized.
    pub fn capture(&self) -> Result<RgbaImage, CaptureError> {
        todo!()
    }
}

/// A crop region as fractions (0-1) of the captured image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A crop region in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Region {
    pub const FULL: Region = Region { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };

    /// Ok when every value is finite, x and y are in [0, 1], w and h are in (0, 1], and
    /// x + w and y + h are at most 1 (tolerance 1e-9).
    pub fn validate(&self) -> Result<(), String> {
        todo!()
    }

    /// Pixel rectangle in an image of `width` x `height`: edges are
    /// `round(x * width)` and `round((x + w) * width)` (likewise for y), clamped to the image,
    /// and the result is at least 1 x 1.
    pub fn to_pixels(&self, width: u32, height: u32) -> PixelRect {
        let _ = (width, height);
        todo!()
    }
}

/// Crop `img` to `region` (see [`Region::to_pixels`]).
pub fn crop(img: &RgbaImage, region: &Region) -> RgbaImage {
    let _ = (img, region);
    todo!()
}

/// PNG bytes of `img` scaled down to at most `max_width` wide (aspect kept, never enlarged).
pub fn thumbnail_png(img: &RgbaImage, max_width: u32) -> Vec<u8> {
    let _ = (img, max_width);
    todo!()
}
