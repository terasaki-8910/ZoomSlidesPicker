//! The tray icon and its menu (SPEC.md > 使い方の流れ, 画面と操作; docs/impl/shell.md >
//! Behaviour). [`apply`] runs on the main thread only (through `ui::sync`) and changes
//! only what differs from what is shown: the menu is rebuilt when its items change, not
//! every second while a session runs (which would also swap out a menu the user has open).

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::ipc_types::Lang;
use crate::state::Shell;
use crate::{i18n, settings, windows};

pub const TRAY_ID: &str = "main";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Visual {
    Stopped,
    Running,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MenuModel {
    lang: Lang,
    has_session: bool,
    has_last: bool,
    can_rebuild: bool,
}

/// What the tray shows, read from the shell state (under its lock) by `ui::sync`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    visual: Visual,
    /// Captures in the current session, while there is one.
    count: Option<usize>,
    menu: MenuModel,
}

impl Model {
    pub fn of(shell: &Shell) -> Self {
        let (visual, count) = match &shell.session {
            None => (Visual::Stopped, None),
            Some(active) => {
                let status = active.handle.status();
                match status.state {
                    zsp_session::State::Paused => (Visual::Paused, Some(status.count)),
                    _ => (Visual::Running, Some(status.count)),
                }
            }
        };
        Self {
            visual,
            count,
            menu: MenuModel {
                lang: settings::effective_language(shell.settings.language),
                has_session: shell.session.is_some(),
                has_last: shell.last_session.is_some(),
                can_rebuild: shell.last_session.as_ref().is_some_and(|l| l.count > 0),
            },
        }
    }
}

/// The model last applied to the tray (touched on the main thread only).
#[derive(Default)]
struct Shown(Mutex<Option<Model>>);

// macOS: black template images, recoloured by the menu bar for light and dark.
#[cfg(target_os = "macos")]
fn icon_bytes(v: Visual) -> &'static [u8] {
    match v {
        Visual::Stopped => include_bytes!("../icons/tray-stopped-64.png"),
        Visual::Running => include_bytes!("../icons/tray-running-64.png"),
        Visual::Paused => include_bytes!("../icons/tray-paused-64.png"),
    }
}

// Elsewhere there are no template images and the taskbar may be light or dark, so the
// glyph sits on an accent-coloured badge (scripts/make-icons.py).
#[cfg(not(target_os = "macos"))]
fn icon_bytes(v: Visual) -> &'static [u8] {
    match v {
        Visual::Stopped => include_bytes!("../icons/tray-stopped-badge-32.png"),
        Visual::Running => include_bytes!("../icons/tray-running-badge-32.png"),
        Visual::Paused => include_bytes!("../icons/tray-paused-badge-32.png"),
    }
}

/// Builds the tray icon once, at startup. Left click toggles the panel; right click
/// shows the menu (`show_menu_on_left_click(false)` keeps it off the left click). The
/// menu, title, and tooltip are filled in by the first `ui::sync`.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.manage(Shown::default());
    let icon = tauri::image::Image::from_bytes(icon_bytes(Visual::Stopped))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(i18n::tooltip_idle())
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_panel(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "library" => crate::library::open_window(app),
        "start" => windows::open_panel(app),
        "stop" => crate::session_ctl::stop(app),
        "open_folder" => crate::session_ctl::open_folder(app),
        "rebuild_pdf" => crate::session_ctl::rebuild_pdf_from_menu(app.clone()),
        "extract_video" => crate::video::extract_video_from_menu(app.clone()),
        "settings" => windows::open_settings(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Brings the tray in line with `model`, touching only what changed since the last call.
/// Main thread only (`ui::sync`).
pub fn apply<R: Runtime>(app: &AppHandle<R>, model: Model) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let shown = app.state::<Shown>();
    let mut shown = shown.0.lock().unwrap();
    let prev = *shown;

    if prev.map(|p| p.visual) != Some(model.visual) {
        set_icon(&tray, model.visual);
    }

    // macOS shows the count as the tray title; Windows/Linux have no tray title, so the
    // count only appears in the tooltip there (SPEC.md > 使い方の流れ).
    #[cfg(target_os = "macos")]
    if prev.map(|p| p.count) != Some(model.count) {
        let _ = tray.set_title(model.count.map(|c| c.to_string()));
    }

    let tooltip_key = |m: &Model| (m.visual, m.count, m.menu.lang);
    if prev.as_ref().map(tooltip_key) != Some(tooltip_key(&model)) {
        let tooltip = match (model.visual, model.count) {
            (Visual::Running, Some(c)) => i18n::tooltip_running(model.menu.lang, c),
            (Visual::Paused, Some(c)) => i18n::tooltip_paused(model.menu.lang, c),
            _ => i18n::tooltip_idle().to_string(),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }

    if prev.map(|p| p.menu) != Some(model.menu)
        && let Ok(menu) = build_menu(app, model.menu)
    {
        let _ = tray.set_menu(Some(menu));
    }

    *shown = Some(model);
}

fn set_icon<R: Runtime>(tray: &TrayIcon<R>, visual: Visual) {
    let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes(visual)) else {
        return;
    };
    // Plain `set_icon` would drop the template flag on macOS.
    #[cfg(target_os = "macos")]
    let _ = tray.set_icon_with_as_template(Some(icon), true);
    #[cfg(not(target_os = "macos"))]
    let _ = tray.set_icon(Some(icon));
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, m: MenuModel) -> tauri::Result<Menu<R>> {
    let item = |id: &str, text: &str, enabled: bool| {
        MenuItem::with_id(app, id, text, enabled, None::<&str>)
    };
    let library = item("library", i18n::tray_history(m.lang), true)?;
    let start = item("start", i18n::tray_start(m.lang), !m.has_session)?;
    let stop = item("stop", i18n::tray_stop(m.lang), m.has_session)?;
    let open_folder = item(
        "open_folder",
        i18n::tray_open_folder(m.lang),
        m.has_session || m.has_last,
    )?;
    let rebuild_pdf = item("rebuild_pdf", i18n::tray_rebuild_pdf(m.lang), m.can_rebuild)?;
    let extract_video = item(
        "extract_video",
        i18n::tray_extract_video(m.lang),
        !m.has_session,
    )?;
    let settings_item = item("settings", i18n::tray_settings(m.lang), true)?;
    let quit = item("quit", i18n::tray_quit(m.lang), true)?;

    Menu::with_items(
        app,
        &[
            &library,
            &start,
            &stop,
            &open_folder,
            &rebuild_pdf,
            &extract_video,
            &PredefinedMenuItem::separator(app)?,
            &settings_item,
            &quit,
        ],
    )
}
