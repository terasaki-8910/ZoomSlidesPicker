//! `list_windows`, `begin_region`, `get_region_snapshot`, `cancel_region`
//! (docs/impl/ipc.md).

use tauri::{AppHandle, Manager, Runtime};

use crate::ipc_types::{self, Snapshot, WindowEntry};
use crate::state::SharedShell;
use crate::{i18n, settings, thumb, windows};

const WINDOW_THUMB_MAX_WIDTH: u32 = 240;
const SNAPSHOT_MAX_WIDTH: u32 = 1600;

fn current_lang<R: Runtime>(app: &AppHandle<R>) -> ipc_types::Lang {
    let shell = app.state::<SharedShell>();
    let language = shell.lock().unwrap().settings.language;
    settings::effective_language(language)
}

/// Capturable windows, with thumbnails (docs/impl/ipc.md > list_windows). A window that
/// fails to capture between listing and thumbnailing (closed, became unreadable) is
/// silently left out rather than failing the whole list.
pub fn list_windows<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<WindowEntry>, String> {
    let lang = current_lang(app);
    let infos = zsp_capture::list_windows()
        .map_err(|e| i18n::error_capture_failed(lang, &e.to_string()))?;

    Ok(infos
        .into_iter()
        .filter_map(|info| {
            let capture = zsp_capture::WindowCapture::open(info.id).ok()?;
            let img = capture.capture().ok()?;
            Some(WindowEntry {
                id: info.id,
                app: info.app,
                title: info.title,
                width: info.width,
                height: info.height,
                thumb: thumb::thumbnail_data_url(&img, WINDOW_THUMB_MAX_WIDTH),
            })
        })
        .collect())
}

/// Captures `window_id`'s snapshot, stores it, closes the panel, and opens the region
/// window (docs/impl/ipc.md > begin_region).
pub fn begin_region<R: Runtime>(app: &AppHandle<R>, window_id: u32) -> Result<(), String> {
    let lang = current_lang(app);
    let capture = zsp_capture::WindowCapture::open(window_id)
        .map_err(|e| i18n::error_capture_failed(lang, &e.to_string()))?;
    let img = capture
        .capture()
        .map_err(|e| i18n::error_capture_failed(lang, &e.to_string()))?;
    let info = capture.info();
    let (width, height) = image::GenericImageView::dimensions(&img);

    let snapshot = Snapshot {
        window_id,
        app: info.app.clone(),
        title: info.title.clone(),
        width,
        height,
        image: thumb::full_data_url(&img, SNAPSHOT_MAX_WIDTH),
    };

    app.state::<SharedShell>().lock().unwrap().pending_region = Some(snapshot);
    windows::after_begin_region(app);
    Ok(())
}

/// The snapshot `begin_region` captured (docs/impl/ipc.md > get_region_snapshot).
pub fn get_region_snapshot<R: Runtime>(app: &AppHandle<R>) -> Result<Snapshot, String> {
    let lang = current_lang(app);
    app.state::<SharedShell>()
        .lock()
        .unwrap()
        .pending_region
        .clone()
        .ok_or_else(|| i18n::error_capture_failed(lang, "no pending snapshot"))
}

/// Closes the region window (docs/impl/ipc.md > cancel_region).
pub fn cancel_region<R: Runtime>(app: &AppHandle<R>) {
    app.state::<SharedShell>().lock().unwrap().pending_region = None;
    windows::close_region(app);
}
