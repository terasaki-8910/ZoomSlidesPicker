//! Frame sources for ZoomSlidesPicker (SPEC.md > キャプチャ).
//!
//! - Windows on screen, by id, through xcap 0.9.8. On macOS this uses
//!   `CGWindowListCreateImage`, which returns the window's own pixels even when other
//!   windows cover it (verified in spike/). Needs Screen Recording permission on macOS.
//! - A crop region given as fractions of the window, so it follows window resizes.
//! - Frames of a video file through the `ffmpeg` command on PATH (the mp4 mode).

use std::fmt;
use std::io::Cursor;

pub use image::RgbaImage;

pub mod mp4;
pub mod permission;

/// `app_name()` values that are system UI, not something a lecture would run in
/// (SPEC.md > キャプチャ). Includes the macOS Japanese display names for the same
/// processes, since xcap reports whatever the OS gives it.
const EXCLUDED_APPS: &[&str] = &[
    "Window Server",
    "Dock",
    "Control Center",
    "コントロールセンター",
    "Notification Center",
    "通知センター",
    "Spotlight",
    "SystemUIServer",
    "ZoomSlidesPicker",
];

/// Smallest window worth picking as a lecture source (points).
const MIN_WIDTH: u32 = 200;
const MIN_HEIGHT: u32 = 120;

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

fn to_capture_error(e: xcap::XCapError) -> CaptureError {
    CaptureError::Failed(e.to_string())
}

/// Look up a window by id via a fresh `Window::all()` (xcap has no by-id lookup).
fn find_window(id: u32) -> Result<xcap::Window, CaptureError> {
    xcap::Window::all()
        .map_err(to_capture_error)?
        .into_iter()
        .find(|w| w.id().is_ok_and(|found| found == id))
        .ok_or(CaptureError::NotFound)
}

/// Whether a window with this id is still in `Window::all()`. Used on the capture error
/// path to tell a closed window apart from a failure for another reason. If the listing
/// itself fails, assume the window is still present rather than reporting a false
/// `NotFound`.
fn window_present(id: u32) -> bool {
    xcap::Window::all()
        .map(|ws| ws.iter().any(|w| w.id().is_ok_and(|found| found == id)))
        .unwrap_or(true)
}

fn window_info(window: &xcap::Window) -> Result<WindowInfo, CaptureError> {
    Ok(WindowInfo {
        id: window.id().map_err(to_capture_error)?,
        pid: window.pid().map_err(to_capture_error)?,
        app: window.app_name().map_err(to_capture_error)?,
        title: window.title().map_err(to_capture_error)?,
        width: window.width().map_err(to_capture_error)?,
        height: window.height().map_err(to_capture_error)?,
    })
}

/// Windows the user can pick: visible (not minimized), at least 200 x 120, with a title,
/// not owned by this process, and not system UI (Window Server, Dock, Control Center,
/// Notification Center, including their Japanese names). Sorted by app name, then title.
pub fn list_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    let own_pid = std::process::id();
    let windows = xcap::Window::all().map_err(to_capture_error)?;

    let mut infos: Vec<WindowInfo> = windows
        .iter()
        .filter_map(|w| {
            let id = w.id().ok()?;
            let pid = w.pid().ok()?;
            let app = w.app_name().ok()?;
            let title = w.title().ok()?;
            let width = w.width().ok()?;
            let height = w.height().ok()?;
            let minimized = w.is_minimized().ok()?;

            let excluded = pid == own_pid
                || minimized
                || title.is_empty()
                || width < MIN_WIDTH
                || height < MIN_HEIGHT
                || EXCLUDED_APPS.contains(&app.as_str());

            if excluded {
                None
            } else {
                Some(WindowInfo {
                    id,
                    pid,
                    app,
                    title,
                    width,
                    height,
                })
            }
        })
        .collect();

    infos.sort_by(|a, b| a.app.cmp(&b.app).then_with(|| a.title.cmp(&b.title)));
    Ok(infos)
}

/// A window being captured. Keeps the xcap handle so each capture is a single call.
pub struct WindowCapture {
    info: WindowInfo,
    window: xcap::Window,
}

impl WindowCapture {
    /// Find the window by id. `NotFound` if it is gone, `Minimized` if minimized.
    pub fn open(id: u32) -> Result<Self, CaptureError> {
        let window = find_window(id)?;
        if window.is_minimized().unwrap_or_default() {
            return Err(CaptureError::Minimized);
        }
        let info = window_info(&window)?;
        Ok(WindowCapture { info, window })
    }

