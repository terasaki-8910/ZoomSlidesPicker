//! Starting a window-sourced session (docs/impl/ipc.md > start_session;
//! docs/impl/shell.md > Behaviour). Shared by the real command and the
//! `ZSP_AUTOSTART_*` test hook (`autostart.rs`), which starts one on the full window.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};
use zsp_capture::{Region, WindowCapture};
use zsp_output::{
    OsTrash, RegionRecord, SessionMeta, SessionWriter, Source, SourceKind as OutKind,
};
use zsp_session::{Session, WindowSource};

use crate::ipc_types::{RegionSel, SourceKind};
use crate::state::{self, ActiveSession, SharedShell};
use crate::{i18n, notify, settings, ui, windows};

/// Starts a window-sourced session (docs/impl/ipc.md > start_session): validates the
/// region, requires no session already running and (on macOS) Screen Recording
/// permission, spawns the session, then closes the region window and notifies. On an
/// error the region window stays open so it can show the message.
pub fn start<R: Runtime>(
    app: &AppHandle<R>,
    window_id: u32,
    region_sel: RegionSel,
) -> Result<(), String> {
    let region = Region {
        x: region_sel.x,
        y: region_sel.y,
        w: region_sel.w,
        h: region_sel.h,
    };
    region.validate()?;

    let shell_state = app.state::<SharedShell>();
    // Held from the "no session yet" check until the new session is stored, so two starts
    // cannot both pass the check. Nothing below waits for the main thread.
    let mut shell = shell_state.lock().unwrap();
    let lang = settings::effective_language(shell.settings.language);

    if shell.session.is_some() {
        return Err(i18n::error_session_already_running(lang).to_string());
    }
    if !zsp_capture::permission::screen_capture_granted() {
        drop(shell);
        notify::notify_permission_missing(app, lang);
        return Err(i18n::permission_notice(lang).to_string());
    }

    let capture = WindowCapture::open(window_id)
        .map_err(|e| i18n::error_capture_failed(lang, &e.to_string()))?;
    let info = capture.info().clone();

    let root = PathBuf::from(&shell.settings.save_root);
    let interval_s = shell.settings.interval_s;
    let meta = SessionMeta {
        app_version: shell.app_version.clone(),
        source: Source {
            kind: OutKind::Window,
            app: Some(info.app.clone()),
            title: Some(info.title.clone()),
            platform: state::platform_str().to_string(),
            file: None,
        },
        region: Some(RegionRecord {
            x: region.x,
            y: region.y,
            w: region.w,
            h: region.h,
        }),
        interval_s,
    };

    let writer = SessionWriter::create(&root, chrono::Local::now(), meta)
        .map_err(|e| i18n::error_save_folder(lang, &e.to_string()))?;
    let dir = writer.dir().to_path_buf();
    let session = Session::new(
        writer,
        zsp_detect::Config {
            interval_s,
            ..Default::default()
        },
        Box::new(OsTrash),
    );
    let source = WindowSource::new(capture, region);
    let handle = zsp_session::spawn(
        Box::new(source),
        session,
        interval_s,
        notify::make_notify(app.clone()),
    );

    shell.session = Some(ActiveSession {
        handle,
        dir,
        source_kind: SourceKind::Window,
        source_title: info.title,
        interval_s,
        thumb: None,
        video_total_frames: None,
    });
    shell.pending_region = None; // the snapshot (a few MB) is no longer needed
    drop(shell);

    windows::close_region(app);
    ui::sync(app);
    notify::notify_started(app, lang);
    Ok(())
}
