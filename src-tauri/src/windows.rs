//! Window creation (docs/impl/ipc.md > Windows). Each window is created when opened and
//! destroyed when closed -- there is no window at launch (SPEC.md > 画面と操作).

use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, LogicalSize, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::ipc_types::Lang;
use crate::state::SharedShell;
use crate::{i18n, settings};

pub const PANEL_LABEL: &str = "panel";
const REGION_LABEL: &str = "region";
const SETTINGS_LABEL: &str = "settings";

const PANEL_WIDTH: f64 = 320.0;
const PANEL_DEFAULT_HEIGHT: f64 = 480.0;
const PANEL_MIN_HEIGHT: f64 = 160.0;
const PANEL_MAX_HEIGHT: f64 = 640.0;

/// The panel is shown once its page has rendered (its first `fit_panel`, so there is no
/// white flash), or after this long if that never comes.
const PANEL_SHOW_FALLBACK: Duration = Duration::from_millis(1500);
/// Windows: right after the panel is shown, the tray click and the tray menu hand focus
/// back to the shell, and the webview takes it while it starts. A blur this soon is that,
/// not the user leaving the panel, so the panel takes focus again instead of closing.
const PANEL_BLUR_GRACE: Duration = Duration::from_millis(800);

/// When the panel was last shown.
static PANEL_SHOWN_AT: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

fn show_panel<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    *PANEL_SHOWN_AT.lock().unwrap() = Some(Instant::now());
    let _ = window.show();
    let _ = window.set_focus();
}

/// Whether a blur of the panel now is one to ride out (see `PANEL_BLUR_GRACE`).
fn in_blur_grace() -> bool {
    cfg!(windows)
        && PANEL_SHOWN_AT
            .lock()
            .unwrap()
            .is_some_and(|at| at.elapsed() < PANEL_BLUR_GRACE)
}

fn effective_lang<R: Runtime>(app: &AppHandle<R>) -> Lang {
    let shell = app.state::<SharedShell>();
    let language = shell.lock().unwrap().settings.language;
    settings::effective_language(language)
}

/// Opens the panel if closed, closes it if open (tray left click, docs/impl/shell.md >
/// Behaviour).
pub fn toggle_panel<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(PANEL_LABEL) {
        let _ = w.close();
    } else {
        open_panel(app);
    }
}

/// Opens the panel, creating it if needed: no decorations, always on top, not in the
/// taskbar, not resizable, width 320, placed at the tray icon, closes on blur
/// (docs/impl/ipc.md > Windows). It is shown once its page has rendered.
pub fn open_panel<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(PANEL_LABEL) {
        show_panel(&w);
        return;
    }
    let Ok(window) = WebviewWindowBuilder::new(
        app,
        PANEL_LABEL,
        WebviewUrl::App("index.html#/panel".into()),
    )
    .title(i18n::title_panel())
    .inner_size(PANEL_WIDTH, PANEL_DEFAULT_HEIGHT)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .build() else {
        return;
    };

    // Built hidden so the positioner can place it before it is ever shown (avoids a
    // flash at the window's default spawn position). The tray position is known only
    // after a tray event; without one (opened by relaunching the app) it is centered.
    if window
        .move_window_constrained(Position::TrayCenter)
        .is_err()
    {
        let _ = window.center();
    }
    // Shown by the first fit_panel; this covers a page that never reports.
    let fallback = window.clone();
    thread::spawn(move || {
        thread::sleep(PANEL_SHOW_FALLBACK);
        if matches!(fallback.is_visible(), Ok(false)) {
            show_panel(&fallback);
        }
    });

    let closing = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            if in_blur_grace() {
                let _ = closing.set_focus();
            } else {
                let _ = closing.close();
            }
        }
    });
}

/// Closes the panel, then opens the region window (docs/impl/ipc.md > begin_region). The
/// caller stores the snapshot itself; this only manages windows.
pub fn after_begin_region<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(PANEL_LABEL) {
        let _ = w.close();
    }
    open_region(app);
}

fn open_region<R: Runtime>(app: &AppHandle<R>) {
    if app.get_webview_window(REGION_LABEL).is_some() {
        return;
    }
    let Ok(window) = WebviewWindowBuilder::new(
        app,
        REGION_LABEL,
        WebviewUrl::App("index.html#/region".into()),
    )
    .title(i18n::title_region(effective_lang(app)))
    .inner_size(1100.0, 760.0)
    .resizable(true)
    .center()
    .build() else {
        return;
    };

    // Closed by its title bar button rather than Cancel: drop the snapshot too.
    let app = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            app.state::<SharedShell>().lock().unwrap().pending_region = None;
        }
    });
}

/// Closes the region window (docs/impl/ipc.md > cancel_region, start_session).
pub fn close_region<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(REGION_LABEL) {
        let _ = w.close();
    }
}

/// Opens or focuses the settings window (docs/impl/ipc.md > open_settings): 560 x 560, a
/// normal window with a title.
pub fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("index.html#/settings".into()),
    )
    .title(i18n::title_settings(effective_lang(app)))
    .inner_size(560.0, 560.0)
    .center()
    .build();
}

/// Resizes the panel to fit its content (docs/impl/ipc.md > fit_panel), clamped to
/// 160-640 CSS px tall; width stays fixed at 320. Ignored for any other window.
pub fn fit_panel<R: Runtime>(window: &tauri::WebviewWindow<R>, height: f64) {
    if window.label() != PANEL_LABEL {
        return;
    }
    let clamped = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
    let _ = window.set_size(LogicalSize::new(PANEL_WIDTH, clamped));
    // Keeps it at the tray icon; with no known tray position it stays where it is.
    let _ = window.move_window_constrained(Position::TrayCenter);
    // The first report means the page has rendered: show the panel now.
    if matches!(window.is_visible(), Ok(false)) {
        show_panel(window);
    }
}

/// Sets every native (outside-webview) window title to the effective language
/// (`apply_language`, docs/recipes/tauri-desktop-app.md gotcha 7).
pub fn retitle_all<R: Runtime>(app: &AppHandle<R>) {
    let lang = effective_lang(app);
    if let Some(w) = app.get_webview_window(REGION_LABEL) {
        let _ = w.set_title(i18n::title_region(lang));
    }
    if let Some(w) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = w.set_title(i18n::title_settings(lang));
    }
}