    pub fn info(&self) -> &WindowInfo {
        &self.info
    }

    /// Capture the whole window in pixels (Retina: 2x the window's point size).
    /// `NotFound` once the window has closed, `Minimized` while minimized.
    pub fn capture(&self) -> Result<RgbaImage, CaptureError> {
        match self.window.capture_image() {
            Ok(img) => Ok(img),
            Err(_) if !window_present(self.info.id) => Err(CaptureError::NotFound),
            Err(_) if self.window.is_minimized().unwrap_or_default() => {
                Err(CaptureError::Minimized)
            }
            Err(e) => Err(to_capture_error(e)),
        }
    }
}

// The session engine moves a `WindowCapture` into its capture thread, so it must be Send.
// xcap 0.9.8's platform handle (`ImplWindow`) is `unsafe impl Send` on both macOS and
// Windows (checked by reading the xcap source), so `xcap::Window` -- and a struct that
// just holds one alongside plain data -- is Send too. This assertion turns a future
// regression of that into a compile error instead of a runtime one.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<WindowCapture>();
};

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
    pub const FULL: Region = Region {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };

    /// Ok when every value is finite, x and y are in [0, 1], w and h are in (0, 1], and
    /// x + w and y + h are at most 1 (tolerance 1e-9).
    pub fn validate(&self) -> Result<(), String> {
        const TOLERANCE: f64 = 1e-9;

        if ![self.x, self.y, self.w, self.h]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(format!("region values must be finite: {self:?}"));
        }
        if !(0.0..=1.0).contains(&self.x) {
            return Err(format!("x out of range [0, 1]: {}", self.x));
        }
        if !(0.0..=1.0).contains(&self.y) {
            return Err(format!("y out of range [0, 1]: {}", self.y));
        }
        if !(self.w > 0.0 && self.w <= 1.0) {
            return Err(format!("w out of range (0, 1]: {}", self.w));
        }
        if !(self.h > 0.0 && self.h <= 1.0) {
            return Err(format!("h out of range (0, 1]: {}", self.h));
        }
        if self.x + self.w > 1.0 + TOLERANCE {
            return Err(format!("x + w exceeds 1: {}", self.x + self.w));
        }
        if self.y + self.h > 1.0 + TOLERANCE {
            return Err(format!("y + h exceeds 1: {}", self.y + self.h));
        }
        Ok(())
    }

    /// Pixel rectangle in an image of `width` x `height`: edges are
    /// `round(x * width)` and `round((x + w) * width)` (likewise for y), clamped to the image,
    /// and the result is at least 1 x 1.
    pub fn to_pixels(&self, width: u32, height: u32) -> PixelRect {
        let (x, x_end) = edge_span(self.x, self.x + self.w, width);
        let (y, y_end) = edge_span(self.y, self.y + self.h, height);
        PixelRect {
            x,
            y,
            w: x_end - x,
            h: y_end - y,
        }
    }
}

/// One axis of [`Region::to_pixels`]: round both edges independently and clamp to
/// `[0, size]`, then, if that left a span shorter than 1 px, pull the start back so
/// `[a, a + 1)` still fits in `[0, size]`.
fn edge_span(start: f64, end: f64, size: u32) -> (u32, u32) {
    let size_i = i64::from(size);
    let edge = |v: f64| ((v * f64::from(size)).round() as i64).clamp(0, size_i);

    let mut a = edge(start);
    let mut b = edge(end);
    if b - a < 1 {
        a = a.min(size_i.saturating_sub(1)).max(0);
        b = (a + 1).min(size_i);
    }
    (a as u32, b as u32)
}

/// Crop `img` to `region` (see [`Region::to_pixels`]).
pub fn crop(img: &RgbaImage, region: &Region) -> RgbaImage {
    let (width, height) = img.dimensions();
    let rect = region.to_pixels(width, height);
    image::imageops::crop_imm(img, rect.x, rect.y, rect.w, rect.h).to_image()
}

/// PNG bytes of `img` scaled down to at most `max_width` wide (aspect kept, never enlarged).
pub fn thumbnail_png(img: &RgbaImage, max_width: u32) -> Vec<u8> {
    let (width, height) = img.dimensions();
    let target_w = max_width.min(width).max(1);
    let target_h =
        ((u64::from(height) * u64::from(target_w)) / u64::from(width.max(1))).max(1) as u32;

    let scaled = image::imageops::resize(
        img,
        target_w,
        target_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut png = Vec::new();
    scaled
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("encoding a PNG to an in-memory buffer does not fail");
    png
}
