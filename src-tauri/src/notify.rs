//! Builds the `Notify` callback a session runs on its own thread (zsp_session::spawn):
//! updates the shared state, sends OS notifications, then `ui::sync` (docs/impl/shell.md >
//! Behaviour). Notification failures never break a session -- every result here is
//! dropped. Nothing here waits for the main thread (`ui.rs`), so joining a session thread
//! from the main thread is safe.

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;
use zsp_session::{Notice, Status};

use crate::ipc_types::Lang;
use crate::state::{LastSessionInfo, SharedShell};
use crate::{i18n, settings, thumb, ui};

const THUMB_MAX_WIDTH: u32 = 280;

fn show<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Builds the per-session notify callback (zsp_session::Notify), bound to `app`.
pub fn make_notify<R: Runtime>(app: AppHandle<R>) -> zsp_session::Notify {
    Box::new(move |notice, status| on_notice(&app, notice, status))
}

fn on_notice<R: Runtime>(app: &AppHandle<R>, notice: &Notice, status: &Status) {
    let shell_state = app.state::<SharedShell>();

    // The latest capture's thumbnail: whether it is stale is decided under the lock, the
    // PNG is read back and scaled outside it.
    let stale = {
        let shell = shell_state.lock().unwrap();
        shell.session.as_ref().is_some_and(|a| {
            a.dir == status.dir && a.thumb.as_ref().map(|(p, _)| p) != status.last_capture.as_ref()
        })
    };
    let thumb = match (&status.last_capture, stale) {
        (Some(path), true) => thumb::file_thumbnail_data_url(path, THUMB_MAX_WIDTH)
            .ok()
            .map(|data_url| (path.clone(), data_url)),
        _ => None,
    };

    let lang = {
        let mut shell = shell_state.lock().unwrap();
        let ours = shell.session.as_ref().is_some_and(|a| a.dir == status.dir);
        if ours
            && stale
            && let Some(active) = shell.session.as_mut()
        {
            active.thumb = thumb;
        }
        if let Notice::Finished { dir, count, pdf } = notice {
            if ours {
                shell.session = None;
            }
            shell.last_session = Some(LastSessionInfo {
                dir: dir.clone(),
                count: *count,
                has_pdf: pdf.is_some(),
            });
        }
        settings::effective_language(shell.settings.language)
    };

    match notice {
        Notice::Blank => show(app, i18n::title_panel(), i18n::notify_blank_body(lang)),
        Notice::WindowLost => show(
            app,
            i18n::title_panel(),
            i18n::notify_window_lost_body(lang),
        ),
        Notice::Finished { count, .. } => {
            show(
                app,
                i18n::title_panel(),
                &i18n::notify_stop_body(lang, *count),
            );
            crate::library::notify_changed(app);
        }
        // A capture was saved, replaced, or undone: the library window's count and
        // thumbnails may be stale (docs/impl/library.md > Events).
        Notice::Saved { .. } | Notice::Replaced { .. } | Notice::Undone { .. } => {
            crate::library::notify_changed(app);
        }
        Notice::Paused | Notice::Resumed | Notice::WindowBack => {}
    }
    ui::sync(app);
}

/// Sends the start notification (there is no `Notice` for it -- a session's start is
/// never something the engine reports to itself, only something the shell causes).
pub fn notify_started<R: Runtime>(app: &AppHandle<R>, lang: Lang) {
    show(
        app,
        i18n::notify_start_title(),
        i18n::notify_start_body(lang),
    );
}

/// Sends the macOS permission notification (docs/impl/shell.md > Behaviour:
/// `start_session` refuses without it and sends this).
pub fn notify_permission_missing<R: Runtime>(app: &AppHandle<R>, lang: Lang) {
    show(app, i18n::title_panel(), i18n::permission_notice(lang));
}
