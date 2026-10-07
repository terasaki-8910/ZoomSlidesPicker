//! Session output for ZoomSlidesPicker (SPEC.md > 出力).
//!
//! One folder per session under the save root:
//! - `NNN_hh-mm-ss.png`: each capture, lossless, at the captured size
//! - `session.json`: the record, rewritten atomically (temp file + rename) after every change,
//!   so a crash leaves a valid file; shape in `docs/session.schema.json`
//! - `slides.pdf`: one page per capture in order, page size = image size (1 px = 1 pt),
//!   images embedded losslessly (8-bit RGB, Flate)

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, SecondsFormat};
use image::RgbImage;
use miniz_oxide::deflate::{CompressionLevel, compress_to_vec_zlib};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use serde::{Deserialize, Serialize};

pub const SESSION_FILE: &str = "session.json";
pub const PDF_FILE: &str = "slides.pdf";
pub const META_FILE: &str = "meta.json";

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
    /// A capture was replaced by a version that only adds to it (`index` says which).
    Replaced,
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
    /// The capture an event is about (`replaced` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
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
    started.format("%Y-%m-%d_%H%M").to_string()
}

/// Create the session folder under `root` (creating `root` if needed). When the name is
/// taken, append `-2`, `-3`, ... Returns the created folder.
pub fn create_session_dir(root: &Path, started: &DateTime<Local>) -> io::Result<PathBuf> {
    fs::create_dir_all(root)?;
    let base = session_dir_name(started);
    let mut candidate = root.join(&base);
    let mut suffix = 2;
    loop {
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                candidate = root.join(format!("{base}-{suffix}"));
                suffix += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// `NNN_hh-mm-ss.png`: index zero-padded to 3 digits (more digits from 1000), and
/// `t_content` (seconds since the session start) floored to whole seconds as hours,
/// minutes, seconds, each two digits. Example: (12, 3725.4) -> "012_01-02-05.png".
pub fn capture_file_name(index: usize, t_content: f64) -> String {
    let total_seconds = t_content.floor() as u64;
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    let s = total_seconds % 60;
    format!("{index:03}_{h:02}-{m:02}-{s:02}.png")
}

/// Where undone captures go. The app uses [`OsTrash`]; tests use a folder.
pub trait Trash {
    fn trash(&self, path: &Path) -> io::Result<()>;
}

/// Moves files to the operating system's trash (recoverable, not deleted).
pub struct OsTrash;

impl Trash for OsTrash {
    fn trash(&self, path: &Path) -> io::Result<()> {
        trash::delete(path).map_err(io::Error::other)
    }
}

/// Drops the alpha channel of an RGBA8 buffer, keeping pixel order.
fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect()
}

/// Writes `data` to `path` atomically: a temp file next to `path`, then a rename over it.
/// Leaves no temp file behind on success.
fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut tmp_name = path
        .file_name()
        .expect("path has a file name")
        .to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

/// Rewrites `dir`/session.json atomically from `file`.
fn write_session_json(dir: &Path, file: &SessionFile) -> io::Result<()> {
    let data = serde_json::to_string_pretty(file).map_err(io::Error::other)?;
    write_atomic(&dir.join(SESSION_FILE), data.as_bytes())
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
        let dir = create_session_dir(root, &started)?;
        let file = SessionFile {
            schema: 1,
            app_version: meta.app_version,
            started_at: started.to_rfc3339_opts(SecondsFormat::Secs, false),
            ended_at: None,
            source: meta.source,
            region: meta.region,
            settings: Settings {
                interval_s: meta.interval_s,
            },
            captures: Vec::new(),
            revisits: Vec::new(),
            events: Vec::new(),
            pdf: None,
        };
        write_session_json(&dir, &file)?;
        Ok(Self { dir, file })
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
        let name = capture_file_name(index, t_content);
        let path = self.dir.join(&name);

        let rgb = rgba_to_rgb(rgba);
        let image = RgbImage::from_raw(width, height, rgb).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "rgba buffer does not match width * height",
            )
        })?;
        image.save(&path).map_err(io::Error::other)?;

        self.file.captures.push(CaptureRecord {
            index,
            file: name,
            t_content,
            t_saved,
            reason,
            width,
            height,
        });
        write_session_json(&self.dir, &self.file)?;
        Ok(path)
    }

    pub fn add_revisit(&mut self, t: f64, matches: usize) -> io::Result<()> {
        self.file.revisits.push(RevisitRecord { t, matches });
        write_session_json(&self.dir, &self.file)
    }

    pub fn add_event(&mut self, t: f64, kind: EventKind) -> io::Result<()> {
        self.file.events.push(EventRecord {
            t,
            kind,
            index: None,
        });
        write_session_json(&self.dir, &self.file)
    }

    /// Replace capture `index` with a new version (R3-2): write the frame as
    /// `capture_file_name(index, t_content)`, move the old PNG to `trash`, update the record
    /// in place (same index and position), and add a `replaced` event at `t_saved`. The new
    /// file is written before the old one is moved, unless both have the same name.
    /// Returns the new PNG path.
    #[allow(clippy::too_many_arguments)]
    pub fn replace_capture(
        &mut self,
        index: usize,
        rgba: &[u8],
        width: u32,
        height: u32,
        t_content: f64,
        t_saved: f64,
        reason: CaptureReason,
        trash: &dyn Trash,
    ) -> io::Result<PathBuf> {
        let pos = self
            .file
            .captures
            .iter()
            .position(|c| c.index == index)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("no capture {index}"))
            })?;
        let old_path = self.dir.join(&self.file.captures[pos].file);
        let name = capture_file_name(index, t_content);
        let path = self.dir.join(&name);
        if path == old_path {
            trash.trash(&old_path)?;
        }
        let rgb = rgba_to_rgb(rgba);
        let image = RgbImage::from_raw(width, height, rgb).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "rgba buffer does not match width * height",
            )
        })?;
        image.save(&path).map_err(io::Error::other)?;
        if path != old_path {
            trash.trash(&old_path)?;
        }
        self.file.captures[pos] = CaptureRecord {
            index,
            file: name,
            t_content,
            t_saved,
            reason,
            width,
            height,
        };
        self.file.events.push(EventRecord {
            t: t_saved,
            kind: EventKind::Replaced,
            index: Some(index),
        });
        write_session_json(&self.dir, &self.file)?;
        Ok(path)
    }

    /// Move the most recent capture's PNG to `trash`, drop its record, and add an `undo`
    /// event at `t`. Returns the moved file's original path, or None when there is none.
    pub fn undo_last(&mut self, t: f64, trash: &dyn Trash) -> io::Result<Option<PathBuf>> {
        let Some(last) = self.file.captures.last() else {
            return Ok(None);
        };
        let path = self.dir.join(&last.file);
        trash.trash(&path)?;

        self.file.captures.pop();
        self.file.events.push(EventRecord {
            t,
            kind: EventKind::Undo,
            index: None,
        });
        write_session_json(&self.dir, &self.file)?;
        Ok(Some(path))
    }

    /// End the session: write slides.pdf when there is at least one capture, set `ended_at`
    /// and `pdf`, and rewrite session.json. Returns the PDF path when one was written.
    pub fn finish(&mut self, ended_at: DateTime<Local>) -> io::Result<Option<PathBuf>> {
        self.file.ended_at = Some(ended_at.to_rfc3339_opts(SecondsFormat::Secs, false));

        let pdf_path = if self.file.captures.is_empty() {
            self.file.pdf = None;
            None
        } else {
            let images: Vec<PathBuf> = self
                .file
                .captures
                .iter()
                .map(|c| self.dir.join(&c.file))
                .collect();
            let path = self.dir.join(PDF_FILE);
            write_pdf(&images, &path)?;
            self.file.pdf = Some(PdfRecord {
                file: PDF_FILE.to_string(),
                pages: images.len(),
                rebuilt_at: None,
            });
            Some(path)
        };

        write_session_json(&self.dir, &self.file)?;
        Ok(pdf_path)
    }
}

