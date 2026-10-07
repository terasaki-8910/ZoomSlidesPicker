//! The history window ("library"): its commands, the scan over save roots, the
//! `zspthumb://` thumbnail protocol, and the window itself (docs/impl/library.md).
//!
//! Every command here is `async` (docs/impl/library.md > Commands: they read folders,
//! decode images, or open the OS shell, so they run off the main thread) and takes a
//! `dir` naming a session folder as an absolute path string, validated by
//! [`resolve_session_dir`] against the known roots (docs/impl/library.md > Roots).

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;

use chrono::DateTime;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;
use zsp_output::Trash as _;

use crate::i18n;
use crate::ipc_types::{self, Lang};
use crate::settings;
use crate::state::SharedShell;

pub const LIBRARY_LABEL: &str = "library";
const LIBRARY_WIDTH: f64 = 1100.0;
const LIBRARY_HEIGHT: f64 = 720.0;
const LIBRARY_MIN_WIDTH: f64 = 760.0;
const LIBRARY_MIN_HEIGHT: f64 = 520.0;

const LIBRARY_CHANGED_EVENT: &str = "library-changed";

const THUMB_SCHEME: &str = "zspthumb";
const THUMB_DEFAULT_WIDTH: u32 = 360;
const THUMB_MIN_WIDTH: u32 = 64;
const THUMB_MAX_WIDTH: u32 = 800;
const THUMB_QUALITY: u8 = 80;

// ---------------------------------------------------------------------------------------
// Wire types (docs/impl/library.md > Types; serde camelCase, kept in sync with
// src/ipc/types.ts by hand, as every other type in ipc_types.rs is).
// ---------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// Absolute session folder: the identifier.
    pub dir: String,
    /// Folder name, e.g. "2026-10-07_1242".
    pub folder: String,
    /// `customTitle`, else `sourceTitle`.
    pub title: String,
    pub custom_title: Option<String>,
    /// `""` when none.
    pub memo: String,
    /// e.g. "Safari"; `None` for mp4.
    pub source_app: Option<String>,
    /// Window title, or video file name.
    pub source_title: String,
    pub source_kind: ipc_types::SourceKind,
    /// RFC 3339, from session.json.
    pub started_at: String,
    /// `None` while recording (or after a crash).
    pub ended_at: Option<String>,
    /// Captures whose PNG exists.
    pub count: usize,
    /// slides.pdf exists.
    pub has_pdf: bool,
    /// The folder of the running or paused session.
    pub recording: bool,
    /// The first existing capture PNG, `None` when there is none.
    pub thumb_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureView {
    pub index: usize,
    /// e.g. "003_00-04-12.png".
    pub file: String,
    /// Absolute path of the PNG.
    pub path: String,
    pub t_content: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    #[serde(flatten)]
    pub summary: SessionSummary,
    /// In index order, only those whose PNG exists.
    pub captures: Vec<CaptureView>,
}

fn to_ipc_source_kind(kind: zsp_output::SourceKind) -> ipc_types::SourceKind {
    match kind {
        zsp_output::SourceKind::Window => ipc_types::SourceKind::Window,
        zsp_output::SourceKind::Mp4 => ipc_types::SourceKind::Mp4,
    }
}

// ---------------------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------------------

/// Emits `library-changed` (no payload): after anything that changes what `list_sessions`
/// or `get_session` returns (docs/impl/library.md > Commands). The window refetches on
/// it; coalescing bursts is the window's job, not ours.
pub fn notify_changed<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(LIBRARY_CHANGED_EVENT, ());
}

// ---------------------------------------------------------------------------------------
// Roots (docs/impl/library.md > Roots)
// ---------------------------------------------------------------------------------------

/// The roots to scan and validate paths against: the current save root (reflecting any
/// `ZSP_SAVE_ROOT` override, which is written into `settings.save_root` at startup) plus
/// every root used before.
fn known_roots<R: Runtime>(app: &AppHandle<R>) -> Vec<PathBuf> {
    let shell = app.state::<SharedShell>();
    let shell = shell.lock().unwrap();
    std::iter::once(PathBuf::from(&shell.settings.save_root))
        .chain(shell.known_roots.iter().map(PathBuf::from))
        .collect()
}

fn active_session_dir<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.state::<SharedShell>()
        .lock()
        .unwrap()
        .session
        .as_ref()
        .map(|a| a.dir.clone())
}

