//! The `#[tauri::command]` functions (docs/impl/ipc.md). Each is a thin wrapper: the
//! real logic lives in the module it calls, which the tray menu also calls directly.
//! Commands without `async` run on the main thread. These are `async`, so Tauri runs them
//! on its async runtime instead (docs/impl/shell.md > Threads): the ones that block
//! (dialogs, the macOS permission prompt, captures, writing a PDF), and `open_settings`,
//! since creating a window from a synchronous command deadlocks on Windows (WebView2).

use tauri::{AppHandle, Emitter, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::ipc_types::{AppState, RegionSel, Settings, SettingsView, Snapshot, WindowEntry};
use crate::state::{self, SharedShell};
use crate::{
    capture_cmds, library, session_ctl, session_start, settings, shortcuts, ui, video, windows,
};

#[tauri::command]
pub fn get_app_state(shell: State<'_, SharedShell>) -> AppState {
    state::snapshot(&shell.lock().unwrap())
}

#[tauri::command]
pub fn get_settings(shell: State<'_, SharedShell>) -> SettingsView {
    settings::view(shell.lock().unwrap().settings.clone())
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    shell: State<'_, SharedShell>,
    settings: Settings,
) -> Result<SettingsView, String> {
    let lang = crate::settings::effective_language(shell.lock().unwrap().settings.language);
    crate::settings::validate(&settings, lang)?;

    // `knownRoots` is Rust-only (docs/impl/library.md > Roots): kept as-is, with the old
    // `saveRoot` appended when it changes.
    let (view, root_changed) = {
        let mut guard = shell.lock().unwrap();
        let root_changed = guard.settings.save_root != settings.save_root;
        if root_changed {
            let old_root = guard.settings.save_root.clone();
            if !guard.known_roots.contains(&old_root) {
                guard.known_roots.push(old_root);
            }
        }
        guard.settings = settings;
        crate::settings::save(&app, &guard.settings, &guard.known_roots)?;
        (crate::settings::view(guard.settings.clone()), root_changed)
    };
    shortcuts::apply(&app);
    ui::sync(&app);
    let _ = app.emit("settings", &view);
    if root_changed {
        library::notify_changed(&app);
    }
    Ok(view)
}

/// Rebuilds the parts outside the webview (tray menu, tooltip, window titles) in the
/// effective language (docs/recipes/tauri-desktop-app.md gotcha 7).
#[tauri::command]
pub fn apply_language(app: AppHandle) {
    windows::retitle_all(&app);
    ui::sync(&app);
}

#[tauri::command]
pub async fn choose_save_root(app: AppHandle) -> Option<String> {
    let picked = app.dialog().file().blocking_pick_folder()?;
    picked
        .into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn list_windows(app: AppHandle) -> Result<Vec<WindowEntry>, String> {
    capture_cmds::list_windows(&app)
}

#[tauri::command]
pub async fn begin_region(app: AppHandle, window_id: u32) -> Result<(), String> {
    capture_cmds::begin_region(&app, window_id)
}

#[tauri::command]
pub fn get_region_snapshot(app: AppHandle) -> Result<Snapshot, String> {
    capture_cmds::get_region_snapshot(&app)
}

#[tauri::command]
pub fn start_session(app: AppHandle, window_id: u32, region: RegionSel) -> Result<(), String> {
    session_start::start(&app, window_id, region)
}

#[tauri::command]
pub fn cancel_region(app: AppHandle) {
    capture_cmds::cancel_region(&app);
}

#[tauri::command]
pub fn pause_session(app: AppHandle) {
    session_ctl::pause(&app);
}

#[tauri::command]
pub fn resume_session(app: AppHandle) {
    session_ctl::resume(&app);
}

#[tauri::command]
pub fn save_now(app: AppHandle) {
    session_ctl::save_now(&app);
}

#[tauri::command]
pub fn undo_last(app: AppHandle) {
    session_ctl::undo_last(&app);
}

#[tauri::command]
pub fn stop_session(app: AppHandle) {
    session_ctl::stop(&app);
}

#[tauri::command]
pub fn open_folder(app: AppHandle) {
    session_ctl::open_folder(&app);
}

#[tauri::command]
pub async fn rebuild_pdf(app: AppHandle) -> Result<usize, String> {
    session_ctl::rebuild_pdf(&app)
}

#[tauri::command]
pub async fn extract_video(app: AppHandle) -> Result<(), String> {
    video::extract_video(&app)
}

#[tauri::command]
pub async fn request_permission(app: AppHandle) -> bool {
    let granted = zsp_capture::permission::request_screen_capture();
    ui::sync(&app);
    granted
}

#[tauri::command]
pub fn open_permission_settings(app: AppHandle) {
    if let Some(url) = zsp_capture::permission::settings_url() {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
}

#[tauri::command]
pub fn close_window(window: WebviewWindow) {
    let _ = window.close();
}

#[tauri::command]
pub fn fit_panel(window: WebviewWindow, height: f64) {
    windows::fit_panel(&window, height);
}