/// Write a PDF with one page per image, in order. Page size equals the image size in
/// pixels, as points; each page carries its own `MediaBox [0 0 w h]`. Each page draws one
/// image XObject (8-bit `DeviceRGB`, `FlateDecode`, lossless) over the whole page.
pub fn write_pdf(images: &[PathBuf], out: &Path) -> io::Result<()> {
    if images.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no images to write a PDF from",
        ));
    }

    let mut pdf = Pdf::new();
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    // Three indirect objects per page: the page itself, its content stream, and its image.
    let page_ids: Vec<Ref> = (0..images.len())
        .map(|i| Ref::new(3 + 3 * i as i32))
        .collect();

    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id)
        .kids(page_ids.iter().copied())
        .count(page_ids.len() as i32);

    let image_name = Name(b"Im0");
    for (path, &page_id) in images.iter().zip(&page_ids) {
        let content_id = Ref::new(page_id.get() + 1);
        let image_id = Ref::new(page_id.get() + 2);

        let rgb = image::open(path).map_err(io::Error::other)?.into_rgb8();
        let (width, height) = rgb.dimensions();

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, width as f32, height as f32));
        page.parent(page_tree_id);
        page.contents(content_id);
        page.resources().x_objects().pair(image_name, image_id);
        page.finish();

        let encoded = compress_to_vec_zlib(rgb.as_raw(), CompressionLevel::DefaultLevel as u8);
        let mut xobject = pdf.image_xobject(image_id, &encoded);
        xobject.filter(Filter::FlateDecode);
        xobject.width(width as i32);
        xobject.height(height as i32);
        xobject.color_space().device_rgb();
        xobject.bits_per_component(8);
        xobject.finish();

        // Map the image's unit square onto the whole page: q w 0 0 h 0 0 cm /Im0 Do Q.
        let mut content = Content::new();
        content.save_state();
        content.transform([width as f32, 0.0, 0.0, height as f32, 0.0, 0.0]);
        content.x_object(image_name);
        content.restore_state();
        pdf.stream(content_id, &content.finish());
    }

    write_atomic(out, &pdf.finish())
}

