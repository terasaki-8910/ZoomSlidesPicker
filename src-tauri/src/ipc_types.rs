//! DTOs exchanged with the webview (docs/impl/ipc.md). The wire format is camelCase;
//! kept in exact sync with `src/ipc/types.ts` by hand -- there is no code generation.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Ja,
    En,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LangSetting {
    Auto,
    Ja,
    En,
}

/// Accelerator strings in tauri-plugin-global-shortcut format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcuts {
    pub save_now: String,
    pub undo: String,
    pub pause_resume: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub language: LangSetting,
    pub save_root: String,
    pub interval_s: f64,
    pub shortcuts: Shortcuts,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub language: LangSetting,
    pub save_root: String,
    pub interval_s: f64,
    pub shortcuts: Shortcuts,
    pub effective_language: Lang,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Running,
    Paused,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Window,
    Mp4,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub state: SessionState,
    /// Captures in the session.
    pub count: usize,
    /// Session folder.
    pub dir: String,
    /// Data URL (PNG, at most 280 px wide) of the latest capture.
    pub last_thumb: Option<String>,
    pub elapsed_s: f64,
    /// Single-colour screen for 10 s or more.
    pub blank: bool,
    /// The window is gone; the session is paused.
    pub window_lost: bool,
    /// Stop was requested; the PDF is being written. Cleared when the session finishes.
    pub stopping: bool,
    pub source_kind: SourceKind,
    /// Window title, or video file name.
    pub source_title: String,
}

/// The most recent finished session.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSession {
    pub dir: String,
    pub count: usize,
    pub has_pdf: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Macos,
    Windows,
    Linux,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionView {
    /// Always true outside macOS.
    pub granted: bool,
    pub platform: Platform,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProgress {
    /// Frames processed.
    pub done: u64,
    /// When known.
    pub total: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    /// Null when no session is running or paused.
    pub session: Option<SessionView>,
    pub last_session: Option<LastSession>,
    pub permission: PermissionView,
    /// Non-null while a video is being processed.
    pub video_progress: Option<VideoProgress>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowEntry {
    pub id: u32,
    pub app: String,
    pub title: String,
    /// Points.
    pub width: u32,
    pub height: u32,
    /// Data URL (PNG, at most 240 px wide).
    pub thumb: String,
}

/// For the region window.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub window_id: u32,
    pub app: String,
    pub title: String,
    /// Pixels of the captured window.
    pub width: u32,
    pub height: u32,
    /// Data URL (PNG, at most 1600 px wide).
    pub image: String,
}

/// Fractions 0-1 of the window.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionSel {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A version the updater found (docs/impl/updates.md > Types).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
}

/// `check_update`'s result: `skipped` says whether `available` is the skipped version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current: String,
    pub available: Option<UpdateInfo>,
    pub skipped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdatePhase {
    Downloading,
    Installing,
}

/// The `update-progress` event payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub phase: UpdatePhase,
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// `answer_update`'s argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateAnswer {
    Later,
    Skip,
}

/// A session folder path, as sent to the webview: always UTF-8 (SPEC.md session folders are
/// named from an RFC 3339-safe timestamp, never from arbitrary OS path bytes).
pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