fn is_recording<R: Runtime>(app: &AppHandle<R>, dir: &Path) -> bool {
    active_session_dir(app).as_deref() == Some(dir)
}

fn current_lang<R: Runtime>(app: &AppHandle<R>) -> Lang {
    let shell = app.state::<SharedShell>();
    let language = shell.lock().unwrap().settings.language;
    settings::effective_language(language)
}

/// `path`, canonicalized, when it is a directory that is a direct child of one of `roots`
/// (each canonicalized too, so `..` and symlinks cannot fake membership). `None` when
/// `path` doesn't exist, isn't a directory, or isn't directly inside any root.
fn canonical_child_dir(path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    let canonical = fs::canonicalize(path).ok()?;
    if !canonical.is_dir() {
        return None;
    }
    let parent = canonical.parent()?;
    roots
        .iter()
        .any(|root| fs::canonicalize(root).map(|r| r == parent).unwrap_or(false))
        .then_some(canonical)
}

/// A capture PNG directly inside a session folder of one of `roots`, canonicalized
/// (docs/impl/library.md > zspthumb protocol, `open_capture`; ACCEPTANCE B13). Rejects
/// anything outside a root, `..` escapes, non-PNG files, and a session folder itself.
fn validate_capture_path(path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    if path.extension().and_then(|e| e.to_str()) != Some("png") {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    if !canonical.is_file() {
        return None;
    }
    let session_dir = canonical.parent()?;
    canonical_child_dir(session_dir, roots)?;
    Some(canonical)
}

/// Resolves a `dir` command argument: a folder directly inside a known root with a
/// parsable session.json (docs/impl/library.md > Commands). The one error shared by every
/// library command that takes `dir`.
///
/// Returns `dir` itself (not its canonical form): `dir` always comes back from `list_sessions`
/// unchanged, so this keeps every command's notion of "this session's folder" byte-for-byte
/// the same as the scan's (in particular, the same as `ActiveSession.dir`, which
/// [`is_recording`] compares against) -- canonicalizing is only needed for the membership
/// check itself, not for the identity used afterwards.
fn resolve_session_dir<R: Runtime>(
    app: &AppHandle<R>,
    dir: &str,
    lang: Lang,
) -> Result<PathBuf, String> {
    let original = Path::new(dir).to_path_buf();
    canonical_child_dir(&original, &known_roots(app))
        .filter(|_| zsp_output::read_session(&original).is_ok())
        .map(|_| original)
        .ok_or_else(|| i18n::library_not_found(lang).to_string())
}

// ---------------------------------------------------------------------------------------
// Scan and summary building (pure: no app handle, no locks -- ACCEPTANCE B13)
// ---------------------------------------------------------------------------------------

/// Builds one session's summary from its folder, or `None` when it has no parsable
/// session.json (docs/impl/library.md > list_sessions: unreadable folders and folders
/// without one are skipped). `active_dir` is the folder of the session currently running
/// or paused, if any -- the only way to tell "still recording" apart from `endedAt` being
/// null because of a crash.
fn build_summary(dir: &Path, active_dir: Option<&Path>) -> Option<SessionSummary> {
    let session = zsp_output::read_session(dir).ok()?;
    let meta = zsp_output::read_meta(dir);

    let mut existing: Vec<&zsp_output::CaptureRecord> = session
        .captures
        .iter()
        .filter(|c| dir.join(&c.file).is_file())
        .collect();
    existing.sort_by_key(|c| c.index);

    let source_title = match session.source.kind {
        zsp_output::SourceKind::Window => session.source.title.clone().unwrap_or_default(),
        zsp_output::SourceKind::Mp4 => session.source.file.clone().unwrap_or_default(),
    };
    let custom_title = meta.title.clone();
    let title = custom_title.clone().unwrap_or_else(|| source_title.clone());

    Some(SessionSummary {
        dir: ipc_types::path_to_string(dir),
        folder: dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        title,
        custom_title,
        memo: meta.memo,
        source_app: session.source.app.clone(),
        source_title,
        source_kind: to_ipc_source_kind(session.source.kind),
        started_at: session.started_at.clone(),
        ended_at: session.ended_at.clone(),
        count: existing.len(),
        has_pdf: dir.join(zsp_output::PDF_FILE).is_file(),
        recording: active_dir == Some(dir),
        thumb_path: existing
            .first()
            .map(|c| ipc_types::path_to_string(&dir.join(&c.file))),
    })
}

fn build_detail(dir: &Path, active_dir: Option<&Path>) -> Option<SessionDetail> {
    let summary = build_summary(dir, active_dir)?;
    let session = zsp_output::read_session(dir).ok()?;
    let mut captures: Vec<CaptureView> = session
        .captures
        .iter()
        .filter(|c| dir.join(&c.file).is_file())
        .map(|c| CaptureView {
            index: c.index,
            file: c.file.clone(),
            path: ipc_types::path_to_string(&dir.join(&c.file)),
            t_content: c.t_content,
        })
        .collect();
    captures.sort_by_key(|c| c.index);
    Some(SessionDetail { summary, captures })
}

/// Every session folder directly inside `roots`, newest `startedAt` first
/// (docs/impl/library.md > list_sessions). Folders without a parsable session.json, ones
/// that fail to read, and roots that don't exist, are all skipped.
fn scan_sessions(roots: &[PathBuf], active_dir: Option<&Path>) -> Vec<SessionSummary> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir())
                && let Some(summary) = build_summary(&entry.path(), active_dir)
            {
                out.push(summary);
            }
        }
    }
    out.sort_by(|a, b| {
        match (
            DateTime::parse_from_rfc3339(&a.started_at),
            DateTime::parse_from_rfc3339(&b.started_at),
        ) {
            (Ok(ta), Ok(tb)) => tb.cmp(&ta),
            _ => b.started_at.cmp(&a.started_at),
        }
    });
    out
}