/// Parses a capture file name matching `^[0-9]{3,}_[0-9]{2,}-[0-9]{2}-[0-9]{2}\.png$`
/// (the `session.schema.json` pattern for `captures[].file`), returning its numeric index.
fn capture_index(file_name: &str) -> Option<usize> {
    let is_digits = |s: &str, min_len: usize| {
        s.len() >= min_len && !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
    };

    let stem = file_name.strip_suffix(".png")?;
    let (index, time) = stem.split_once('_')?;
    if !is_digits(index, 3) {
        return None;
    }

    let mut parts = time.split('-');
    let (h, m, s) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some()
        || !is_digits(h, 2)
        || m.len() != 2
        || !is_digits(m, 2)
        || s.len() != 2
        || !is_digits(s, 2)
    {
        return None;
    }

    index.parse().ok()
}

/// Rebuild slides.pdf in a finished session folder from the `NNN_*.png` files still there,
/// in index order (R2-3). Updates session.json: `pdf.pages` and `pdf.rebuilt_at` (`now`);
/// `captures` is left unchanged. Returns the page count. With no images left, writes
/// nothing and returns an `InvalidInput` error.
pub fn rebuild_pdf(dir: &Path, now: DateTime<Local>) -> io::Result<usize> {
    let mut found: Vec<(usize, PathBuf)> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let index = capture_index(entry.file_name().to_str()?)?;
            Some((index, entry.path()))
        })
        .collect();
    found.sort_by_key(|(index, _)| *index);

    if found.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no capture images left to rebuild the PDF from",
        ));
    }

    let images: Vec<PathBuf> = found.into_iter().map(|(_, path)| path).collect();
    let pages = images.len();
    write_pdf(&images, &dir.join(PDF_FILE))?;

    let session_path = dir.join(SESSION_FILE);
    let mut file: SessionFile =
        serde_json::from_str(&fs::read_to_string(&session_path)?).map_err(io::Error::other)?;
    file.pdf = Some(PdfRecord {
        file: PDF_FILE.to_string(),
        pages,
        rebuilt_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, false)),
    });
    write_session_json(dir, &file)?;

    Ok(pages)
}

