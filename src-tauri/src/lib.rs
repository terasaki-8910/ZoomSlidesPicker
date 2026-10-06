//! ZoomSlidesPicker desktop shell: connects `zsp-session` to the tray, the windows, and
//! the React UI. See docs/impl/shell.md and docs/impl/ipc.md for the contract.

mod autostart;
mod capture_cmds;
mod commands;
mod i18n;
mod ipc_types;
mod notify;
mod session_ctl;
mod session_start;
mod settings;
mod shortcuts;
mod state;
mod thumb;
mod tray;
mod video;
mod windows;

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, RunEvent};

use state::{SharedShell, Shell};

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(shortcuts::handle)
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            // No Dock icon: this is a resident, tray-only app (SPEC.md > 使い方の流れ).
            #[cfg(target_os = "macos")]
            app.handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory)?;

            let settings = settings::load(app.handle());
            let app_version = app.package_info().version.to_string();
            app.manage(Mutex::new(Shell::new(app_version, settings)) as SharedShell);

            let handle = app.handle().clone();
            tray::build(&handle)?;
            shortcuts::apply(&handle);
            {
                let shell = handle.state::<SharedShell>();
                tray::refresh(&handle, &shell.lock().unwrap());
            }

            start_ticker(handle.clone());
            autostart::run(&handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::get_settings,
            commands::set_settings,
            commands::apply_language,
            commands::choose_save_root,
            commands::list_windows,
            commands::begin_region,
            commands::get_region_snapshot,
            commands::start_session,
            commands::cancel_region,
            commands::pause_session,
            commands::resume_session,
            commands::save_now,
            commands::undo_last,
            commands::stop_session,
            commands::open_folder,
            commands::rebuild_pdf,
            commands::extract_video,
            commands::request_permission,
            commands::open_permission_settings,
            commands::open_settings,
            commands::close_window,
            commands::fit_panel,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the tauri application");

    app.run(|app_handle, event| {
        // This app has no main window: closing the last window must not quit it (it
        // lives in the tray with zero windows open). `code` is None exactly when the
        // exit was requested that way (the last window closing) and Some when it was
        // requested programmatically (AppHandle::exit, called only by the Quit menu
        // item and the ZSP_AUTOSTART_SECONDS test hook) -- only the latter should
        // actually proceed, after stopping and joining any running session so its PDF
        // is written before the process exits (docs/impl/shell.md > Behaviour).
        //
        // We always drive the actual exit ourselves (`stop_and_join`, below) rather than
        // letting the runtime's own exit machinery finish the job after this callback
        // returns: that path was observed empirically (the D1 smoke test) to never
        // complete for this accessory-policy, often-zero-window app, so every branch
        // below calls `prevent_exit`.
        if let RunEvent::ExitRequested { code, api, .. } = event {
            api.prevent_exit();
            if let Some(code) = code {
                stop_and_join(app_handle, code);
            }
        }
    });
}

/// Emits `app-state` at least once a second while a session runs (docs/impl/shell.md >
/// Behaviour: elapsed time). A no-op tick when there is none.
fn start_ticker(app: AppHandle) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let shell = app.state::<SharedShell>();
            let guard = shell.lock().unwrap();
            if guard.session.is_some() {
                state::emit_app_state(&app, &guard);
                tray::refresh(&app, &guard);
            }
        }
    });
}

/// Stops a running session and exits once its thread has fully ended, so the PDF is
/// written before the process exits (docs/impl/shell.md > Behaviour: on quit).
///
/// Takes the session out of the shared state up front (so the session thread's own
/// notify callback, which re-locks that same state to record it finished, never
/// contends with this function for it), then joins and exits on a **separate** thread
/// rather than this one (the main thread, here). Joining right on the main thread was
/// tried first and deadlocks: that notify callback also refreshes the tray, which on
/// macOS must run on the main thread, so a main thread blocked in `join()` can never
/// service it and the session thread (and so the join) never completes. Freeing the
/// main thread to keep servicing that dispatch while a different thread waits avoids
/// the deadlock; process::exit works from any thread once the wait is over.
fn stop_and_join(app: &AppHandle, code: i32) {
    let active = {
        let shell = app.state::<SharedShell>();
        shell.lock().unwrap().session.take()
    };
    let Some(active) = active else {
        std::process::exit(code);
    };
    active.handle.send(zsp_session::Control::Stop);
    thread::spawn(move || {
        let _ = active.handle.join();
        std::process::exit(code);
    });
}
