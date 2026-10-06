//! Central shared state (docs/impl/shell.md): the active session (if any), the last
//! finished session, and everything needed to answer `get_app_state` and rebuild the
//! tray. Behind one `Mutex` -- commands, the notify callback (which runs on the session
//! thread), and the periodic ticker all touch it.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Runtime};
use zsp_session::EngineHandle;

use crate::ipc_types::{
    self, AppState, LastSession, PermissionView, Platform, Settings, SourceKind,
};

pub struct ActiveSession {
    pub handle: EngineHandle,
    pub source_kind: SourceKind,
    pub source_title: String,
    /// The interval this session was started with (settings can change while it runs;
    /// the running session keeps using its own).
    pub interval_s: f64,
    /// Cached data-URL thumbnail of the latest capture, keyed by its PNG path so it is
    /// only recomputed when the latest capture changes.
    pub thumb: Option<(PathBuf, String)>,
    /// Mp4 sessions only: the frame count known from the video's duration via ffprobe,
    /// when available (docs/impl/shell.md > extract_video).
    pub video_total_frames: Option<u64>,
}

pub struct LastSessionInfo {
    pub dir: PathBuf,
    pub count: usize,
    pub has_pdf: bool,
}

pub struct Shell {
    pub settings: Settings,
    pub session: Option<ActiveSession>,
    pub last_session: Option<LastSessionInfo>,
    /// The snapshot `begin_region` captured, until `start_session` or `cancel_region`.
    pub pending_region: Option<ipc_types::Snapshot>,
    pub app_version: String,
}

impl Shell {
    pub fn new(app_version: String, settings: Settings) -> Self {
        Self {
            settings,
            session: None,
            last_session: None,
            pending_region: None,
            app_version,
        }
    }
}

/// The shared state type managed by Tauri; commands take `State<'_, SharedShell>`.
pub type SharedShell = Mutex<Shell>;

pub fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// The platform string zsp-output's `Source.platform` expects (SPEC.md > 出力).
pub fn platform_str() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn session_state(s: zsp_session::State) -> ipc_types::SessionState {
    match s {
        zsp_session::State::Running => ipc_types::SessionState::Running,
        zsp_session::State::Paused => ipc_types::SessionState::Paused,
        zsp_session::State::Stopped => ipc_types::SessionState::Stopped,
    }
}

/// Builds the `AppState` snapshot sent to the webview (docs/impl/ipc.md).
pub fn snapshot(shell: &Shell) -> AppState {
    let permission = PermissionView {
        granted: zsp_capture::permission::screen_capture_granted(),
        platform: current_platform(),
    };

    let mut video_progress = None;
    let session = shell.session.as_ref().map(|active| {
        let status = active.handle.status();

        if active.source_kind == SourceKind::Mp4 {
            let done = (status.elapsed_s / active.interval_s).round().max(0.0) as u64 + 1;
            video_progress = Some(ipc_types::VideoProgress {
                done,
                total: active.video_total_frames,
            });
        }

        ipc_types::SessionView {
            state: session_state(status.state),
            count: status.count,
            dir: ipc_types::path_to_string(&status.dir),
            last_thumb: active.thumb.as_ref().map(|(_, data_url)| data_url.clone()),
            elapsed_s: status.elapsed_s,
            blank: status.blank,
            window_lost: status.window_lost,
            source_kind: active.source_kind,
            source_title: active.source_title.clone(),
        }
    });

    let last_session = shell.last_session.as_ref().map(|l| LastSession {
        dir: ipc_types::path_to_string(&l.dir),
        count: l.count,
        has_pdf: l.has_pdf,
    });

    AppState {
        session,
        last_session,
        permission,
        video_progress,
    }
}

/// Emits the current `AppState` to every window (docs/impl/ipc.md > Events). Called
/// after every state change: session control, settings load, and -- while a session
/// runs -- at least once a second by the ticker (docs/impl/shell.md > Behaviour).
pub fn emit_app_state<R: Runtime>(app: &AppHandle<R>, shell: &Shell) {
    let state = snapshot(shell);
    let _ = app.emit("app-state", state);
}