/// Dry run of `zsp_detect::fold_captures` over `dir`'s existing capture PNGs, in index
/// order; returns the `CaptureRecord.index` values it would not keep
/// (docs/impl/library.md > fold_session).
fn fold_dry_run(dir: &Path) -> io::Result<Vec<usize>> {
    let session = zsp_output::read_session(dir)?;
    let mut existing: Vec<&zsp_output::CaptureRecord> = session
        .captures
        .iter()
        .filter(|c| dir.join(&c.file).is_file())
        .collect();
    existing.sort_by_key(|c| c.index);

    let mut analyzed = Vec::with_capacity(existing.len());
    for c in &existing {
        let img = image::open(dir.join(&c.file))
            .map_err(io::Error::other)?
            .into_rgba8();
        let (w, h) = img.dimensions();
        analyzed.push(zsp_detect::analyze(img.as_raw(), w, h));
    }

    let keep = zsp_detect::fold_captures(analyzed);
    Ok(existing
        .iter()
        .zip(keep)
        .filter(|(_, k)| !k)
        .map(|(c, _)| c.index)
        .collect())
}

// ---------------------------------------------------------------------------------------
// Window (docs/impl/library.md > Window)
// ---------------------------------------------------------------------------------------

/// Opens the library window, creating it on demand, or focuses it if already open: label
/// `library`, 1100 x 720, minimum 760 x 520, resizable, centered, normal decorations
/// (docs/impl/library.md > Window). macOS: `ActivationPolicy::Regular` while it exists,
/// set before the window is shown; back to `Accessory` once it is destroyed.
pub fn open_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(LIBRARY_LABEL) {
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        let _ = w.set_focus();
        return;
    }

    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    let Ok(window) = WebviewWindowBuilder::new(
        app,
        LIBRARY_LABEL,
        WebviewUrl::App("index.html#/library".into()),
    )
    .title(i18n::title_panel()) // "ZoomSlidesPicker", same fixed brand name in every language
    .inner_size(LIBRARY_WIDTH, LIBRARY_HEIGHT)
    .min_inner_size(LIBRARY_MIN_WIDTH, LIBRARY_MIN_HEIGHT)
    .resizable(true)
    .center()
    .visible(false)
    .build() else {
        return;
    };

    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
        });
    }

    let _ = window.show();
    let _ = window.set_focus();
}

// ---------------------------------------------------------------------------------------
// zspthumb protocol (docs/impl/library.md > zspthumb protocol; ACCEPTANCE B13)
// ---------------------------------------------------------------------------------------

