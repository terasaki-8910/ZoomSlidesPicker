//! Session output for ZoomSlidesPicker (SPEC.md > 出力).
//!
//! One folder per session under the save root:
//! - `NNN_hh-mm-ss.png`: each capture, lossless, at the captured size
//! - `session.json`: the record, rewritten atomically (temp file + rename) after every change,
//!   so a crash leaves a valid file; shape in `docs/session.schema.json`
//! - `slides.pdf`: one page per capture in order, page size = image size (1 px = 1 pt),
//!   images embedded losslessly (8-bit RGB, Flate)

use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

pub const SESSION_FILE: &str = "session.json";
pub const PDF_FILE: &str = "slides.pdf";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureReason {
    Settled,
    Manual,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Window,
    Mp4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Paused,
    Resumed,
    Blank,
    WindowLost,
    WindowBack,
    Undo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub kind: SourceKind,
    pub app: Option<String>,
    pub title: Option<String>,
    /// "macos", "windows", or "linux".
    pub platform: String,
    /// The video file, for kind Mp4.
    pub file: Option<String>,
}

/// Capture region as fractions of the window (0-1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegionRecord {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub interval_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptureRecord {
    pub index: usize,
    pub file: String,
    pub t_content: f64,
    pub t_saved: f64,
    pub reason: CaptureReason,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RevisitRecord {
    pub t: f64,
    pub matches: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub t: f64,
    #[serde(rename = "type")]
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PdfRecord {
    pub file: String,
    pub pages: usize,
    /// RFC 3339 local time of the last rebuild; None until rebuilt.
    pub rebuilt_at: Option<String>,
}

/// The contents of session.json (schema version 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionFile {
    pub schema: u32,
    pub app_version: String,
    /// RFC 3339 with the local offset, seconds precision, e.g. "2026-10-06T20:14:05+09:00".
    pub started_at: String,
    pub ended_at: Option<String>,
    pub source: Source,
    pub region: Option<RegionRecord>,
    pub settings: Settings,
    pub captures: Vec<CaptureRecord>,
    pub revisits: Vec<RevisitRecord>,
    pub events: Vec<EventRecord>,
    /// None until slides.pdf is written (and when it could not be written).
    pub pdf: Option<PdfRecord>,
}

/// What the caller knows when a session starts.
#[derive(Clone, Debug)]
pub struct SessionMeta {
    pub app_version: String,
    pub source: Source,
    pub region: Option<RegionRecord>,
    pub interval_s: f64,
}

/// Folder name for a session started at `started`: `YYYY-MM-DD_HHMM` (local time).
pub fn session_dir_name(started: &DateTime<Local>) -> String {
    let _ = started;
    todo!()
}

/// Create the session folder under `root` (creating `root` if needed). When the name is
/// taken, append `-2`, `-3`, ... Returns the created folder.
pub fn create_session_dir(root: &Path, started: &DateTime<Local>) -> io::Result<PathBuf> {
    let _ = (root, started);
    todo!()
}

/// `NNN_hh-mm-ss.png`: index zero-padded to 3 digits (more digits from 1000), and
/// `t_content` (seconds since the session start) floored to whole seconds as hours,
/// minutes, seconds, each two digits. Example: (12, 3725.4) -> "012_01-02-05.png".
pub fn capture_file_name(index: usize, t_content: f64) -> String {
    let _ = (index, t_content);
    todo!()
}

/// Where undone captures go. The app uses [`OsTrash`]; tests use a folder.
pub trait Trash {
    fn trash(&self, path: &Path) -> io::Result<()>;
}

/// Moves files to the operating system's trash (recoverable, not deleted).
pub struct OsTrash;

impl Trash for OsTrash {
    fn trash(&self, path: &Path) -> io::Result<()> {
        let _ = path;
        todo!()
    }
}

/// Writes one session's folder. Every mutating call rewrites session.json atomically.
pub struct SessionWriter {
    dir: PathBuf,
    file: SessionFile,
}

impl SessionWriter {
    /// Create the session folder and write the initial session.json
    /// (`ended_at` null, `pdf` null, empty lists).
    pub fn create(root: &Path, started: DateTime<Local>, meta: SessionMeta) -> io::Result<Self> {
        let _ = (root, started, meta);
        todo!()
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn session(&self) -> &SessionFile {
        &self.file
    }

    pub fn capture_count(&self) -> usize {
        self.file.captures.len()
    }

    /// Write an RGBA8 frame as `capture_file_name(index, t_content)` (PNG, RGB, lossless)
    /// and append its record. Returns the PNG path.
    #[allow(clippy::too_many_arguments)]
    pub fn save_capture(
        &mut self,
        index: usize,
        rgba: &[u8],
        width: u32,
        height: u32,
        t_content: f64,
        t_saved: f64,
        reason: CaptureReason,
    ) -> io::Result<PathBuf> {
        let _ = (index, rgba, width, height, t_content, t_saved, reason);
        todo!()
    }

    pub fn add_revisit(&mut self, t: f64, matches: usize) -> io::Result<()> {
        let _ = (t, matches);
        todo!()
    }

    pub fn add_event(&mut self, t: f64, kind: EventKind) -> io::Result<()> {
        let _ = (t, kind);
        todo!()
    }

    /// Move the most recent capture's PNG to `trash`, drop its record, and add an `undo`
    /// event at `t`. Returns the moved file's original path, or None when there is none.
    pub fn undo_last(&mut self, t: f64, trash: &dyn Trash) -> io::Result<Option<PathBuf>> {
        let _ = (t, trash);
        todo!()
    }

    /// End the session: write slides.pdf when there is at least one capture, set `ended_at`
    /// and `pdf`, and rewrite session.json. Returns the PDF path when one was written.
    pub fn finish(&mut self, ended_at: DateTime<Local>) -> io::Result<Option<PathBuf>> {
        let _ = ended_at;
        todo!()
    }
}

/// Write a PDF with one page per image, in order. Page size equals the image size in
/// pixels, as points; each page carries its own `MediaBox [0 0 w h]`. Each page draws one
/// image XObject (8-bit `DeviceRGB`, `FlateDecode`, lossless) over the whole page.
pub fn write_pdf(images: &[PathBuf], out: &Path) -> io::Result<()> {
    let _ = (images, out);
    todo!()
}

/// Rebuild slides.pdf in a finished session folder from the `NNN_*.png` files still there,
/// in index order (R2-3). Updates session.json: `pdf.pages` and `pdf.rebuilt_at` (`now`);
/// `captures` is left unchanged. Returns the page count. With no images left, writes
/// nothing and returns an `InvalidInput` error.
pub fn rebuild_pdf(dir: &Path, now: DateTime<Local>) -> io::Result<usize> {
    let _ = (dir, now);
    todo!()
}