/// Reads and parses `dir`/session.json (the history window's `list_sessions`/`get_session`;
/// docs/impl/library.md > Rust side).
pub fn read_session(dir: &Path) -> io::Result<SessionFile> {
    let text = fs::read_to_string(dir.join(SESSION_FILE))?;
    serde_json::from_str(&text).map_err(io::Error::other)
}

/// Session metadata (`meta.json`): a custom title and a memo, kept separate from
/// session.json so a rename or memo save never races the session writer, which rewrites
/// session.json after every capture (docs/impl/library.md > Rust side).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionMetaFile {
    pub title: Option<String>,
    #[serde(default)]
    pub memo: String,
}

impl SessionMetaFile {
    /// `title` trimmed; empty or all-whitespace becomes `None` (ACCEPTANCE B11).
    pub fn new(title: Option<&str>, memo: impl Into<String>) -> Self {
        let title = title
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string);
        Self {
            title,
            memo: memo.into(),
        }
    }
}

/// Reads `dir`/meta.json, defaulting (title `None`, memo `""`) when it is missing,
/// unreadable, or fails to parse -- this never errors (ACCEPTANCE B11).
pub fn read_meta(dir: &Path) -> SessionMetaFile {
    fs::read_to_string(dir.join(META_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Writes `dir`/meta.json atomically (temp file + rename).
pub fn write_meta(dir: &Path, meta: &SessionMetaFile) -> io::Result<()> {
    let data = serde_json::to_string_pretty(meta).map_err(io::Error::other)?;
    write_atomic(&dir.join(META_FILE), data.as_bytes())
}

/// Moves the PNGs at `indices` to `trash`, removes their records from session.json,
/// rebuilds slides.pdf from the images still on disk (or removes it when none is left), and
/// updates `pdf` (`pages` and `rebuilt_at`, or `None`) -- the history window's
/// `trash_captures` command (docs/impl/library.md). `captures` keeps every other record, in
/// their original order. A chosen PNG that is already gone counts as moved. Errors, and
/// changes nothing, when `indices` names one that doesn't exist (ACCEPTANCE B12).
/// session.json is rewritten as soon as the files are moved, so a failure while writing the
/// PDF never leaves records pointing at files in the trash.
pub fn trash_captures(
    dir: &Path,
    indices: &[usize],
    trash: &dyn Trash,
    now: DateTime<Local>,
) -> io::Result<usize> {
    if indices.is_empty() {
        return Ok(0);
    }

    let session_path = dir.join(SESSION_FILE);
    let mut file: SessionFile =
        serde_json::from_str(&fs::read_to_string(&session_path)?).map_err(io::Error::other)?;

    let wanted: BTreeSet<usize> = indices.iter().copied().collect();
    for &index in &wanted {
        if !file.captures.iter().any(|c| c.index == index) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no capture {index}"),
            ));
        }
    }

    let (removed, kept): (Vec<CaptureRecord>, Vec<CaptureRecord>) = file
        .captures
        .into_iter()
        .partition(|rec| wanted.contains(&rec.index));
    file.captures = kept;
    for rec in &removed {
        let path = dir.join(&rec.file);
        if path.exists() {
            trash.trash(&path)?;
        }
    }
    write_session_json(dir, &file)?;

    let images: Vec<PathBuf> = file
        .captures
        .iter()
        .map(|c| dir.join(&c.file))
        .filter(|p| p.is_file())
        .collect();
    let pdf_path = dir.join(PDF_FILE);
    if images.is_empty() {
        if pdf_path.exists() {
            fs::remove_file(&pdf_path)?;
        }
        file.pdf = None;
    } else {
        write_pdf(&images, &pdf_path)?;
        file.pdf = Some(PdfRecord {
            file: PDF_FILE.to_string(),
            pages: images.len(),
            rebuilt_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, false)),
        });
    }
    write_session_json(dir, &file)?;
    Ok(removed.len())
}