/// Registers the `zspthumb://` protocol on `builder`. The request path is a capture PNG
/// path (URL-decoded); only files that end in `.png` and lie directly in a session folder
/// of a known root are served (else 404). Decoding and resizing run off the main thread.
pub fn register_protocol<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol(THUMB_SCHEME, |ctx, request, responder| {
        let app = ctx.app_handle().clone();
        thread::spawn(move || {
            responder.respond(handle_thumb_request(&app, &request));
        });
    })
}

fn not_found_response() -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(tauri::http::StatusCode::NOT_FOUND)
        .body(Vec::new())
        .unwrap()
}

fn handle_thumb_request<R: Runtime>(
    app: &AppHandle<R>,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    // Skip the leading `/`, as tauri's own asset protocol does, then percent-decode: the
    // frontend's `thumbUrl` builds the request from `encodeURIComponent(path)`, so a path
    // separator in the real path survives as a decoded `/` here.
    let raw = request.uri().path();
    let decoded = percent_encoding::percent_decode(raw.as_bytes().get(1..).unwrap_or(b""))
        .decode_utf8_lossy()
        .into_owned();

    let roots = known_roots(app);
    let Some(source) = validate_capture_path(Path::new(&decoded), &roots) else {
        return not_found_response();
    };

    let width = request
        .uri()
        .query()
        .and_then(parse_width_query)
        .unwrap_or(THUMB_DEFAULT_WIDTH)
        .clamp(THUMB_MIN_WIDTH, THUMB_MAX_WIDTH);

    match thumbnail_jpeg(app, &source, width) {
        Ok(bytes) => tauri::http::Response::builder()
            .header(tauri::http::header::CONTENT_TYPE, "image/jpeg")
            .body(bytes)
            .unwrap(),
        Err(_) => not_found_response(),
    }
}

/// The `w` query parameter, e.g. `"w=400"` or `"w=400&x=1"`.
fn parse_width_query(query: &str) -> Option<u32> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("w="))
        .and_then(|v| v.parse().ok())
}

/// A JPEG (quality 80) of `source`, at most `width` wide (aspect kept, never enlarged),
/// cached in `<app cache dir>/thumbs/<hash of path, width, and mtime>.jpg`.
fn thumbnail_jpeg<R: Runtime>(
    app: &AppHandle<R>,
    source: &Path,
    width: u32,
) -> io::Result<Vec<u8>> {
    let cache_path = thumb_cache_path(app, source, width)?;
    if let Ok(bytes) = fs::read(&cache_path) {
        return Ok(bytes);
    }

    let img = image::open(source).map_err(io::Error::other)?.into_rgb8();
    let (w, h) = img.dimensions();
    let target_w = width.min(w).max(1);
    let target_h = ((u64::from(h) * u64::from(target_w)) / u64::from(w.max(1))).max(1) as u32;
    let scaled = image::imageops::resize(
        &img,
        target_w,
        target_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, THUMB_QUALITY)
        .encode_image(&scaled)
        .map_err(io::Error::other)?;

    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&cache_path, &bytes); // best-effort: a cache-write failure still serves

    Ok(bytes)
}

fn thumb_cache_path<R: Runtime>(
    app: &AppHandle<R>,
    source: &Path,
    width: u32,
) -> io::Result<PathBuf> {
    let modified = fs::metadata(source)?.modified()?;
    let mtime_nanos = modified
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    width.hash(&mut hasher);
    mtime_nanos.hash(&mut hasher);
    let key = hasher.finish();

    let cache_dir = app.path().app_cache_dir().map_err(io::Error::other)?;
    Ok(cache_dir.join("thumbs").join(format!("{key:016x}.jpg")))
}

// ---------------------------------------------------------------------------------------
// Commands (docs/impl/library.md > Commands)
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub async fn open_library(app: AppHandle) {
    open_window(&app);
}

#[tauri::command]
pub async fn list_sessions(app: AppHandle) -> Vec<SessionSummary> {
    scan_sessions(&known_roots(&app), active_session_dir(&app).as_deref())
}

#[tauri::command]
pub async fn get_session(app: AppHandle, dir: String) -> Result<SessionDetail, String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    build_detail(&session_dir, active_session_dir(&app).as_deref())
        .ok_or_else(|| i18n::library_not_found(lang).to_string())
}

#[tauri::command]
pub async fn open_session_folder(app: AppHandle, dir: String) -> Result<(), String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    let _ = app
        .opener()
        .open_path(session_dir.to_string_lossy(), None::<&str>);
    Ok(())
}

