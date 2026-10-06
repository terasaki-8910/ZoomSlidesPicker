//! The tray icon and its menu (SPEC.md > 使い方の流れ, 画面と操作; docs/impl/shell.md >
//! Behaviour). Rebuilt in full on every state change and by `apply_language` -- simpler
//! and less error-prone than patching individual menu items in place while both the
//! language and the enabled/disabled set can change independently.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::ipc_types::Lang;
use crate::state::Shell;
use crate::{i18n, settings, windows};

pub const TRAY_ID: &str = "main";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visual {
    Stopped,
    Running,
    Paused,
}

/// The tray's visual state and, while a session runs, its capture count.
fn visual_for(shell: &Shell) -> (Visual, Option<usize>) {
    match &shell.session {
        None => (Visual::Stopped, None),
        Some(active) => {
            let status = active.handle.status();
            match status.state {
                zsp_session::State::Paused => (Visual::Paused, Some(status.count)),
                _ => (Visual::Running, Some(status.count)),
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn icon_bytes(v: Visual) -> &'static [u8] {
    match v {
        Visual::Stopped => include_bytes!("../icons/tray-stopped-64.png"),
        Visual::Running => include_bytes!("../icons/tray-running-64.png"),
        Visual::Paused => include_bytes!("../icons/tray-paused-64.png"),
    }
}

#[cfg(not(target_os = "macos"))]
fn icon_bytes(v: Visual) -> &'static [u8] {
    match v {
        Visual::Stopped => include_bytes!("../icons/tray-stopped-32.png"),
        Visual::Running => include_bytes!("../icons/tray-running-32.png"),
        Visual::Paused => include_bytes!("../icons/tray-paused-32.png"),
    }
}

/// Builds the tray icon once, at startup. Left click toggles the panel; right click
/// shows the menu (`show_menu_on_left_click(false)` keeps it off the left click). Its
/// menu and visuals are filled in by the first [`refresh`] call right after.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let icon = tauri::image::Image::from_bytes(icon_bytes(Visual::Stopped))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
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
        "start" => windows::open_panel(app),
        "stop" => crate::session_ctl::stop(app),
        "open_folder" => crate::session_ctl::open_folder(app),
        "rebuild_pdf" => crate::session_ctl::rebuild_pdf_from_menu(app),
        "extract_video" => crate::video::extract_video_from_menu(app.clone()),
        "settings" => windows::open_settings(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Rebuilds the menu and updates the icon/title/tooltip from the current state and
/// effective language. Called after every state change and by `apply_language`
/// (docs/impl/shell.md > Behaviour; docs/recipes/tauri-desktop-app.md gotcha 7).
pub fn refresh<R: Runtime>(app: &AppHandle<R>, shell: &Shell) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let lang = settings::effective_language(shell.settings.language);
    let (visual, count) = visual_for(shell);

    if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes(visual)) {
        let _ = tray.set_icon(Some(icon));
    }

    // macOS shows the count as the tray title; Windows/Linux have no tray title, so the
    // count only appears in the tooltip there (SPEC.md > 使い方の流れ).
    #[cfg(target_os = "macos")]
    {
        let _ = tray.set_title(count.map(|c| c.to_string()));
    }

    let tooltip = match (visual, count) {
        (Visual::Running, Some(c)) => i18n::tooltip_running(lang, c),
        (Visual::Paused, Some(c)) => i18n::tooltip_paused(lang, c),
        _ => i18n::tooltip_idle().to_string(),
    };
    let _ = tray.set_tooltip(Some(tooltip));

    if let Ok(menu) = build_menu(app, shell, lang) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, shell: &Shell, lang: Lang) -> tauri::Result<Menu<R>> {
    let has_session = shell.session.is_some();
    let has_last = shell.last_session.is_some();
    let can_rebuild = shell.last_session.as_ref().is_some_and(|l| l.count > 0);

    let start = MenuItem::with_id(
        app,
        "start",
        i18n::tray_start(lang),
        !has_session,
        None::<&str>,
    )?;
    let stop = MenuItem::with_id(
        app,
        "stop",
        i18n::tray_stop(lang),
        has_session,
        None::<&str>,
    )?;
    let open_folder = MenuItem::with_id(
        app,
        "open_folder",
        i18n::tray_open_folder(lang),
        has_session || has_last,
        None::<&str>,
    )?;
    let rebuild_pdf = MenuItem::with_id(
        app,
        "rebuild_pdf",
        i18n::tray_rebuild_pdf(lang),
        can_rebuild,
        None::<&str>,
    )?;
    let extract_video = MenuItem::with_id(
        app,
        "extract_video",
        i18n::tray_extract_video(lang),
        !has_session,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(
        app,
        "settings",
        i18n::tray_settings(lang),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", i18n::tray_quit(lang), true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
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
