//! Session controls that act on the already-running session (docs/impl/ipc.md): pause,
//! resume, save now, undo, stop, open the save folder, rebuild the last session's PDF.
//! Used by both the `#[tauri::command]` wrappers (`commands.rs`) and the tray menu
//! (`tray.rs`).

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_opener::OpenerExt;
use zsp_session::Control;

use crate::state::SharedShell;
use crate::{i18n, settings, ui};

fn send_control<R: Runtime>(app: &AppHandle<R>, control: Control) {
    let shell = app.state::<SharedShell>();
    if let Some(active) = &shell.lock().unwrap().session {
        active.handle.send(control);
    }
}

pub fn pause<R: Runtime>(app: &AppHandle<R>) {
    send_control(app, Control::Pause);
}

pub fn resume<R: Runtime>(app: &AppHandle<R>) {
    send_control(app, Control::Resume);
}

pub fn save_now<R: Runtime>(app: &AppHandle<R>) {
    send_control(app, Control::SaveNow);
}

pub fn undo_last<R: Runtime>(app: &AppHandle<R>) {
    send_control(app, Control::Undo);
}

/// `stop_session`: sends `Control::Stop` and returns immediately. The session thread
/// writes the PDF and, through its notify callback (`notify.rs`), updates the shared
/// state and sends the stop notification -- the control is handled essentially
/// instantly (the engine checks for one before every wait), so there is nothing to wait
/// for here. Only app quit needs a hard guarantee that the write finished before the
/// process exits; that path sends `Stop` and joins explicitly (`lib.rs`).
pub fn stop<R: Runtime>(app: &AppHandle<R>) {
    send_control(app, Control::Stop);
}

/// `open_folder`: the current session folder, else the last session's (docs/impl/ipc.md).
pub fn open_folder<R: Runtime>(app: &AppHandle<R>) {
    let shell = app.state::<SharedShell>();
    let guard = shell.lock().unwrap();
    let dir = guard
        .session
        .as_ref()
        .map(|a| a.handle.status().dir)
        .or_else(|| guard.last_session.as_ref().map(|l| l.dir.clone()));
    drop(guard);

    if let Some(dir) = dir {
        let _ = app.opener().open_path(dir.to_string_lossy(), None::<&str>);
    }
}

/// `rebuild_pdf`: rebuilds the last session's PDF from the images still in its folder
/// and returns the page count (docs/impl/ipc.md). Errors when there is no last session
/// or it has no images left. Slow with many pages, so callers run it off the main thread;
/// the shell lock is not held while the PDF is written.
pub fn rebuild_pdf<R: Runtime>(app: &AppHandle<R>) -> Result<usize, String> {
    let shell_state = app.state::<SharedShell>();
    let (dir, lang) = {
        let shell = shell_state.lock().unwrap();
        let lang = settings::effective_language(shell.settings.language);
        let Some(last) = shell.last_session.as_ref() else {
            return Err(i18n::error_no_last_session_images(lang).to_string());
        };
        (last.dir.clone(), lang)
    };

    let pages = zsp_output::rebuild_pdf(&dir, chrono::Local::now()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidInput {
            i18n::error_no_last_session_images(lang).to_string()
        } else {
            i18n::error_save_folder(lang, &e.to_string())
        }
    })?;

    {
        let mut shell = shell_state.lock().unwrap();
        if let Some(last) = shell.last_session.as_mut()
            && last.dir == dir
        {
            last.count = pages;
            last.has_pdf = true;
        }
    }
    ui::sync(app);
    Ok(pages)
}

/// The tray menu's "PDF を作り直す": same as the command, moved off the main thread
/// (where menu events arrive), with nowhere to show the result or an error.
pub fn rebuild_pdf_from_menu<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = rebuild_pdf(&app);
    });
}