#[tauri::command]
pub async fn open_session_pdf(app: AppHandle, dir: String) -> Result<(), String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    let pdf = session_dir.join(zsp_output::PDF_FILE);
    if !pdf.is_file() {
        return Err(i18n::library_no_pdf(lang).to_string());
    }
    let _ = app.opener().open_path(pdf.to_string_lossy(), None::<&str>);
    Ok(())
}

#[tauri::command]
pub async fn rebuild_session_pdf(app: AppHandle, dir: String) -> Result<usize, String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    if is_recording(&app, &session_dir) {
        return Err(i18n::library_recording_locked(lang).to_string());
    }
    let pages = zsp_output::rebuild_pdf(&session_dir, chrono::Local::now()).map_err(|e| {
        if e.kind() == io::ErrorKind::InvalidInput {
            i18n::library_no_images(lang).to_string()
        } else {
            i18n::error_save_folder(lang, &e.to_string())
        }
    })?;
    notify_changed(&app);
    Ok(pages)
}

#[tauri::command]
pub async fn rename_session(
    app: AppHandle,
    dir: String,
    title: Option<String>,
) -> Result<(), String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    let current = zsp_output::read_meta(&session_dir);
    zsp_output::write_meta(
        &session_dir,
        &zsp_output::SessionMetaFile::new(title.as_deref(), current.memo),
    )
    .map_err(|e| i18n::error_save_folder(lang, &e.to_string()))?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn set_session_memo(app: AppHandle, dir: String, memo: String) -> Result<(), String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    let current = zsp_output::read_meta(&session_dir);
    zsp_output::write_meta(
        &session_dir,
        &zsp_output::SessionMetaFile::new(current.title.as_deref(), memo),
    )
    .map_err(|e| i18n::error_save_folder(lang, &e.to_string()))?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn trash_session(app: AppHandle, dir: String) -> Result<(), String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    if is_recording(&app, &session_dir) {
        return Err(i18n::library_recording_locked(lang).to_string());
    }
    zsp_output::OsTrash
        .trash(&session_dir)
        .map_err(|e| i18n::error_save_folder(lang, &e.to_string()))?;
    clear_last_session_if(&app, &session_dir);
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn trash_captures(
    app: AppHandle,
    dir: String,
    indices: Vec<usize>,
) -> Result<usize, String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    if is_recording(&app, &session_dir) {
        return Err(i18n::library_recording_locked(lang).to_string());
    }
    let moved = zsp_output::trash_captures(
        &session_dir,
        &indices,
        &zsp_output::OsTrash,
        chrono::Local::now(),
    )
    .map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            i18n::library_bad_index(lang).to_string()
        } else {
            i18n::error_save_folder(lang, &e.to_string())
        }
    })?;
    notify_changed(&app);
    Ok(moved)
}

#[tauri::command]
pub async fn fold_session(app: AppHandle, dir: String) -> Result<Vec<usize>, String> {
    let lang = current_lang(&app);
    let session_dir = resolve_session_dir(&app, &dir, lang)?;
    if is_recording(&app, &session_dir) {
        return Err(i18n::library_recording_locked(lang).to_string());
    }
    fold_dry_run(&session_dir).map_err(|e| i18n::error_save_folder(lang, &e.to_string()))
}

#[tauri::command]
pub async fn open_capture(app: AppHandle, path: String) -> Result<(), String> {
    let lang = current_lang(&app);
    let roots = known_roots(&app);
    let canonical = validate_capture_path(Path::new(&path), &roots)
        .ok_or_else(|| i18n::library_capture_not_found(lang).to_string())?;
    let _ = app
        .opener()
        .open_path(canonical.to_string_lossy(), None::<&str>);
    Ok(())
}

/// `trash_session`: clears `lastSession` when it pointed at the trashed folder
/// (docs/impl/library.md > Commands), syncing `app-state` if it changed. Never called
/// while the shell lock is held.
fn clear_last_session_if<R: Runtime>(app: &AppHandle<R>, dir: &Path) {
    let cleared = {
        let shell = app.state::<SharedShell>();
        let mut shell = shell.lock().unwrap();
        if shell.last_session.as_ref().is_some_and(|l| l.dir == dir) {
            shell.last_session = None;
            true
        } else {
            false
        }
    };
    if cleared {
        crate::ui::sync(app);
    }
}

