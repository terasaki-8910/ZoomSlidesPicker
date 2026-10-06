//! Builds the `Notify` callback a session runs on its own thread (zsp_session::spawn):
//! updates the shared state, emits `app-state`, refreshes the tray, and sends OS
//! notifications (docs/impl/shell.md > Behaviour). Notification failures never break a
//! session -- every result here is dropped.

use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

use crate::state::{self, LastSessionInfo, SharedShell, Shell};
use crate::{i18n, settings, thumb, tray};

const THUMB_MAX_WIDTH: u32 = 280;

/// Recomputes `active.thumb` when the session's latest capture changed (cleared when
/// there is none). Reads the PNG back from disk -- the engine only hands this callback a
/// path, not pixels (zsp_session::Notice::Saved/Undone).
fn refresh_thumb(shell: &mut Shell, status: &zsp_session::Status) {
    let Some(active) = &mut shell.session else {
        return;
    };
    match &status.last_capture {
        Some(path) if active.thumb.as_ref().map(|(p, _)| p) != Some(path) => {
            active.thumb = thumb::file_thumbnail_data_url(path, THUMB_MAX_WIDTH)
                .ok()
                .map(|data_url| (path.clone(), data_url));
        }
        None => active.thumb = None,
        Some(_) => {}
    }
}

fn notify<R: tauri::Runtime>(app: &tauri::AppHandle<R>, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Builds the per-session notify callback (zsp_session::Notify), bound to `app`.
pub fn make_notify<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> zsp_session::Notify {
    Box::new(move |notice, status| on_notice(&app, notice, status))
}

fn on_notice<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    notice: &zsp_session::Notice,
    status: &zsp_session::Status,
) {
    use zsp_session::Notice;

    let shell_state = app.state::<SharedShell>();
    let mut shell = shell_state.lock().unwrap();

    refresh_thumb(&mut shell, status);

    let lang = settings::effective_language(shell.settings.language);
    match notice {
        Notice::Saved { .. } | Notice::Undone { .. } | Notice::WindowBack => {}
        Notice::Blank => notify(app, i18n::title_panel(), i18n::notify_blank_body(lang)),
        Notice::WindowLost => notify(
            app,
            i18n::title_panel(),
            i18n::notify_window_lost_body(lang),
        ),
        Notice::Finished { dir, count, pdf } => {
            shell.session.take();
            shell.last_session = Some(LastSessionInfo {
                dir: dir.clone(),
                count: *count,
                has_pdf: pdf.is_some(),
            });
            notify(
                app,
                i18n::title_panel(),
                &i18n::notify_stop_body(lang, *count),
            );
        }
    }

    state::emit_app_state(app, &shell);
    tray::refresh(app, &shell);
}

/// Sends the start notification (there is no `Notice` for it -- a session's start is
/// never something the engine reports to itself, only something the shell causes).
pub fn notify_started<R: tauri::Runtime>(app: &tauri::AppHandle<R>, lang: crate::ipc_types::Lang) {
    notify(
        app,
        i18n::notify_start_title(),
        i18n::notify_start_body(lang),
    );
}

/// Sends the macOS permission notification (docs/impl/shell.md > Behaviour:
/// `start_session` refuses without it and sends this).
pub fn notify_permission_missing<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    lang: crate::ipc_types::Lang,
) {
    notify(app, i18n::title_panel(), i18n::permission_notice(lang));
}
