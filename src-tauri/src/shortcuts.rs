//! Global shortcuts: registered from settings at startup and after every `set_settings`
//! (docs/impl/shell.md > Behaviour). The handler maps the fired shortcut back to one of
//! the three configured accelerators by equality, not a stored enum, since the mapping
//! itself can change at any time via `set_settings`.

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::session_ctl;
use crate::state::SharedShell;

/// Unregisters every shortcut, then registers the three from the current settings
/// ("register ... at startup and after every set_settings (unregister the old ones
/// first)"). A parse failure is skipped silently -- `set_settings` already validates
/// accelerators before they reach here.
pub fn apply<R: Runtime>(app: &AppHandle<R>) {
    let shortcuts = {
        let shell = app.state::<SharedShell>();
        shell.lock().unwrap().settings.shortcuts.clone()
    };
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for accel in [
        &shortcuts.save_now,
        &shortcuts.undo,
        &shortcuts.pause_resume,
    ] {
        if let Ok(shortcut) = accel.parse::<Shortcut>() {
            let _ = gs.register(shortcut);
        }
    }
}

/// The `with_handler` callback registered once at plugin init (`lib.rs`): dispatches to
/// the control the fired shortcut currently maps to. All three do nothing without a
/// session -- `session_ctl`'s controls already guarantee that.
pub fn handle<R: Runtime>(app: &AppHandle<R>, fired: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let shortcuts = {
        let shell = app.state::<SharedShell>();
        shell.lock().unwrap().settings.shortcuts.clone()
    };
    if matches(&shortcuts.save_now, fired) {
        session_ctl::save_now(app);
    } else if matches(&shortcuts.undo, fired) {
        session_ctl::undo_last(app);
    } else if matches(&shortcuts.pause_resume, fired) {
        toggle_pause_resume(app);
    }
}

fn matches(accel: &str, fired: &Shortcut) -> bool {
    accel.parse::<Shortcut>().is_ok_and(|s| &s == fired)
}

fn toggle_pause_resume<R: Runtime>(app: &AppHandle<R>) {
    let shell = app.state::<SharedShell>();
    let paused = shell
        .lock()
        .unwrap()
        .session
        .as_ref()
        .map(|a| a.handle.status().state == zsp_session::State::Paused);
    match paused {
        Some(true) => session_ctl::resume(app),
        Some(false) => session_ctl::pause(app),
        None => {}
    }
}