// ---------------------------------------------------------------------------------------
// Tests (ACCEPTANCE B13): scan order/skip rules, and capture-path validation. Pure
// functions, no running app.
// ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn make_session(
        root: &Path,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
    ) -> PathBuf {
        let started = Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap();
        let meta = zsp_output::SessionMeta {
            app_version: "0.1.0".into(),
            source: zsp_output::Source {
                kind: zsp_output::SourceKind::Window,
                app: Some("zoom.us".into()),
                title: Some("Lecture".into()),
                platform: "macos".into(),
                file: None,
            },
            region: None,
            interval_s: 0.5,
        };
        zsp_output::SessionWriter::create(root, started, meta)
            .unwrap()
            .dir()
            .to_path_buf()
    }

    /// B13: two save roots, newest `startedAt` first; a folder with no session.json and
    /// one with an unparsable session.json are both skipped.
    #[test]
    fn scan_returns_newest_first_and_skips_bad_folders() {
        let root_a = tempfile::tempdir().unwrap();
        let root_b = tempfile::tempdir().unwrap();

        let older = make_session(root_a.path(), 2026, 1, 1, 9, 0);
        let newer = make_session(root_b.path(), 2026, 6, 1, 9, 0);

        fs::create_dir_all(root_a.path().join("no-session")).unwrap();
        let bad = root_a.path().join("bad-session");
        fs::create_dir_all(&bad).unwrap();
        fs::write(bad.join("session.json"), b"not json").unwrap();

        let roots = vec![root_a.path().to_path_buf(), root_b.path().to_path_buf()];
        let found = scan_sessions(&roots, None);

        let dirs: Vec<String> = found.iter().map(|s| s.dir.clone()).collect();
        assert_eq!(
            dirs,
            vec![
                ipc_types::path_to_string(&newer),
                ipc_types::path_to_string(&older),
            ]
        );
    }

    /// A session matching `active_dir` is marked `recording`, even though `endedAt` is
    /// null (as it would also be after a crash) -- this is the only thing that tells the
    /// two apart.
    #[test]
    fn scan_marks_the_active_session_as_recording() {
        let root = tempfile::tempdir().unwrap();
        let dir = make_session(root.path(), 2026, 1, 1, 9, 0);

        let not_recording = scan_sessions(&[root.path().to_path_buf()], None);
        assert_eq!(not_recording.len(), 1);
        assert!(!not_recording[0].recording);

        let recording = scan_sessions(&[root.path().to_path_buf()], Some(&dir));
        assert_eq!(recording.len(), 1);
        assert!(recording[0].recording);
    }

    /// B13: a capture PNG inside a session folder of a known root is accepted; outside
    /// the root, `..` traversal, a non-PNG file, and the session folder itself are all
    /// rejected.
    #[test]
    fn validate_capture_path_rules() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let roots = vec![root.path().to_path_buf()];

        let session_dir = root.path().join("2026-01-01_0900");
        fs::create_dir_all(&session_dir).unwrap();
        let png = session_dir.join("001_00-00-01.png");
        fs::write(&png, b"not a real png, just bytes for the path check").unwrap();
        let session_json = session_dir.join("session.json");
        fs::write(&session_json, b"{}").unwrap();

        // Accepts: a PNG directly inside a session folder of a known root.
        assert_eq!(
            validate_capture_path(&png, &roots),
            Some(fs::canonicalize(&png).unwrap())
        );

        // Rejects: outside the root entirely.
        let outside_png = outside.path().join("001_00-00-01.png");
        fs::write(&outside_png, b"x").unwrap();
        assert_eq!(validate_capture_path(&outside_png, &roots), None);

        // Rejects: `..` traversal that resolves outside the root (two tempdirs created
        // back to back share a parent, so this walks back out to `outside` and across).
        let traversal_png = outside.path().join("traversal.png");
        fs::write(&traversal_png, b"x").unwrap();
        let traversal_path = session_dir
            .join("..")
            .join("..")
            .join(outside.path().file_name().unwrap())
            .join("traversal.png");
        assert_eq!(validate_capture_path(&traversal_path, &roots), None);

        // Rejects: a non-PNG file.
        assert_eq!(validate_capture_path(&session_json, &roots), None);

        // Rejects: the session folder itself (not a file, and not `.png`).
        assert_eq!(validate_capture_path(&session_dir, &roots), None);
    }
}
