//! ZoomSlidesPicker desktop shell: connects `zsp-session` to the tray, the windows, and
//! the React UI. See docs/impl/shell.md and docs/impl/ipc.md for the contract.

mod autostart;
mod capture_cmds;
mod commands;
mod i18n;
mod ipc_types;
mod library;
mod notify;
mod session_ctl;
mod session_start;
mod settings;
mod shortcuts;
mod state;
mod thumb;
mod tray;
mod ui;
mod video;
mod windows;

use std::env;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, RunEvent};

use state::{SharedShell, Shell};

pub fn run() {
    let builder = library::register_protocol(
        tauri::Builder::default()
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
                // The library window raises it to `Regular` while it is open (library.rs).
                #[cfg(target_os = "macos")]
                app.handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory)?;

                let (mut settings, known_roots) = settings::load(app.handle());
                // Test hook: a save root for this run only, never written to the settings
                // (docs/impl/shell.md > Test hook).
                if let Ok(root) = env::var("ZSP_SAVE_ROOT") {
                    settings.save_root = root;
                }
                let app_version = app.package_info().version.to_string();
                app.manage(
                    Mutex::new(Shell::new(app_version, settings, known_roots)) as SharedShell
                );

                let handle = app.handle().clone();
                tray::build(&handle)?;
                shortcuts::apply(&handle);
                ui::sync(&handle);

                start_ticker(handle.clone());
                // The library window opens at launch (R3-8 a), unless a test hook is in
                // play: ZSP_OPEN_WINDOW asks for a specific window, and
                // ZSP_AUTOSTART_TITLE starts a session outright (docs/impl/shell.md >
                // Test hook).
                if env::var("ZSP_AUTOSTART_TITLE").is_err() && env::var("ZSP_OPEN_WINDOW").is_err()
                {
                    library::open_window(&handle);
                }
                autostart::spawn(handle);

                Ok(())
            }),
    );

    let app = builder
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
            library::open_library,
            library::list_sessions,
            library::get_session,
            library::open_session_folder,
            library::open_session_pdf,
            library::rebuild_session_pdf,
            library::rename_session,
            library::set_session_memo,
            library::trash_session,
            library::trash_captures,
            library::fold_session,
            library::open_capture,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the tauri application");

    app.run(|app_handle, event| match event {
        // The last window closed. This app lives in the tray with no window open, so it
        // stays (SPEC.md > 使い方の流れ).
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        // Quit (the tray menu, or the ZSP_AUTOSTART_SECONDS test hook): stop a running
        // session first and exit once its PDF is written (docs/impl/shell.md >
        // Behaviour). The wait happens on another thread so the tray keeps working; that
        // thread asks to exit again, and with no session left the exit goes ahead.
        RunEvent::ExitRequested {
            code: Some(code),
            api,
            ..
        } => {
            if let Some(active) = take_session(app_handle) {
                api.prevent_exit();
                active.handle.send(zsp_session::Control::Stop);
                let app = app_handle.clone();
                thread::spawn(move || {
                    let _ = active.handle.join();
                    app.exit(code);
                });
            }
        }
        // macOS: the app was opened again (Finder, Spotlight, `open`) while running. Open
        // or focus the library window, since the menu bar may have no room left to show
        // the tray icon (docs/impl/library.md > Window).
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => library::open_window(app_handle),
        // Exiting without a request first (for example the OS logging out): finish a
        // running session here. Blocking the main thread is safe: the session thread
        // never waits for it (ui.rs).
        RunEvent::Exit => {
            if let Some(active) = take_session(app_handle) {
                active.handle.send(zsp_session::Control::Stop);
                let _ = active.handle.join();
            }
        }
        _ => {}
    });
}

fn take_session(app: &AppHandle) -> Option<state::ActiveSession> {
    app.state::<SharedShell>().lock().unwrap().session.take()
}

/// Updates `app-state` (elapsed time) and the tray at least once a second while a
/// session runs (docs/impl/shell.md > Behaviour).
fn start_ticker(app: AppHandle) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let running = app.state::<SharedShell>().lock().unwrap().session.is_some();
            if running {
                ui::sync(&app);
            }
        }
    });
}
