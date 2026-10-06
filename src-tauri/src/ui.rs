//! Pushes the shell state out: the `app-state` event to every window, and the tray.
//!
//! Threading rule (docs/impl/shell.md > Threads): a tray or menu call made off the main
//! thread blocks until the main thread has run it (tauri's `run_item_main_thread`). A
//! thread that did that while holding the shell lock would deadlock against main-thread
//! code waiting for the same lock: sync commands, tray menu and global shortcut handlers.
//! So `sync` does all of it on the main thread, posted there without waiting when called
//! from any other thread, and takes the lock itself.

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::state::{self, SharedShell};
use crate::tray;

/// Emits the current `AppState` and updates the tray to match.
///
/// Call it without holding the shell lock: on the main thread the work runs right away
/// and locks the shell itself.
pub fn sync<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let (app_state, tray_model) = {
            let shell = handle.state::<SharedShell>();
            let shell = shell.lock().unwrap();
            (state::snapshot(&shell), tray::Model::of(&shell))
        };
        let _ = handle.emit("app-state", app_state);
        tray::apply(&handle, tray_model);
    });
}
